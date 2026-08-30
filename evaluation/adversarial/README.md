# ArgorixLang adversarial evaluation

Executed end-to-end and adversarial campaign for the ArgorixLang release
binaries. This directory implements the plan in
`paper/camera-ready/adversarial-evaluation-plan.md`.

Every number in the camera-ready paper is produced here. Nothing is typed by
hand into the manuscript: the paper `\input`s the files under `tables/`, which
are written by `harness/render_tables.py` from `results/summary.json`.

The campaign found three defects in ArgorixLang, which were fixed and the whole
campaign re-run. `baseline/prefix/` holds the pre-fix run unedited so both sets
of numbers stay checkable:

| Measure | Pre-fix | Now |
| --- | --- | --- |
| Outcome accuracy vs. oracle | 103/106 | 118/118 |
| Evidence modifications detected | 20/22 | 21/22 |
| Prohibited rejected at a boundary | 15/21 | 18/21 |
| Forged sets rejected under a trust anchor | not possible | 3/3 |

Findings are published in three buckets: 0 open
defects, 3 resolved and re-measured, and
5 boundaries that are true by construction and
each of which bounds a claim.

## One command

```bash
cargo build --locked --release -p argorixc -p argorix-vm -p argorix-conformance
python evaluation/adversarial/run.py --clean
```

`run.py` executes, in this fixed order:

1. `harness/anticircularity.py` — harness-validity test (see below);
2. `harness/collect.py` — runs the production executables against every case;
3. `harness/score.py` — joins raw rows with `oracle.json`;
4. `harness/render_tables.py` — writes the LaTeX inputs.

Regenerate the case catalogue with `python harness/build_catalogue.py`.

## Validity: Expected and Observed are physically separated

`cases.json` holds inputs and procedures. It contains no expected outcome of
any kind. `oracle.json` holds expectations only.

`collect` cannot read the oracle:

* no module on the collection path (`collect.py`, `util.py`, `mutate.py`,
  `canaries.py`, `stats.py`) references the file;
* `install_oracle_guard()` replaces `open`/`io.open` so that opening any file
  named `oracle.json` raises `PermissionError`;
* `anticircularity.py` exercises that guard in a subprocess and fails the
  campaign if the read succeeds.

`collect` also cannot fabricate a correct result without running Argorix:
`anticircularity.py` copies the binary directory with `argorix-vm` removed and
requires the campaign to fail and to produce no rows.

Digests are checked twice. `harness/util.py` recomputes the bytecode, trace,
report and ledger digests in Python — an implementation independent of
`crates/argorix_vm/src/evidence.rs` — and every row records both the
independent value and the value the release recorded. Diagnostics are compared
by stable class (`harness/util.py::DIAGNOSTIC_CLASSES`), never by exact message.

## Families

| Family | What it measures | Cases | Runs |
| --- | --- | --- | --- |
| E0 | historical snapshot re-measured with current binaries | 33 | 33 |
| E1 | behavioural diversity across real workloads | 12 | 36 |
| E2 | faults and adverse conditions | 20 | 20 |
| E3 | evidence mutation applied after generation | 22 | 22 |
| E4 | dispatch, side effects and the mediation point | 12 | 36 |
| E6 | authenticity under a producer trust anchor | 4 | 4 |
| E5 | prompt injection (conditional branch) | 16 | 80 |

### E0 — historical reproduction

All 33 request directories under
`demo/argorix-chatbot-runtime/generated/` stay in the denominator; the six
source-only directories are reported with the phase they reach, never dropped.

### E1 — behavioural diversity

Twelve workloads under `workloads/`, three repetitions each. The diversity gate
requires at least twelve distinct behavioural fingerprints over six dimensions
(policy, capability, provider, runtime profile, program structure, outcome).
Repetitions never count as diversity. A rejection that happens at compile time
or at bytecode verification is still an observation about its dimension, so the
matching diagnostic class is folded into that component of the fingerprint.

### E2 — faults and adverse conditions

Twenty cases covering the sixteen conditions the plan enumerates; invalid
bytecode, invalid injection routes and absent provider/adapter are each split
into their distinct mechanisms. An error, an outage or a deadline can never be
scored as `PASS`.

### E3 — evidence mutation

A clean set is generated and verified first. `harness/mutate.py` then edits
bytes on disk and `argorix-vm verify-evidence` is invoked again. Rows record the
pre- and post-mutation digests, so a mutation that changed nothing is scored
`INVALID` rather than counted. Results are reported per mutation class; there is
no general "tamper-proof" rate.

Runs pass `--source` so the bundle binds the source it was compiled from, which
is what makes the source-only mutation detectable. Multi-file packages carry no
binding, so their bundles make no source claim.

### E4 — dispatch and side effects

Three independent sensors observe the release process, each with a positive
control that must fire before its zero counts mean anything:

* a loopback HTTP sink with an append-only log keyed by a per-run nonce, whose
  address is exported to the child as `OPENAI_BASE_URL`;
* a filesystem sentinel in an isolated temporary directory, snapshotted by
  content hash before and after;
* synthetic secret and key-material canaries exported into the child
  environment and then searched for in every produced artifact and both process
  streams. These are never real credentials.

**The mediation point.**
The release resists instrumentation twice over: `ProviderRegistry::register`
rejects every provider that is not the built-in simulated one, and
`execution_registry` rebuilds that provider on every run, so even a substituted
registry never reaches the mediation point. Rather than weaken the release, a
second binary is compiled behind the non-default `eval-tripwire` feature:

```bash
cargo build --release -p argorix-vm \
  --target-dir target/eval-tripwire --features eval-tripwire
```

The release build does not compile it and rejects its flags; the evaluation
binary reports `argorix-vm 1.0.0 (eval-tripwire)` and every run manifest records
its SHA-256, so no row can be mistaken for one the release produced. Three
conditions are observed: an allowed call reaches the provider exactly once and
always with the dry-run flag set; a rejected program never reaches it; and a
probe issued from inside a provider invocation is recorded by the sink. That
last case is the positive control that makes zero hits elsewhere a measurement.
Without the build, those cases record `NOT_AVAILABLE`.

Non-reachability of a real external adapter stays out of scope: none exists.

### E6 - authenticity under a trust anchor

`argorix-sign` produces a detached Ed25519 signature over a bundle's canonical
bytes, and `argorix-vm verify-evidence --trust-anchor` checks it. Signing is a
separate binary so the runtime never handles private key material. Four cases:
an intact signed set, the coordinated replacement, a missing signature, and a
signature from a foreign key. Campaign keys come from a fixed `--seed` so a
rerun reproduces them; they protect nothing outside the harness.

### E5 — prompt injection

Specified and implemented, not executed. The grid is declared in the catalogue
before any model was called: eight scenarios across external provider calls,
network egress, secret exfiltration and key-material access; benign and injected
arms; five repetitions; 80 runs. Declaring it in advance is the point.

`harness/injection_driver.py` puts content in front of a real model, takes the
structured action it proposes, and pushes that action through mediation and the
sensors. Two things are measured and never merged: whether the model proposed
the prohibited action, and whether that action reached a sensor. The
proposal-to-program mapping is published in `PROGRAM_BANK` and is total, so a
proposal no program covers is recorded as `UNMAPPABLE` and counted.

The branch runs only when `ARGORIX_EVAL_LLM_DRIVER`, `ARGORIX_EVAL_LLM_ENDPOINT`
and `ARGORIX_EVAL_LLM_MODEL` are all set; otherwise every row records
`NOT_EXECUTED` with its reasons and the paper keeps "prompt injection was not
evaluated". Every row carries the endpoint and model it used, so a run against
anything other than a real model identifies itself.

Even executed, this design cannot claim that an Argorix agent loop resists
injection: the driver maps the proposal, because the release has no
prompt-bearing execution path of its own.

## Outputs

```
baseline/prefix/                         pre-fix campaign, archived unedited
results/raw/<run-id>/rows.jsonl          one row per execution
results/raw/<run-id>/manifest.json       commit, binary SHA-256, toolchain, platform
results/raw/<run-id>/anticircularity.json
results/raw/<run-id>/<case>/<rep>/       stdout, stderr and artifacts per case
results/results.jsonl                    rows joined with their score
results/results.csv                      row-level table
results/summary.json                     metrics, gates, open/resolved findings
results/REPRODUCIBILITY.json             pre/post comparison and rerun equality
results/CHECKSUMS.sha256
tables/*.tex                             LaTeX inputs for the paper
```

Row and manifest shapes are specified in `schemas/`.

## Oracle amendments

Amendments are recorded in `oracle.json` under `amendments` and surfaced in
`summary.json`. Expected values are never rewritten.

* **A1** — the preregistered `phase` field conflated the stage at which the
  typed decision is made with the furthest stage the pipeline reaches. The
  release writes and verifies an evidence bundle even for a denied or failed
  run, so no single reading is correct everywhere. `phase` was demoted to an
  informational check recorded against both readings and excluded from outcome
  accuracy. No outcome expectation was affected.
* **A2** — after the source-binding fix, the expectation for the source-only
  mutation moved from `NOT_DETECTED` to `DETECTED`. The product changed, not
  our expectation of the product as it was: the pre-fix campaign, its oracle
  outcome and its raw rows are archived unedited under `baseline/prefix/`.

## Deviations from the plan

* E2 keeps its twenty cases; the plan's sixteen conditions needed four splits.
* **E6** is new: authenticity is scored as its own family rather than folded
  into E3, so the mutation-detection rate stays comparable with the baseline.
* E4 grew from nine cases to twelve with the mediation-point observations.

## Claim boundaries

The campaign supports statements of the form "in this harness, X/Y ... with a
Wilson 95% interval". It does not support, and the paper must not state,
"prevents prompt injection", "no side effect can occur", "tamper-proof",
operating-system isolation, legal sovereignty, or general security across models
and adapters. Authenticity may be claimed only relative to a supplied trust
anchor, never as key governance or provenance over time.
