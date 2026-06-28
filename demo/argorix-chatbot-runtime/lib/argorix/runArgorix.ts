/**
 * runArgorix.ts
 * -------------
 * Thin, security-conscious wrapper around the locally-built Argorix binaries
 * (`argorixc` and `argorix-vm`).
 *
 * Design rules enforced here:
 *  - We NEVER run a shell. Every call uses `execFileSync` with an explicit
 *    argument array, so user input can never be interpreted as a command.
 *  - We NEVER pass secrets to the binaries. Argorix only ever sees reference
 *    names like `env:OPENAI_API_KEY`, which already live inside the contract.
 *  - All captured stdout/stderr is passed through `redact()` before it can be
 *    returned to the caller / surfaced in the UI.
 *
 * If the exact CLI surface changes, this is the ONLY file that should need
 * adjusting — the rest of the demo depends on these typed helpers.
 */

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { redact } from "./parseArgorixOutput";

function resolveBin(envVar: string, fallback: string): string {
  const raw = process.env[envVar]?.trim() || fallback;
  return path.resolve(process.cwd(), raw);
}

export function argorixcBin(): string {
  return resolveBin("ARGORIXC_BIN", "../../target/debug/argorixc.exe");
}

export function argorixVmBin(): string {
  return resolveBin("ARGORIX_VM_BIN", "../../target/debug/argorix-vm.exe");
}

export interface CliResult {
  ok: boolean;
  exitCode: number;
  stdout: string;
  stderr: string;
}

/** Run a binary with explicit args. Output is always redacted. */
function run(bin: string, args: string[]): CliResult {
  try {
    const stdout = execFileSync(bin, args, {
      encoding: "utf8",
      // Hard caps so a runaway process cannot hang or flood the demo.
      timeout: 20_000,
      maxBuffer: 16 * 1024 * 1024,
      // We deliberately do NOT forward the demo's full environment with secrets.
      // Argorix needs none of it; it works purely on the contract file.
      env: { PATH: process.env.PATH } as unknown as NodeJS.ProcessEnv,
    });
    return { ok: true, exitCode: 0, stdout: redact(stdout), stderr: "" };
  } catch (err: unknown) {
    const e = err as {
      status?: number;
      stdout?: Buffer | string;
      stderr?: Buffer | string;
      message?: string;
    };
    return {
      ok: false,
      exitCode: typeof e.status === "number" ? e.status : 1,
      stdout: redact(e.stdout?.toString() ?? ""),
      stderr: redact(e.stderr?.toString() ?? e.message ?? "unknown error"),
    };
  }
}

/** `argorixc check <contract>` — syntax + semantic validation. */
export function check(contractPath: string): CliResult {
  return run(argorixcBin(), ["check", contractPath]);
}

/** `argorixc emit-bytecode <contract>` — writes bytecode JSON to `outPath`. */
export function emitBytecode(contractPath: string, outPath: string): CliResult {
  const res = run(argorixcBin(), ["emit-bytecode", contractPath]);
  if (res.ok) {
    // emit-bytecode prints JSON to stdout; persist it as an auditable artifact.
    fs.writeFileSync(outPath, res.stdout);
  }
  return res;
}

/** `argorixc verify-bytecode <contract>` — compile + verify the bytecode. */
export function verifyBytecode(contractPath: string): CliResult {
  return run(argorixcBin(), ["verify-bytecode", contractPath]);
}

export interface ReactiveArtifacts {
  inject: string; // "From:To:act:MessageType"
  securityReport: string;
  traceOut: string;
  evidenceBundle: string;
}

/**
 * `argorix-vm run <bytecode> --dry-run --reactive --inject ... --json`
 * Produces the security report, trace and evidence bundle artifacts and returns
 * the parsed JSON summary printed on stdout.
 */
export function runReactiveEvidence(
  bytecodePath: string,
  a: ReactiveArtifacts,
): CliResult {
  return run(argorixVmBin(), [
    "run",
    bytecodePath,
    "--dry-run",
    "--reactive",
    "--inject",
    a.inject,
    "--json",
    "--security-report",
    a.securityReport,
    "--trace-out",
    a.traceOut,
    "--evidence-bundle",
    a.evidenceBundle,
  ]);
}

export interface RuntimeRunOpts {
  runtime: string; // runtime_execution_profile name
  adapter: string; // sandboxed_provider_adapter name
  operation: string; // e.g. "responses.create"
  sandboxedExternal: boolean;
}

/**
 * `argorix-vm run <bytecode> --dry-run --json --runtime ... --adapter ... --operation ...`
 * Returns the governed runtime decision. Without `--sandboxed-external` the
 * core returns `blocked`; with it, the core returns `planned` (and STILL makes
 * no network call — planning is the most the core will ever do).
 */
export function runRuntimeProfile(
  bytecodePath: string,
  o: RuntimeRunOpts,
): CliResult {
  const args = [
    "run",
    bytecodePath,
    "--dry-run",
    "--json",
    "--runtime",
    o.runtime,
    "--adapter",
    o.adapter,
    "--operation",
    o.operation,
  ];
  if (o.sandboxedExternal) args.push("--sandboxed-external");
  return run(argorixVmBin(), args);
}

/** `argorix-vm verify-evidence <bundle> --json` — integrity check of the bundle. */
export function verifyEvidence(bundlePath: string): CliResult {
  return run(argorixVmBin(), ["verify-evidence", bundlePath, "--json"]);
}
