"""Small same-directory atomic publication helpers."""

from __future__ import annotations

import os
import tempfile
from pathlib import Path
from typing import Callable


def atomic_publish(target: Path, writer: Callable[[Path], None]) -> None:
    target = Path(target)
    target.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(
        prefix=f".{target.name}.", suffix=".tmp", dir=target.parent
    )
    os.close(fd)
    temporary = Path(name)
    try:
        writer(temporary)
        os.replace(temporary, target)
    finally:
        temporary.unlink(missing_ok=True)


def atomic_write_text(target: Path, text: str) -> None:
    atomic_publish(
        target,
        lambda temporary: temporary.write_text(
            text, encoding="utf-8", newline="\n"
        ),
    )
