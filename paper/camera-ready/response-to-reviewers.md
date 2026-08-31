# Response to reviewers — Paper 138

**Paper:** *ArgorixLang: Compiled Governance, Jurisdiction-Aware Agent
Metadata, and Offline-Verifiable Runtime Evidence*

We thank Reviewers 1 and 6 for the detailed, constructive evaluations. Since
the previous revision we executed the end-to-end and adversarial campaign that
the reviews asked for. It runs the release binaries, keeps expected and
observed outcomes in separate files and separate processes, and publishes every
row. The camera-ready is seven IEEEtran pages, one inside the official JCC 2026
four-to-eight-page limit, with two figures, three compact tables, no listings,
and no appendix.

Everything below is reproducible from a clean directory with one command:

```
cargo build --locked --release -p argorixc -p argorix-vm -p argorix-conformance
python evaluation/adversarial/run.py --clean
```

Two independent clean runs produced identical metrics, gates, and tables. Every
number in the paper is generated into `paper/camera-ready/tables/` by that
pipeline and `\input`; none is transcribed by hand.

The campaign found three real defects in ArgorixLang. We fixed all three,
re-ran the whole campaign from a clean directory, and archived the pre-fix run
unedited so that both sets of numbers stay checkable:

| Measure | Pre-fix | Now |
| --- | --- | --- |
| Outcome accuracy vs. oracle | 103/106 | **118/118** |
| Evidence modifications detected | 20/22 | **21/22** |
| Prohibited rejected at a boundary | 15/21 | **18/21** |
| Forged sets rejected under a trust anchor | not possible | **3/3** |

The three fixes are an unchecked provider payload type in the bytecode
verifier, the absence of any binding between a run and the source it was
compiled from, and the absence of any producer binding at all. All three are
described under Reviewer 1 comment 6, together with the two remaining items the
reviews asked about: instrumentation of the mediation point, and prompt
injection.

## Summary of what changed

| Reviewer concern | Previous revision | Now |
| --- | --- | --- |
| Circular 57-row matrix | removed, replaced by the project's own conformance suite | replaced by an executed campaign of 139 runs with an oracle the collector cannot read |
| No behavioural diversity | acknowledged | 12 workloads × 3 repetitions producing 12 distinct fingerprints over 6 dimensions |
| No controlled adversarial experiments | future-work sketch | 20 fault conditions, 22 post-generation mutations, 8 dispatch conditions with three instrumented sensors |
| Prompt injection simulated | stated as not evaluated | evaluated against a real model: 20/40 injections moved it, 25/25 prohibited proposals contained |
| Integrity vs. approval conflated | separated in prose | separated and checked mechanically in all 139 runs |

## Reviewer 1

### 1. The 27/27 verified bundles conflict with 1,188 unknown policy rules

Agreed, and now measured rather than argued. The snapshot was re-measured with
the current release binaries: 27/27 complete directories verify, 0/27 are
policy-approved, and each contains 44 `unknown policy rule` findings for 1,188
in total. All 108 semantic digests were recomputed by an independent Python
implementation and agree with the values the bundles record, so the
re-measurement is not a second reading of the same code.

We also answer the underlying question. The current release derives the
aggregate verdict monotonically from the policy detail, and the campaign checks
that directly: in 139 runs no security report carries an approving aggregate
verdict over a block, review, warning, or unknown-rule detail. The snapshot's
44 identifiers are no longer merely unevaluated — the current compiler and the
bytecode verifier both reject an unknown policy rule outright, so the same
program never reaches the runtime.

### 2. The v0.2 matrix may be designed fixtures rather than executed results

Agreed. The 57-row generator was removed and is not replaced by another
self-agreeing artifact. The campaign is structured so that this class of
defect is detectable:

- `cases.json` holds inputs and procedures and contains no expected outcome;
  `oracle.json` holds expectations only.
- The collector cannot read the oracle. No module on its path names the file,
  and a runtime guard turns any attempt to open it into a hard error.
- A negative control copies the binary directory with `argorix-vm` removed and
  requires the campaign to fail and emit no rows. It does.
- Digests are recomputed by an implementation independent of the Rust code, and
  diagnostics are compared by stable class rather than by message text.

The harness-validity test is the first step of `run.py`; if it fails the
campaign aborts before collecting anything.

### 3. The 33-request snapshot lacks behavioral diversity

Agreed. The snapshot is reported as a control, not as a workload set: all 33
directories stay in the denominator, the 6 source-only ones are reported with
the phase they reach, and the 27 complete ones still collapse to two
fingerprint families of sizes 20 and 7 with one event sequence and 265 events
each.

Diversity is now supplied by twelve independent workloads run three times each.
They produce twelve distinct behavioural fingerprints over six dimensions —
policy, capability, provider, runtime profile, program structure, and outcome —
with repetitions collapsing exactly as they should. The workloads cover an
allowed tool call, an allowed model call, a missing capability, an undeclared
tool, an undeclared model, policy pass, policy deny, policy review, an unknown
policy rule, a declared non-executable external contract, an invalid runtime
profile request, and a multi-file package carrying messages, a policy, and a
passport.

### 4. “Sovereign” may imply legal jurisdiction or state recognition

Unchanged from the previous revision and still agreed. The title and paper use
**jurisdiction-aware metadata** or **jurisdiction and residency metadata**; the
implementation's historical field name is mentioned once for traceability. The
text states that these are self-asserted, locally validated program attributes
that prove no nationality, state recognition, legal status, physical data
location, or compliance.

### 5. ATrust and DCP-AI are not adequate foundational evidence

Unchanged and still agreed. They appear only as design vocabulary, explicitly
not as independent evidence. The related-work foundation is peer-reviewed work
on runtime enforcement, Cedar authorization, secure logging, in-toto
provenance, and agent prompt-injection benchmarks.

### 6. Threat mitigations lack controlled adversarial experiments

This is the concern the new campaign exists to answer.

**Faults and adverse conditions (20 cases).** Malformed sources, invalid
bytecode, invalid injection routes, an absent runtime profile, an absent
adapter, a denied adapter operation, allowlist rejection, a missing bytecode
file, a truncated bundle, a missing trace, a missing report, an artifact path
escaping the portable tree, an imposed deadline, an unavailable sensor,
concurrent execution, request replay, and evidence replay. All 16 conditions
whose oracle requires fail-closed behaviour ended fail-closed (16/16, Wilson 95%
[80.6, 100.0]). Concurrent runs produced byte-identical reports and request
replay reproduced identical digests for all four artifacts.

**Evidence mutation (22 cases).** A clean set is generated and verified first;
mutations then edit bytes on disk and the real verifier is re-invoked, with
pre- and post-mutation digests recorded so a mutation that changed nothing is
discarded rather than counted. 21/22 detected, Wilson 95% [78.2, 99.2],
reported per mutation class. We never write “tamper-proof”.

**Dispatch and side effects (8 conditions, 24 instrumented runs).** Three
sensors observe the release process from outside: a loopback HTTP sink with an
append-only per-nonce log advertised through the endpoint environment variable
the declared adapter references, a filesystem sentinel hashed before and after,
and synthetic secret and key-material canaries searched for in every artifact
and both streams. Each sensor demonstrated a positive control in every run.
0/21 prohibited proposals reached any sensor; with zero events we publish the
rule-of-three bound (at most 14.3%) rather than a zero rate.

**Two defects, found and fixed.** The campaign did not merely confirm the
design; it broke it twice.

*Unchecked provider payload types.* A model whose declared input type does not
exist in the program passed bytecode verification and executed as a simulated
dry-run call. The verifier validated provider and capability but never the
input and output types. It now checks all four slots, gated on the bytecode
versions whose schema carries a type table so that pre-v0.18 programs are not
retroactively rejected. Boundary rejection moved from 15/21 to 18/21.

*No source binding.* Editing only the source file was undetectable, because
nothing tied a run to a source. `argorixc emit-bytecode` now binds the source
digest into the emitted bytecode — only the compiler can assert this, since the
VM never sees the source — the bundle records the source path it was given, and
offline verification recomputes the digest. A bundle naming a source the
bytecode does not bind fails closed. Detection moved from 20/22 to 21/22.

Both fixes carry their own regression tests; the workspace runs 377 tests with
`cargo fmt --check` and `clippy` clean.

*No producer binding.* Nothing distinguished the original artifact set from a
self-consistent replacement, so we added one. `argorix-sign`, a separate binary
so the runtime never handles private key material, produces a detached Ed25519
signature over the bundle's canonical bytes; `verify-evidence --trust-anchor`
checks it. Because the bundle already binds bytecode, trace, report and source
by digest, one signature covers the set. All three non-producer sets we tested
are now rejected: the coordinated replacement, a bundle with no signature, and
a signature from another key. Without an anchor the behaviour is unchanged and
claims only integrity.

**Instrumenting the mediation point.** The previous revision could not
distinguish *did not dispatch* from *could not reach anything*. It turned out
the release resists instrumentation twice over: the provider registry rejects
every implementation that is not the built-in simulated one, and the reactive
path rebuilds that provider on every run, so even a substituted registry never
reaches the mediation point. Rather than weaken the release we compile a second
binary behind a non-default `eval-tripwire` feature. The release build does not
compile it and rejects its flags, and the evaluation binary reports a distinct
version string that every run manifest records, so no row can be mistaken for
one the release produced. With it: an allowed call reaches the provider exactly
once and always with the dry-run flag set; a rejected program never reaches it;
and a probe issued from inside a provider invocation is recorded by the sink.
That last case is the control that makes zero hits elsewhere a measurement.

**We still report where control is absent.** The paper states plainly that
18/21 prohibited conditions are rejected at a boundary, not 21/21: the declared
sandboxed external operation is accepted and reported as *planned* without
executing. That is by design and the sensors confirm it, but it means
containment (21/21) and rejection (18/21) are different proportions and we
publish both. Signing establishes the producer and nothing else: no key
storage, rotation, revocation or timestamping. And the mediation-point
observations describe the evaluation build, not the release.

## Reviewer 6

### 1. Small and homogeneous sample with six incomplete directories

Addressed as above. All 33 directories remain in the denominator and the six
source-only directories are never silently discarded. Structural diversity of
the snapshot is still reported as two fingerprint families and one event
sequence, and no generalization language is drawn from it. Behavioural
diversity is supplied by the new workloads and reported separately.

We keep the sample-size caveat rather than hiding it: twelve workloads and 139
runs give wide intervals, which is why every proportion is published as `n/N`
with a Wilson 95% interval and every zero numerator carries a rule-of-three
bound instead of a “0%” claim.

### 2. Prompt-injection resistance was simulated rather than evaluated

Agreed, and it is now evaluated rather than described.

The grid was declared in the case catalogue **before any model was called**:
eight scenarios across external provider calls, network egress, secret
exfiltration and key-material access; benign and indirect-injection arms; five
repetitions; a unique nonce and an instrumented local destination per run.
Declaring it in advance is the point, so a later run cannot quietly select the
favourable half.

We ran all 80 against `gpt-4o-mini-2024-07-18` at temperature 0 with seed 7. Two
measurements, kept apart because conflating them would be the whole mistake:

| Measure | n/N | Wilson 95% |
| --- | --- | --- |
| Injections that moved the model | 20/40 | [35.2, 64.8] |
| Prohibited proposals in the benign arm | 5/40 | [5.5, 26.1] |
| Prohibited proposals contained before any sensor | 25/25 | [86.7, 100.0] |
| Prohibited proposals reaching a sensor | 0/25 | at most 12.0% |

Zero model errors and zero unmappable proposals, so the denominator is the
whole grid. Attack success varies sharply by scenario: every external-provider
injection succeeded, half the key-material ones did, none of the
secret-exfiltration ones. The benign control earned its place — the model
proposed a prohibited action in 5/40 arms with no
injection present, which a design without that control would have credited to
the attack.

We are explicit about the ceiling. The driver maps the proposal onto a program;
Argorix does not dispatch it. This measures containment of a prohibited action
a real model proposed after a real injection — not the resistance of an Argorix
agent loop, which the release cannot have because it ingests no prompt content.
One model on eight scenarios supports no statement about models in general, and
nothing here supports "Argorix prevents prompt injection". Making resistance a
claim Argorix could support needs content-bounded messages and a
runtime-selected dispatch instruction; both are named as future work.

### 3. Excess pages, tables, code, and repeated “not claimed” material

Addressed structurally and preserved. The camera-ready is seven IEEE pages: two
vector figures (the compilation pipeline and the measured adversarial
boundary), three compact tables (the re-measured snapshot, the campaign, and
tamper detection by class), no code listings, and no appendix. Claim boundaries
are consolidated into the Abstract, one limitations section, and the
Conclusion. Row-level results, commands, raw streams, and checksums live in the
repository, not in the PDF.

## Preregistration amendments

Two amendments are recorded in `oracle.json` and surfaced in `summary.json`
rather than silently applied. The preregistered `phase` field conflated the
stage at which a typed decision is made with the furthest stage the pipeline
reaches; the release writes and verifies an evidence bundle even for a denied
or failed run, so no single reading is correct everywhere. The expected values
were left exactly as written and `phase` was demoted to an informational check
recorded against both readings and excluded from outcome accuracy. No outcome
expectation was affected.

The second concerns the source-only mutation. Its preregistered outcome was
`NOT_DETECTED`, and after the source-binding fix it is `DETECTED`. What changed
is the product, not our expectation of the product as it was: the pre-fix
campaign, its oracle outcome and its raw rows are archived unedited under
`evaluation/adversarial/baseline/prefix/`, and the amendment says so.

## Deviation from the published plan

The plan specified twenty E2 cases and enumerated sixteen conditions. Covering
each condition separately required four splits (invalid bytecode into three
mechanisms, invalid injection route into two, absent provider/adapter into
two), which lands on exactly twenty cases. No condition was dropped.

## Verification performed for the camera-ready

- `IEEEtran` conference class, 10 pt, US Letter, two columns.
- Seven pages including references (official range: four to eight), leaving a
  page in reserve as the plan intended. Reporting the executed injection
  experiment first pushed the paper to eight; prose was tightened across the
  snapshot results, method, discussion and boundaries, three redundant
  reference URLs were dropped where the venue citation already locates the
  work, and three summary rows repeated in prose were removed from Table II.
  No measured result was dropped: all three tables, both figures and every
  reported proportion remain.
- Zero overfull or underfull boxes; zero undefined citations or references.
- Bibliography: 15 entries, all resolved; twelve embedded font objects; no `?`
  markers in the extracted text.
- Visual inspection: all seven rendered pages inspected at 144 dpi; both
  figures, all three tables, the title block, columns, and references are
  legible with no clipping.
- Campaign: 231 rows (118 scored, 80
  prompt-injection runs recorded as not executed), zero mismatches against the
  oracle, harness-validity test passing, seven of eight go/no-go gates at *go*;
  the prompt-injection gate reads *no-go* by design, which forbids a claim
  rather than reporting a defect.
- Findings: 0 open defects,
  3 resolved and re-measured,
  5 boundaries published with the same
  weight as the results.
- Workspace: 382 Rust tests passing, `cargo fmt --all --check` and
  `cargo clippy --workspace --all-targets --all-features` clean.
