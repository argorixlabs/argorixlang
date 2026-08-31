"""Independent side-effect sensors for the E4 dispatch experiments.

Three local, credential-free sensors observe the *process* under test:

* ``LoopbackSink``   - an HTTP server bound to 127.0.0.1 with an append-only
  request log keyed by a per-run nonce.  Any egress the release binary performs
  toward the advertised endpoint is recorded here, not inferred from the VM's
  own report.
* ``FilesystemSentinel`` - an isolated temporary directory holding a sentinel
  file.  Content, mtime and the directory listing are captured before and after
  the run, so writes and new files are detected out of band.
* ``SecretCanary`` - a synthetic, non-credential token exported into the child
  environment.  After the run every produced artifact and both process streams
  are scanned for the token.

Every sensor exposes a positive control that must produce at least one hit; a
sensor with no demonstrated hit contributes no evidence.
"""

from __future__ import annotations

import http.server
import json
import os
import secrets
import socket
import threading
import urllib.error
import urllib.request
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


# --------------------------------------------------------------------------
# loopback network sink
# --------------------------------------------------------------------------


class _SinkHandler(http.server.BaseHTTPRequestHandler):
    server_version = "ArgorixEvalSink/1.0"

    def _record(self, method: str) -> None:
        length = int(self.headers.get("Content-Length") or 0)
        body = self.rfile.read(length).decode("utf-8", errors="replace") if length else ""
        self.server.hits.append(  # type: ignore[attr-defined]
            {
                "method": method,
                "path": self.path,
                "client": self.client_address[0],
                "user_agent": self.headers.get("User-Agent"),
                "body_bytes": len(body.encode("utf-8")),
                "body_preview": body[:512],
            }
        )
        # The hit is already recorded. A client that writes and exits without
        # reading -- which is exactly what an egress probe does -- makes the
        # response write fail; that must not look like a sensor failure.
        payload = json.dumps({"sink": "argorix-eval", "recorded": True}).encode("utf-8")
        try:
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)
        except OSError:
            self.close_connection = True

    def do_GET(self) -> None:  # noqa: N802 - http.server API
        self._record("GET")

    def do_POST(self) -> None:  # noqa: N802 - http.server API
        self._record("POST")

    def do_PUT(self) -> None:  # noqa: N802 - http.server API
        self._record("PUT")

    def log_message(self, *_args: Any) -> None:  # silence stderr logging
        return

    def handle_one_request(self) -> None:
        try:
            super().handle_one_request()
        except OSError:
            self.close_connection = True


class _SinkServer(http.server.ThreadingHTTPServer):
    daemon_threads = True

    def handle_error(self, request, client_address) -> None:
        # A probe that never reads the reply is normal here, not an error.
        return

    def __init__(self, address: tuple[str, int]) -> None:
        super().__init__(address, _SinkHandler)
        self.hits: list[dict[str, Any]] = []


@dataclass
class LoopbackSink:
    """Append-only loopback HTTP sink with a per-run nonce."""

    log_path: Path
    nonce: str
    _server: _SinkServer | None = field(default=None, init=False, repr=False)
    _thread: threading.Thread | None = field(default=None, init=False, repr=False)
    port: int = field(default=0, init=False)

    def start(self) -> None:
        self._server = _SinkServer(("127.0.0.1", 0))
        self.port = self._server.server_address[1]
        self._thread = threading.Thread(target=self._server.serve_forever, daemon=True)
        self._thread.start()

    @property
    def base_url(self) -> str:
        return f"http://127.0.0.1:{self.port}/{self.nonce}"

    def hits(self) -> list[dict[str, Any]]:
        return list(self._server.hits) if self._server else []

    def positive_control(self) -> bool:
        """Prove the sensor can observe an egress hit."""
        try:
            request = urllib.request.Request(
                f"{self.base_url}/positive-control",
                data=json.dumps({"nonce": self.nonce, "control": True}).encode("utf-8"),
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            with urllib.request.urlopen(request, timeout=5):  # noqa: S310 - loopback only
                pass
        except (urllib.error.URLError, OSError):
            return False
        return any("positive-control" in hit["path"] for hit in self.hits())

    def flush(self) -> None:
        self.log_path.parent.mkdir(parents=True, exist_ok=True)
        with self.log_path.open("a", encoding="utf-8") as handle:
            for hit in self.hits():
                handle.write(json.dumps({"nonce": self.nonce, **hit}) + "\n")

    def stop(self) -> None:
        self.flush()
        if self._server is not None:
            self._server.shutdown()
            self._server.server_close()
        if self._thread is not None:
            self._thread.join(timeout=5)


def closed_port() -> int:
    """Return a loopback port with no listener (outage / timeout condition)."""
    probe = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    probe.bind(("127.0.0.1", 0))
    port = probe.getsockname()[1]
    probe.close()
    return port


# --------------------------------------------------------------------------
# filesystem sentinel
# --------------------------------------------------------------------------


@dataclass
class FilesystemSentinel:
    root: Path
    nonce: str

    def start(self) -> None:
        self.root.mkdir(parents=True, exist_ok=True)
        self.sentinel.write_text(
            f"argorix-eval-sentinel {self.nonce}\n", encoding="utf-8"
        )
        self._before = self._snapshot()

    @property
    def sentinel(self) -> Path:
        return self.root / "sentinel.txt"

    def _snapshot(self) -> dict[str, Any]:
        entries = {}
        for path in sorted(self.root.rglob("*")):
            if path.is_file():
                stat = path.stat()
                entries[str(path.relative_to(self.root)).replace("\\", "/")] = {
                    "size": stat.st_size,
                    "mtime_ns": stat.st_mtime_ns,
                    "sha256": _sha256(path),
                }
        return entries

    def observe(self) -> dict[str, Any]:
        after = self._snapshot()
        before = getattr(self, "_before", {})
        created = sorted(set(after) - set(before))
        removed = sorted(set(before) - set(after))
        modified = sorted(
            name
            for name in set(after) & set(before)
            if after[name]["sha256"] != before[name]["sha256"]
        )
        return {
            "created": created,
            "removed": removed,
            "modified": modified,
            "hits": len(created) + len(removed) + len(modified),
        }

    def positive_control(self) -> bool:
        (self.root / f"positive-control-{self.nonce}.txt").write_text(
            "control", encoding="utf-8"
        )
        observation = self.observe()
        return observation["hits"] > 0

    def rebaseline(self) -> None:
        self._before = self._snapshot()


def _sha256(path: Path) -> str:
    import hashlib

    return hashlib.sha256(path.read_bytes()).hexdigest()


# --------------------------------------------------------------------------
# secret canary
# --------------------------------------------------------------------------


@dataclass
class SecretCanary:
    """A synthetic token; never a real credential."""

    nonce: str

    def __post_init__(self) -> None:
        self.token = f"ARGXCANARY-{self.nonce}-{secrets.token_hex(8)}"
        self.key_material = f"ARGXKEYCANARY-{self.nonce}-{secrets.token_hex(8)}"

    def env(self, base_url: str, sentinel_dir: Path) -> dict[str, str]:
        return {
            "ARGORIX_PROVIDER_TOKEN": self.token,
            "OPENAI_API_KEY": self.token,
            "OPENAI_BASE_URL": base_url,
            "ARGORIX_EVAL_KEY_MATERIAL": self.key_material,
            "ARGORIX_EVAL_SENTINEL_DIR": str(sentinel_dir),
        }

    def scan(self, texts: list[str], files: list[Path]) -> dict[str, Any]:
        needles = {"secret_token": self.token, "key_material": self.key_material}
        hits: list[dict[str, str]] = []
        for label, needle in needles.items():
            for index, text in enumerate(texts):
                if needle in text:
                    hits.append({"canary": label, "where": f"stream[{index}]"})
            for path in files:
                try:
                    content = path.read_text(encoding="utf-8", errors="replace")
                except OSError:
                    continue
                if needle in content:
                    hits.append({"canary": label, "where": str(path)})
        return {"hits": len(hits), "detail": hits}

    def positive_control(self, tmp_dir: Path) -> bool:
        probe = tmp_dir / f"secret-positive-control-{self.nonce}.txt"
        probe.parent.mkdir(parents=True, exist_ok=True)
        probe.write_text(f"leaked {self.token}\n", encoding="utf-8")
        return self.scan([], [probe])["hits"] > 0


# --------------------------------------------------------------------------
# combined harness
# --------------------------------------------------------------------------


@dataclass
class CanarySet:
    root: Path
    nonce: str

    def __enter__(self) -> "CanarySet":
        self.sink = LoopbackSink(log_path=self.root / "sink.log.jsonl", nonce=self.nonce)
        self.sink.start()
        self.filesystem = FilesystemSentinel(root=self.root / "sentinel", nonce=self.nonce)
        self.filesystem.start()
        self.secret = SecretCanary(nonce=self.nonce)
        return self

    def __exit__(self, *_exc: Any) -> None:
        self.sink.stop()

    def child_env(self) -> dict[str, str]:
        env = self.secret.env(self.sink.base_url, self.filesystem.root)
        env["ARGORIX_EVAL_NONCE"] = self.nonce
        return env

    def controls(self) -> dict[str, bool]:
        """Run every positive control, then re-baseline so the measurement of
        the process under test starts from a clean sensor state."""
        results = {
            "network_sink": self.sink.positive_control(),
            "filesystem_sentinel": self.filesystem.positive_control(),
            "secret_canary": self.secret.positive_control(self.root / "control"),
        }
        self.reset()
        return results

    def reset(self) -> None:
        self.sink.flush()
        if self.sink._server is not None:  # noqa: SLF001 - same module
            self.sink._server.hits.clear()  # noqa: SLF001
        self.filesystem.rebaseline()

    def observe(self, streams: list[str], artifacts: list[Path]) -> dict[str, Any]:
        network = self.sink.hits()
        filesystem = self.filesystem.observe()
        secret = self.secret.scan(streams, artifacts)
        return {
            "network_hits": len(network),
            "network_detail": network,
            "filesystem": filesystem,
            "secret": secret,
            "sink_reachable": True,
            "total_hits": len(network) + filesystem["hits"] + secret["hits"],
        }


def new_nonce() -> str:
    return secrets.token_hex(12)


def isolated_root(base: Path) -> Path:
    root = base / f"canary-{new_nonce()}"
    root.mkdir(parents=True, exist_ok=True)
    return root


__all__ = [
    "CanarySet",
    "FilesystemSentinel",
    "LoopbackSink",
    "SecretCanary",
    "closed_port",
    "isolated_root",
    "new_nonce",
]
