# Offline-Verifiable Runtime Evidence Bundles for Governed AI Agent Systems

*Gustavo Venegas, Edison Vazquez, Danilo Naranjo, Benjamin Gonzalez*

> Derived from the ArgorixLang preprint. No new experiments, metrics, citations, or
> deployment claims are introduced. All empirical values are reused unchanged from the
> original ArgorixLang corpus.

## Abstract

Governed AI agent runtimes emit model calls, tool decisions, provider-boundary
outcomes, and policy results, but the evidence of what happened is usually a
human-facing UI label, a provider log, or an informal status field that a consumer
must trust without independent recomputation. We present ArgorixLang's *EvidenceBundle*,
a local runtime artifact that links a compiled bytecode program, an execution trace, a
security report, and a trace-event ledger digest through semantic SHA-256 digests
computed over canonical JSON. A verifier recomputes each digest and re-derives the
ledger digest from the trace events, then checks that the security report's
LedgerSummary digest matches the bundle, establishing cross-artifact consistency
without network access, provider credentials, a live runtime, or any external service.
We give a lightweight formal model of the verification predicate and prove that it is
distinct from policy approval: a bundle can verify while the security report records
policy failure and a review-required state. In an existing corpus of 33 request
directories, 27 are complete and pass offline verification (27/27); 6 are source-only
and not assessable. The corpus contains 7,155 counted ledger events and 1,188 detailed
policy violations, and every complete session records `policy_passed=false`,
`review_required=true`. We hold a strict interpretation boundary: successful
verification proves internal artifact consistency relative to the bundle only --- not
source integrity, producer authentication, absence of side effects, policy approval,
certification, or production isolation.

**Keywords:** AI agents, runtime evidence, evidence bundles, offline verification,
semantic digests, canonical JSON, agent governance, auditability, policy evidence,
reproducibility.

---

## I. Introduction

A governed AI agent does not execute in a vacuum. A single request can issue model
calls, invoke tools, cross a provider boundary, trigger policy evaluation, and produce
a verdict. *Governance* means a downstream party --- an auditor, an operator, a
compliance reviewer, or a second agent --- must later be able to determine what the
runtime actually did. The difficulty is that the evidence on offer is almost always
*mediated*: a dashboard renders a green checkmark, a provider returns an opaque log
line, a human writes "reviewed --- OK" into a status field. Each of these is a summary
produced by a party the consumer must already trust, and none of them can be
independently recomputed from the artifacts themselves.

This is unsatisfactory for governed deployments for three reasons. First, a favorable
*summary* can mask an unfavorable *underlying* result: a top-level "checks passed"
banner says nothing about whether a detailed policy object recorded a violation.
Second, evidence tied to a *live* service is unavailable precisely when it is most
needed --- after the fact, offline, when the provider is gone or uncooperative. Third,
a mediated label conflates distinct predicates --- *did the artifacts survive
unchanged?* and *was the behavior approved?* --- that governance requires be kept apart.

We argue that runtime evidence for governed agents must instead be (i) *inspectable
after execution*, as concrete artifacts on disk; (ii) *verifiable without a live
provider*, using only local recomputation; and (iii) *claim-bounded*, so that what
verification establishes is never silently promoted into what it does not. ArgorixLang
realizes this with the **EvidenceBundle**: a local artifact that records semantic
SHA-256 digests of a compiled bytecode program, an execution trace, and a security
report, plus a digest of the trace-event ledger, together with a relative-path
artifact map. A verifier loads the referenced artifacts, recomputes every digest over
canonical JSON, re-derives the ledger digest from the trace events, and checks that the
report's embedded LedgerSummary digest matches the bundle --- with no network, no
credential, and no external service.

The central stance of this paper, and its central claim boundary, is that
**artifact integrity is not policy approval.** A verified bundle proves only that the
bytecode, trace, and report are mutually consistent relative to the bundle. It does not
prove that the source compiled to that bytecode, who produced the bundle, that no side
effects occurred outside instrumentation, or that any policy approved the behavior. We
make this separation formal and show that the existing corpus contains cases that are
*verifiable and policy-failing simultaneously*.

**Contributions.**

1. A runtime evidence model for governed AI agent systems, expressed as concrete
   on-disk artifacts rather than mediated summaries.
2. A semantic-digest **bundle** linking bytecode, trace, security report, and a
   trace-event ledger digest over canonical JSON.
3. A formal **offline verification predicate** over those artifacts, with an
   accompanying verification algorithm requiring no live service.
4. A formal **interpretation boundary** separating EvidenceBundle verification from
   policy approval, stated as `Verify ⇏ PolicyApproved`.
5. An **observational evaluation** using the existing ArgorixLang runtime corpus,
   reporting what the evidence does and does not establish.

A scope note distinguishing this paper from the companion Agent Passport paper is given
in Section XIII.

---

## II. Background and Motivation

### A. What an agent runtime produces

A governed agent runtime emits a heterogeneous record: model invocations, tool-call
decisions, provider-boundary crossings (or refusals), policy evaluations, and a final
verdict. ArgorixLang serializes this record into four artifact *kinds* downstream of an
upstream source program: a compiled bytecode program, an ordered execution trace, a
structured security report, and an evidence bundle that ties them together. The
distinction between these kinds matters because they play different evidential roles.

### B. Logs, reports, provenance, attestation, evidence bundles

These terms are often used interchangeably and should not be. A *log* is an
append-only stream, typically unverifiable after the fact and trusted by provenance of
the emitter. A *report* is an interpretation: a structured judgment over a run. A
*provenance record* asserts how an artifact was produced, and in signed forms (e.g.,
in-toto) binds that assertion to a key. An *attestation* is a signed statement by an
identified party. An *evidence bundle*, as used here, is none of these: it is an
*unsigned, local, recomputable cross-artifact consistency record*. It says "these
specific artifacts hash to these values and agree with one another," and nothing about
who produced them.

### C. Why digest verification helps but is not trust

A semantic digest detects *change relative to a recorded value*. If a verifier
recomputes `H(bc)` and it matches `B.bytecode_digest`, the bytecode is the one the
bundle recorded. This is genuinely useful: it makes silent tampering of a *self-
contained* bundle detectable, and it is reproducible offline. But it establishes
nothing about *origin* --- an adversary who controls artifacts and bundle together can
produce a perfectly self-consistent bundle with attacker-chosen content. Digest
verification is a consistency primitive, not a trust primitive.

### D. Why policy evidence must preserve failure

A governance evidence system is only useful if it *faithfully preserves negative
results*. The temptation in agent UIs is to collapse a run into a single optimistic
status. ArgorixLang's security report instead retains detailed policy objects ---
per-block results, violations, and a `review_required` flag --- so that a favorable
coarse label cannot overwrite a detailed failure. Preserving the negative is the whole
point: an auditor needs to see the violation, not a smoothed-over banner.

### E. Relation to adjacent systems

in-toto provides *signed* supply-chain provenance; EvidenceBundles are local and
unsigned. OPA is an external policy *decision engine*; ArgorixLang records policy
outcomes inside runtime artifacts rather than delegating the decision. WASI provides
capability isolation at the host; bundle verification proves nothing about host
sandboxing. MCP and A2A standardize agent interaction surfaces; bundles record local
runtime boundaries, not live interoperability. The NIST AI Risk Management Framework
and the OWASP Top 10 for LLM Applications frame risk and inform our threat
discussion, but neither is a certification we claim to satisfy.

---

## III. Evidence Artifact Model

We define each artifact technically and fix its verification scope. Table I summarizes.

### 1. Source --- `session.argx`

The upstream compiler input. The source is **outside EvidenceBundle verification**: the
bundle carries *no* source path and *no* source digest. Verification therefore begins
at compiled bytecode and never asserts that `session.argx` produced it.

### 2. Bytecode --- `session.argbc.json`

The compiled artifact loaded by the VM. Verified by `bytecode_digest`. Cross-checks on
`language`, `module`, and `bytecode_version` guard against gross substitution.

### 3. Trace --- `session.trace.json`

An ordered ledger of runtime and policy events --- agent and policy declarations,
message scheduling/delivery, handler execution, policy evaluations, and VM lifecycle
markers. Verified by `trace_digest`, and additionally the source of the ledger digest
(below).

### 4. SecurityReport --- `session.security.json`

A structured interpretation of runtime state: execution status, policy outcomes
(`passed`, `violations`, `review_required`), provider boundary (executable providers,
blocked attempts), call counts, and a verdict. Verified by `report_digest`. It contains
a **LedgerSummary** holding `events_total`, event-kind counts, first/last event, and a
`ledger_digest` that must equal the bundle's `ledger_digest`.

### 5. EvidenceBundle --- `session.evidence.json`

The artifact map plus four semantic digests: `bytecode_digest`, `trace_digest`,
`report_digest`, and `ledger_digest`. The artifact map (`bytecode_path`, `trace_path`,
`security_report_path`) holds **relative** paths confined to the bundle's portable
subtree; the bundle has no source path and no standalone ledger path.

### 6. Ledger digest

Computed as `H(events(tr))` --- the canonical digest of the trace's event list, *not* a
separate file. It is checked twice: against the bundle's `ledger_digest`, and against
the SecurityReport's embedded LedgerSummary digest. This double linkage ties the
trace's events to the report's claims about them.

**Scope, stated plainly.** The EvidenceBundle's verifiable region *starts* at bytecode,
trace, and security report. It does not verify that `session.argx` produced the
bytecode; it does not authenticate who produced the bundle; and it does not prove
security, compliance, or absence of effects.

**Table I --- Runtime evidence artifacts**

| Artifact | Path | Role | Verified by | Out-of-scope |
|---|---|---|---|---|
| Source | `session.argx` | Upstream compiler input | --- (not in bundle) | No source digest; not linked to bytecode |
| Bytecode | `session.argbc.json` | Compiled program | `bytecode_digest` | Source-to-bytecode provenance |
| Trace | `session.trace.json` | Ordered runtime/policy event ledger | `trace_digest`, `ledger_digest` | Events outside instrumentation |
| SecurityReport | `session.security.json` | Structured runtime/policy interpretation | `report_digest`; LedgerSummary match | Truth of declared metadata |
| EvidenceBundle | `session.evidence.json` | Artifact map + semantic digests | self-describing; cross-checked | Producer identity; signatures |
| Ledger digest | (none; from `trace.events`) | Binds trace events to report | `H(events(tr))` | Standalone ledger file |

---

## IV. Formal Model

We give a lightweight formalization. It is an *interpretation boundary*, not a
cryptographic security proof.

**Definitions.** Let `B` be an EvidenceBundle; `bc`, `tr`, `sr` the bytecode, trace, and
security-report artifacts; `events(tr)` the ordered trace event list; `LS(sr)` the
LedgerSummary digest stored in the security report; `C(x)` the canonical JSON
serialization of `x`; and `H(x) = SHA-256(C(x))`.

**Semantic-digest equations.**

```
B.bytecode_digest = H(bc)
B.trace_digest    = H(tr)
B.report_digest   = H(sr)
B.ledger_digest   = H(events(tr))
LS(sr)            = B.ledger_digest
```

**Verification predicate.**

```
Verify(B, bc, tr, sr) :=
      B.bytecode_digest = H(bc)
    ∧ B.trace_digest    = H(tr)
    ∧ B.report_digest   = H(sr)
    ∧ B.ledger_digest   = H(events(tr))
    ∧ LS(sr)            = B.ledger_digest
```

**Policy approval (a separate predicate).**

```
PolicyApproved(sr) :=
      sr.policy_passed   = true
    ∧ sr.review_required = false
    ∧ |sr.violations|    = 0
```

**Interpretation boundary (central result).** Verification does not imply approval:

```
Verify(B, bc, tr, sr)  ⇏  PolicyApproved(sr)
```

The two predicates read disjoint fields. `Verify` reads digests and the LedgerSummary;
`PolicyApproved` reads `policy_passed`, `review_required`, and `violations`. Nothing in
`Verify` constrains the latter, so a faithfully recorded policy failure passes through
verification unchanged. The observed corpus *witnesses* the satisfiable conjunction:

```
∃ sr :  Verify(B, bc, tr, sr)  ∧  ¬PolicyApproved(sr)
```

because complete bundles verify (27/27) while their security reports record
`policy_passed=false` and `review_required=true`. We deliberately do *not* present
`Verify` as proof of authenticity, source integrity, or external truth; it is a
statement of internal cross-artifact consistency relative to `B`. Table III restates
these predicates.

---

## V. Offline Verification Procedure

The verifier is a pure function of local files. It loads the bundle; resolves
`bytecode_path`, `trace_path`, and `security_report_path` relative to the bundle
directory (rejecting absolute paths and paths escaping the portable subtree); loads the
three artifacts; canonicalizes each to JSON; recomputes `bytecode_digest`,
`trace_digest`, and `report_digest`; recomputes `ledger_digest` from `trace.events`;
compares the SecurityReport's LedgerSummary digest against the bundle's `ledger_digest`;
and returns *verified* only if all checks pass. It requires **no network access, no
provider credential, no live runtime, and no external service.** Auxiliary checks
(version-field agreement across artifacts, digest syntactic well-formedness, path
confinement) accompany the core digest comparisons.

**Algorithm 1 --- Offline EvidenceBundle Verification**

```
Input:  bundle file B (referencing bc, tr, sr)
Output: verified ∈ {true, false}

 1.  Load B.
 2.  Resolve B.artifact_map paths (relative, inside portable tree).
 3.  Load bc, tr, sr.
 4.  d_bc     ← H(bc)
 5.  d_tr     ← H(tr)
 6.  d_sr     ← H(sr)
 7.  d_ledger ← H(events(tr))
 8.  check d_bc     = B.bytecode_digest
 9.  check d_tr     = B.trace_digest
10.  check d_sr     = B.report_digest
11.  check d_ledger = B.ledger_digest
12.  check LS(sr)   = B.ledger_digest
13.  return verified  iff  all checks (8)–(12) hold
```

---

## VI. Integrity Is Not Approval

This section is the conceptual core. Six statements bound what a verified bundle means.

1. **Consistency, not external truth.** A valid bundle proves the artifacts agree
   relative to the bundle. It does not reach outside the bundle to validate any claim
   the artifacts make.
2. **Change detection, not origin.** A matching digest detects change relative to a
   recorded artifact. It does not prove *who* produced the artifact; an unsigned bundle
   has no producer binding.
3. **Instrumented events, not all events.** A verified trace is a faithful record of
   instrumented events. It cannot prove that no effect occurred *outside* the
   instrumentation boundary.
4. **Faithful negatives.** A verified SecurityReport can preserve a negative policy
   result without contradiction; verification of the report is orthogonal to the
   report's verdict.
5. **Detail dominates summary.** A favorable top-level status field must not override a
   detailed policy failure. Component status and `review_required` dominate
   interpretation.
6. **The central statement.** *A bundle can be offline-verifiable and still contain
   policy failure, a review-required status, unknown-rule violations, or otherwise
   non-certified behavior.*

Table V tabulates the integrity-versus-approval distinction across specific claims.

---

## VII. Observational Evaluation

We reuse the existing ArgorixLang runtime corpus and add no new experiment.

**Corpus.** Of **33** request directories, **27** are *complete* (containing
`session.argx`, `session.argbc.json`, `session.trace.json`, `session.security.json`,
and `session.evidence.json`) and **6** are *source-only* and not assessable for
trace/security/evidence fields. The corpus contains **7,155** counted ledger events and
**1,188** detailed policy violations across the complete sessions.

**Offline verification.** All **27/27** complete bundles pass offline verification: the
three semantic digests reproduce, the ledger digest recomputes from `trace.events`, and
the LedgerSummary digest matches the bundle. No bundle digests or verifies
`session.argx`.

**Policy results.** Every one of the 27 detailed policy reports sets
`policy_passed=false` and `review_required=true`, with **44** unknown-rule violations
per complete session. Verification success therefore coexists with policy
non-approval --- the witnessed conjunction of Section IV.

**Table IV --- Observational corpus results**

| Measure | Observed value | Interpretation |
|---|---|---|
| Request directories | 33 | Full corpus |
| Complete directories | 27 | Assessable for trace/report fields |
| Source-only directories | 6 | Not assessable; explicitly incomplete |
| Bundles passing offline verification | 27 / 27 | Internal consistency only |
| Counted ledger events | 7,155 | Pipeline emits structured events |
| Detailed policy violations | 1,188 | Negatives preserved, not smoothed |
| Unknown-rule violations / session | 44 | Faithful per-session failure detail |
| Complete reports with `policy_passed=false` | 27 / 27 | Verify ⇏ PolicyApproved (witnessed) |
| Complete reports with `review_required=true` | 27 / 27 | Review state preserved |

**Interpretation.** The evidence pipeline is *reproducible* across complete sessions and
offline verification works for every complete bundle. Missing artifacts surface as
*explicit incomplete observations*, not silent passes --- a source-only directory is
*unavailable*, never *verified*. Verification success does not imply policy success.
The repeated structure across sessions supports pipeline reproducibility but limits any
claim of behavioral diversity.

---

## VIII. Threat Model and Security Analysis

We analyze threats as attack paths across trust boundaries, not as proof that a control
defeats every adversary.

**Threats.** *Artifact tampering* --- altering bytecode, trace, or report after the
fact. *Bundle replacement* --- swapping the bundle for another. *Report/status
confusion* --- a consumer reading a coarse status and missing a detailed failure.
*Missing source integrity* --- no link from bytecode back to `session.argx`. *Producer
impersonation* --- claiming a bundle came from a party that did not produce it.
*Unsigned bundle replay* --- re-presenting an old self-consistent bundle. *Runtime side
effects outside instrumentation* --- effects the trace cannot witness. *Consumer
overinterpretation* --- treating consistency as certification. *Malicious but
self-consistent bundle generation* --- an adversary controlling all artifacts emits a
coherent bundle with chosen content.

**Bounded mitigations.** Semantic-digest linkage detects post-hoc change to a recorded
artifact; the ledger digest derived from trace events ties events to the report's
LedgerSummary; cross-artifact consistency checks (including version-field agreement and
path confinement) catch mismatched substitution; explicit incomplete-session handling
prevents missing artifacts from masquerading as passes; component-level policy status
and `review_required` are preserved; and the whole procedure is offline-reproducible.

**Remaining gaps.** There are *no* signatures, *no* trusted timestamp, *no* transparency
log, *no* source digest, *no* hardware root, *no* independent witness, *no* OS-level
sandbox proof, *no* producer authentication, and *no* revocation or witness model.
Tampering and replacement are defeated only when the adversary cannot also rewrite the
bundle; against an adversary who controls artifact and bundle together, self-consistency
is necessary but not sufficient for trust. The strongest supported statement is
conditional: *within the recorded artifact set, complete bundles are offline-consistent
and policy negatives are preserved.* Trust outside that set is unmeasured.

---

## IX. Related Work

We compare carefully and claim no equivalence. **in-toto** provides signed supply-chain
provenance binding steps to keys; ArgorixLang EvidenceBundles are *local, unsigned*
runtime-evidence records with no producer binding. **OPA** is an external policy
decision engine evaluating Rego; ArgorixLang *records* policy and evidence outcomes in
runtime artifacts rather than externalizing the decision. **WASI** provides
capability-based host isolation; evidence verification proves nothing about host
sandboxing. **MCP** and **A2A** standardize agent interaction and interoperability;
EvidenceBundles record *local* runtime boundaries, not live interaction. The **NIST AI
RMF** and the **OWASP Top 10 for LLM Applications** provide risk framing that motivates
the threat model; we map to neither as a certification. More broadly, the bundle sits in
the lineage of provenance and reproducibility systems but trades signatures and trust
anchors for offline recomputability and a strict claim boundary.

---

## X. Limitations

We state limitations strictly. There is **no** source integrity (no source digest, no
bytecode-to-source link); **no** producer authentication; **no** signatures; **no**
trusted timestamp; **no** transparency log; **no** blockchain immutability; **no**
post-quantum security; **no** policy-approval guarantee; **no** legal or compliance
certification; **no** host-level containment proof; **no** controlled adversarial test;
**no** prompt-level qualitative analysis (prompt content is unavailable in the corpus);
**no** proof of semantic correctness of every trace event; and **no** proof of absence
of side effects outside the instrumented runtime. The complete sessions are
structurally uniform, which supports reproducibility but limits generalization across
prompts, policies, and jurisdictions.

---

## XI. Future Work

Natural extensions, none claimed as present: **source-digest inclusion** so verification
extends to `session.argx`; **signed EvidenceBundles** for producer authentication and
non-repudiation; a **transparency log** for independent witnessing; **trusted
timestamps**; **producer identity binding**; an **independent witness service**;
**policy-decision canonicalization** into a single tri-state decision from typed
component results; **differential verification** across runtime versions; **host-level
sandbox measurement**; a **controlled adversarial corpus**; a **public reproducibility
package**; and an **independent external verifier** implementation.

---

## XII. Conclusion

Offline-verifiable EvidenceBundles provide a reproducible *integrity layer* for governed
AI agent runtimes. They link bytecode, trace, security report, and a trace-event ledger
digest through semantic digests over canonical JSON, and a verifier re-establishes that
linkage with no live service. Their value must be read narrowly: artifact consistency,
trace/report linkage, faithful policy-result preservation, and offline auditability ---
*not* security certification, policy approval, or external truth. The corpus shows the
pipeline is reproducible (27/27 complete bundles verify) and, decisively, that
verification and approval are distinct predicates: every complete session verifies while
recording policy failure and a review-required state.

---

## XIII. Scope --- Difference from the Agent Passport Paper

This paper differs from the companion Agent Passport paper by focusing on the *runtime
evidence and offline verification mechanism* rather than sovereign metadata. The Agent
Passport paper concerns *what* metadata (country, jurisdiction, residency, policy
bindings, local agent name) is compiled and preserved; this paper concerns *how*
bytecode, trace, report, and ledger evidence are linked through semantic digests and
verified offline, and where the resulting integrity claim stops. All empirical values
are reused from the original ArgorixLang corpus, and no new experiment, deployment, or
security guarantee is introduced.

---

### Table II --- EvidenceBundle digest fields

| Digest field | Computed from | Purpose | Does not prove |
|---|---|---|---|
| `bytecode_digest` | `H(bc)` over canonical JSON | Pin the compiled program | Source produced this bytecode |
| `trace_digest` | `H(tr)` over canonical JSON | Pin the full trace artifact | Completeness of instrumentation |
| `report_digest` | `H(sr)` over canonical JSON | Pin the security report | Truth of the report's verdict |
| `ledger_digest` | `H(events(tr))` | Bind trace events to LedgerSummary | Events outside the trace |

### Table III --- Formal predicates

| Predicate | Definition | Interpretation |
|---|---|---|
| `H(x)` | `SHA-256(C(x))` | Semantic digest over canonical JSON |
| `Verify(B,bc,tr,sr)` | all five digest/LedgerSummary equalities hold | Internal cross-artifact consistency relative to `B` |
| `PolicyApproved(sr)` | `policy_passed ∧ ¬review_required ∧ no violations` | Approval as recorded by the report |
| `Verify ⇏ PolicyApproved` | verification does not entail approval | Integrity boundary (central result) |
| `∃: Verify ∧ ¬PolicyApproved` | witnessed by the corpus | 27/27 verify while policy-failing |

### Table V --- Integrity vs. approval distinction

| Claim | Supported by verification? | Reason |
|---|---|---|
| Artifacts unchanged relative to the bundle | Yes | Digests recompute and match |
| Trace events match the report's LedgerSummary | Yes | `H(events(tr)) = LS(sr) = B.ledger_digest` |
| The source produced this bytecode | No | No source path or digest in the bundle |
| A known party produced the bundle | No | Unsigned; no producer binding |
| No side effects occurred outside instrumentation | No | Trace witnesses only instrumented events |
| Policy approved the behavior | No | Separate predicate; corpus shows failure |
| Legal/compliance certification | No | No certification mechanism present |
| Host/production isolation | No | No sandbox measurement |

---

## Figures

**Figure 1 --- Evidence artifact graph.** Source (`session.argx`) sits *outside* the
verification boundary (dashed edge to the compiler). Inside the offline-verifiable
region: Bytecode, Trace, SecurityReport, and EvidenceBundle, with the EvidenceBundle's
digests pointing at bytecode/trace/report and the ledger digest derived from
`trace.events` and matched against the report's LedgerSummary.

**Figure 2 --- Offline verification sequence.** Load bundle → resolve relative artifact
paths → load bytecode/trace/report → recompute `bytecode_digest`, `trace_digest`,
`report_digest` → recompute `ledger_digest` from `trace.events` → compare LedgerSummary
digest → return verified / fail. No network or provider participates.

**Figure 3 --- Two-axis interpretation.** X-axis: artifact integrity (verification
fail → pass). Y-axis: policy approval (not approved → approved). The corpus populates
the *verified-but-not-approved* quadrant (high integrity, low approval): every complete
session is offline-verifiable yet records policy failure. This quadrant is the visual
statement of `Verify ⇏ PolicyApproved`.
