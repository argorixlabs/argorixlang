# Agent Passport: Compiled Sovereign Metadata for Governed AI Agent Runtime Systems

*IEEE conference-style technical paper draft (English). Formatted as plain Markdown for direct conversion to IEEEtran LaTeX. Citation keys reuse the original ArgorixLang preprint `references.bib`; placeholders marked `[X]` carry a verification note.*

**Authors (carry over / confirm before submission):** Gustavo Venegas¹, Edison Vazquez¹, Danilo Naranjo², Benjamin Gonzalez¹
¹ Chilean Chamber of Artificial Intelligence  ² Ocular

---

## Abstract

Agentic applications increasingly span model providers, tools, data flows, and inter-agent protocols, yet their jurisdictional, residency, identity, and governance assumptions are typically expressed outside the executable artifact—scattered across prompts, deployment configuration, and application code. This produces a reviewability gap: an auditor or runtime consumer cannot determine from one artifact which identity metadata was declared, which jurisdictional constraints applied, which policy bindings accompanied an agent, or whether those assumptions survived compilation into runtime evidence. We describe the **Agent Passport**, a construct in the ArgorixLang language and runtime that represents sovereign agent metadata—country, jurisdiction, permitted data residency, capabilities, policy bindings, and a local agent name (`ans_name`)—as a typed, semantically validated program declaration that is lowered into intermediate representation and bytecode and surfaced in runtime traces, security reports, and offline-verifiable evidence bundles. We emphasize a strict claim boundary: a passport is a *local declaration*, not legal identity, not decentralized-identifier (DID) verification, not credential issuance, not operational naming/DNS, and not authentication. We report an observational study of an existing ArgorixLang artifact corpus of 33 runtime request directories, of which 27 are complete and pass offline digest and cross-artifact verification; each complete session reports two passports with country and jurisdiction Chile (`CL`) and residency declarations including `CL` and `EU`, while the bounded `injected.content` field is absent from all 27 complete traces. The results show that passport metadata can be compiled and preserved into runtime evidence; they do not demonstrate real identity authentication, credential verification, operational discovery, legal compliance, or physical residency enforcement.

**Keywords:** AI agents, agent governance, sovereign metadata, identity metadata, runtime evidence, compiled governance, data residency, policy enforcement.

---

## I. Introduction

An agentic application is not a single model invocation. It is a composition of principals (agents), typed messages, provider operations, tools, policy decisions, and the evidence those decisions leave behind. As such systems operate across model providers, external tools, and inter-agent protocols, they accumulate governance assumptions—*who* the agent claims to be, *under which jurisdiction* it operates, *where* its data may reside, and *which policies* bind it. In current practice these assumptions live outside the executable artifact: in natural-language prompts, in deployment manifests, in code comments, or in human process. When that happens, a reviewer cannot reconstruct from one artifact which identity metadata was declared, which jurisdictional constraints were asserted, which policy bindings were associated with the agent, or whether any of this survived into the runtime record.

We call this the **reviewability gap** for sovereign agent metadata. Standardization efforts address adjacent surfaces—Model Context Protocol (MCP) and Agent2Agent (A2A) standardize interaction surfaces [mcp_spec_2025], [a2a_spec]; W3C Decentralized Identifiers (DIDs) and Verifiable Credentials (VCs) define interoperable identity and credential data models [w3c_did_core_2022], [w3c_vc_data_model_2025]—but none of these, by itself, makes an application's *local* governance metadata compile-checked and reproducibly present in its execution evidence.

This paper isolates and develops one construct from the ArgorixLang language and runtime [argorixlang2026]: the **Agent Passport**. A passport is a typed, semantically validated declaration that binds an agent to a declared country, jurisdiction, permitted residency set, capabilities, policy bindings, and a local naming field (`ans_name`), together with optional identity, credential, and trust metadata. The compiler retains these declarations through semantic validation, lowers only validated forms into intermediate representation (IR) and bytecode, and the runtime surfaces the resulting metadata into a trace, a SecurityReport, and an offline-verifiable EvidenceBundle.

The central design stance—and the central claim boundary of this paper—is that the Agent Passport is about **representation, validation, and evidence preservation**, not real-world identity verification. The passport does not verify external identity, does not resolve a DID, does not issue or check a credential, does not operate as DNS, and does not prove that data physically resided in any region. What it provides is a *typed, inspectable, runtime-visible metadata boundary*: a place where sovereign metadata is declared once, checked for internal consistency, and carried into evidence that a downstream consumer can inspect offline.

**Contributions.**
1. A compiled **Agent Passport abstraction** for governed AI agents, treating sovereign metadata as a first-class, type-checked language construct rather than out-of-band configuration.
2. A **semantic validation model** for passport–agent–policy–residency bindings, including the rejection of dangling references and invalid bindings.
3. A **runtime evidence path** showing how passport metadata survives compilation into IR/bytecode and appears in traces, security reports, and offline-verifiable bundles.
4. A **claim-boundary taxonomy** that separates *implemented* mechanisms, *declarative* metadata, *proposed* architecture, and properties *not claimed*—applied uniformly so digest success is never promoted to external assurance.
5. An **observational evaluation** over the existing ArgorixLang runtime artifact corpus, reporting what passport evidence the corpus does and does not establish.

To prevent overreading, every claim is assigned to one of four regions (implemented / declarative / proposed / not claimed) before it is discussed (Table II, Fig. 2). We make no claim of real identity authentication, credential verification, cryptographic handshake execution, legal-compliance certification, blockchain immutability, post-quantum security, or production isolation.

A note on scope relative to the original preprint is given at the end (§XII): that paper covers the full ArgorixLang language, runtime, and evidence pipeline; this paper narrows to the Agent Passport construct and the sovereign-metadata evidence path.

---

## II. Background and Motivation

### A. Agent systems as governed compositions

An agent runtime mediates interactions among agents, typed messages, provider operations, tools, policy decisions, and evidence. Governance questions—authorization, provider permission, residency, identity context—are properties of that composition, not of any single prompt. When governance lives outside the executable artifact, three failure modes follow: (i) a reviewer cannot tell an *omitted* constraint from an *approved* one; (ii) two artifacts (e.g., a deployment manifest and the running code) can silently disagree; and (iii) a favorable human-facing summary can mask an unfavorable underlying decision.

### B. Naming vs. identity vs. credentials vs. policy

A recurring category error conflates four distinct functions. A **resolvable name** answers *"where is the record?"*. An **authenticated identity** answers *"who controls the relevant key or account?"*. A **credential** answers a scoped, issuer-signed assertion. A **policy decision** answers *"is this operation permitted in context?"*. These cannot substitute for one another. The Agent Passport deliberately occupies only the *local declaration* slice of this space: it records a local name and locally asserted metadata, and binds them to policy declarations, without claiming to resolve, authenticate, or verify anything external.

### C. Why residency and jurisdiction metadata matter

Data-residency and jurisdiction constraints are increasingly first-order governance inputs for agent deployments operating across regions. Today these constraints are usually informal. Making them *explicit and compile-checked*—even as declarations—lets a runtime record which residency set and jurisdiction accompanied an operation, and lets a reviewer detect when a declared residency value is outside an allowed set. This is an auditability benefit, not an enforcement guarantee: a declared residency of `EU` does not prove that bytes were stored in the EU.

### D. Distinguishing the Agent Passport from adjacent systems

- **DID/VC** [w3c_did_core_2022], [w3c_vc_data_model_2025] define *external* identity and credential models with cryptographic control and issuer signatures. The Agent Passport stores only *local declarations* unless a resolver/verifier is implemented; none is implemented here.
- **DNS / naming / NANDA-style discovery** [mockapetris1987dnsconcepts], [projectnanda2026], [raskar2025beyonddns] provide operational resolution and discovery. The passport's `ans_name` is local metadata; operational discovery is proposed, not implemented.
- **MCP / A2A** [mcp_spec_2025], [a2a_spec] standardize interaction. Passport metadata can *bind to* such boundaries but this paper implements no live interoperability.
- **Policy-as-code (OPA)** [opa_docs] is an external decision engine. ArgorixLang compiles policy *metadata and bindings together with* the passport, rather than delegating to an external engine.

---

## III. Agent Passport Model

### A. Definition

An **Agent Passport** is a local, compiled declaration that collects locally asserted identity and governance context for one subject agent. Its fields are:

- **Passport name** — the declaration's local identifier.
- **Subject agent** — a reference to an agent declared in the same program.
- **Country** — declared country of the agent (observed: `CL`).
- **Jurisdiction** — declared governing jurisdiction (observed: `CL`).
- **Permitted residency** — a set of allowed data-residency values (observed: `CL`, `EU`).
- **Capabilities** — capability references associated with the subject agent.
- **Policy bindings** — references to policy declarations that bind the passport/agent.
- **`ans_name`** — a local agent naming field, a declarative identifier.
- **Optional identity / credential / trust metadata** — references into identity, credential, and ATrust declarations (handled as declarative; see §IV, and the original preprint's ATrust treatment [vazquez_atrust]).

The passport relates to four neighboring construct families: **agent declarations** (the subject), **policies** (bindings), **runtime profiles** (which govern what the runtime will and will not do), and **evidence requirements** (which determine what is emitted into traces, reports, and bundles).

### B. What the passport is — and is not

The passport **is** a local compiled declaration that creates a typed, inspectable, runtime-visible metadata boundary. Within the compiled program, a passport binds a name and sovereign metadata to a specific agent and to specific policies, and that binding is checked.

The passport **is not** a government document, **is not** legal identity, **does not** verify external identity, and **does not** prove physical data residency. The word *sovereign* here denotes *explicit jurisdictional and residency metadata controlled by the program*, not state recognition or legal status. No external registry validates the declared country, jurisdiction, or residency values; they are declarations.

> **Table I.** Agent Passport fields and semantics.

| Field | Meaning | Implemented status | External assurance claim |
|---|---|---|---|
| Passport name | Local identifier of the declaration | Implemented (parsed, validated, lowered, emitted) | None |
| Subject agent | Reference to an in-program agent | Implemented; reference resolution enforced | None |
| Country | Declared country of the agent | Declarative; preserved into evidence | None (not externally validated) |
| Jurisdiction | Declared governing jurisdiction | Declarative; preserved into evidence | None |
| Permitted residency | Allowed data-residency value set | Declarative; membership-checkable | None (no physical residency proof) |
| Capabilities | Capability references for the agent | Implemented binding/validation | None beyond local checks |
| Policy bindings | References to policy declarations | Implemented; reference resolution enforced | None (binding ≠ approval) |
| `ans_name` | Local agent naming field | Implemented as local metadata; survives to artifacts | None (no DNS/DID/registry resolution) |
| Identity/credential/trust metadata | References into identity/credential/ATrust declarations | Declarative; references validated | None (no issuance/verification) |

---

## IV. Compilation and Semantic Validation

ArgorixLang groups declarations into modules, then processes them through five relevant stages, of which passports traverse all five.

1. **Parsing.** The compiler produces a source-faithful abstract syntax tree (AST); passport declarations are retained verbatim in structure, not rewritten or discarded.
2. **Name and module resolution.** Declarations are bound without discarding provenance, so a passport's subject-agent and policy-binding references are resolved against the program's declared names.
3. **Semantic analysis.** Uniqueness, references, cross-construct constraints, and denied combinations are checked. Passport-relevant checks include: the subject agent must exist; each policy binding must reference an existing policy; residency values must belong to an allowed declaration set; identity/credential/trust references must resolve.
4. **Lowering.** Only validated forms are converted to IR and serializable bytecode. Passport metadata is preserved into the lowered artifacts rather than erased—this is what makes it visible at runtime.
5. **VM execution.** The VM loads bytecode, installs only explicitly executable providers, and evaluates policy before scheduling work. Passport constraints and metadata are thereby available to the runtime and to the evidence pipeline.

This ordering matters: a runtime-only check discovers invalid configuration after deployment, and an unchecked lowering pass can erase the distinction between an omitted field and an approved one. Semantic validation rejects, among other cases, unknown references and dangling bindings *before* anything is lowered.

### A. Declarative metadata vs. executable controls

A crucial distinction runs through the passport model. **Executable runtime controls** (e.g., the fail-closed provider boundary, network/secret denial) change what the runtime *does*. **Declarative metadata** (country, jurisdiction, residency, identity/credential references, `ans_name`) is *validated and preserved* but does not, by itself, perform an external action. The passport is primarily declarative metadata with implemented binding/validation and implemented evidence preservation; it does not execute identity verification or residency enforcement.

### B. Validation rules (formal-ish)

The following rules summarize passport semantic validation. Let `P` be a passport, `A` the set of declared agents, `Π` the set of declared policies, and `R` the allowed residency declaration set.

```
R1 (subject existence):     P.subject ∈ A                       else reject(dangling agent reference)
R2 (policy binding):        ∀ b ∈ P.policy_bindings: b ∈ Π      else reject(dangling policy reference)
R3 (residency membership):  ∀ r ∈ P.permitted_residency: r ∈ R  else reject(residency outside allowed set)
R4 (local name scope):      P.ans_name is valid as LOCAL metadata only;
                            it confers no resolution/authentication unless a resolver exists (none here).
R5 (external claims):       identity/credential/trust references remain DECLARATIVE;
                            they confer no verification unless a verifier is implemented (none here).
R6 (lowering gate):         lower(P) is emitted ⟺ R1 ∧ R2 ∧ R3 hold;
                            metadata for country/jurisdiction/residency/ans_name is preserved into IR/bytecode.
```

Rules R1–R3 are enforced bindings/validations. Rules R4–R5 are *boundary* rules: they describe the semantic ceiling of a validated passport. R6 makes explicit that successful lowering is conditioned on reference and membership validity, and that the surviving metadata is what later appears in evidence. None of these rules establishes that a declared value is externally *true*.

---

## V. Runtime Evidence Path

Passport metadata becomes auditable because it is carried into the runtime evidence artifacts. Each **complete** request directory in the corpus contains five artifacts: source (`session.argx`), compiled bytecode (`session.argbc.json`), a trace (`session.trace.json`), a security report (`session.security.json`), and an evidence bundle (`session.evidence.json`).

- **Trace.** An ordered ledger of compiler/VM and policy events. Among its declaration events, each complete trace records declarations for two agents, six policies, **two passports**, plus governance, threat, conformance, release, ATrust, MCP, and A2A metadata. Passport declarations therefore appear in the runtime ledger, not only in source.
- **SecurityReport.** A structured interpretation of runtime evidence: execution status, policy results, review state, provider boundary, and declared controls. It carries the sovereign metadata context alongside the policy outcome.
- **EvidenceBundle.** Records four semantic SHA-256 digests—`bytecode_digest`, `trace_digest`, `report_digest`, and `ledger_digest` (the canonical digest of `trace.events`)—and an artifact map of `bytecode_path`, `trace_path`, and `security_report_path`. There is **no source path or source digest** and no standalone ledger path.

A *semantic digest* is computed over canonical JSON derived from a deserialized value, so it is independent of whitespace or key ordering. **Offline verification** loads bytecode, trace, and the security report; recomputes their three semantic digests; separately computes `ledger_digest` from `trace.events`; and checks that the report's LedgerSummary digest equals the bundle's `ledger_digest`. It requires no network access and no provider credentials. Source integrity is *outside* bundle verification: the verifier begins at compiled bytecode and never asserts that `session.argx` produced it.

**Key claim.** The Agent Passport lets runtime consumers *inspect which sovereign metadata and policy bindings accompanied an agent operation*. It does **not** prove that the metadata is externally true. Three orthogonality statements bound this: a valid digest detects content change relative to the bundle but does not prove who created the bundle; a linked trace does not prove that events outside the instrumented runtime did not occur; and artifact integrity does not turn a failing policy result into approval. No signature, trusted timestamp, transparency service, hardware root, or independent witness is evaluated here. Bundle integrity therefore does **not** imply legal compliance, certification, authentication, or policy approval.

> **Figure 1.** *Agent Passport binding model.* A box **Agent** feeds into **Passport** (carrying country/jurisdiction/residency/`ans_name`/capabilities); the Passport connects by directed edges to **Policy bindings**, which feed a **Runtime profile**; the runtime profile emits to a fan of three boxes — **Trace**, **SecurityReport**, **EvidenceBundle**. Every edge is annotated "validated reference / preserved metadata," not "verified real-world relationship."

> **Figure 3.** *Evidence path.* Left-to-right pipeline: **Source (`session.argx`)** → **Compiler** → **IR / Bytecode** → **VM** → **Trace** → **SecurityReport** → **EvidenceBundle**. The Source→Compiler edge is drawn dashed and labeled "outside bundle verification"; the Bytecode/Trace/Report nodes are enclosed in a shaded "offline-verifiable" region; passport metadata is annotated as flowing from Source through Bytecode into Trace and SecurityReport.

---

## VI. Observational Evaluation

We reuse the existing ArgorixLang runtime artifact corpus. All numbers below are taken directly from the source preprint's normalized datasets; we add no new experiment.

### A. Corpus and completeness

Of **33** request directories, **27** (81.8%) contain all five expected artifacts and **6** (18.2%) contain source only. All 27 complete sessions report execution status `completed`; execution status is unavailable for the six source-only directories, which cannot be assessed for trace-level fields. We retain the six as explicit incomplete/negative observations rather than discarding them.

### B. Offline bundle verification

All **27/27** complete bundles pass the repository's offline verifier. For each, the bytecode, trace, security-report, and trace-event-ledger digest fields are syntactically valid SHA-256 identifiers, the three semantic digests reproduce, the ledger digest recomputes from trace events, and the SecurityReport LedgerSummary digest matches. The verifier does not digest or verify `session.argx`; source integrity is outside this result. Verification establishes *internal cross-artifact consistency*, not producer authentication, absence of side effects, or policy approval.

### C. Passport evidence

Every complete session reports **two passports**, with country and jurisdiction Chile (`CL`) and data-residency declarations including `CL` and `EU`. These values demonstrate that sovereign metadata *survives compilation and appears in runtime evidence*. They do **not** show that data was physically stored in either region, nor that any external registry validated the declarations.

### D. Prompt-content availability

The bounded `injected.content` field is present in **0 of the 27** complete, valid traces; the six source-only directories have no trace and are not assessable. Consequently no qualitative prompt themes, quotations, injection labels, or attack-success rates can be reported from this corpus. This is simultaneously a data-minimization benefit and an evaluation limitation.

> **Table III.** Observed passport evidence in the runtime corpus.

| Measure | Observed value | Interpretation |
|---|---|---|
| Request directories | 33 | Full corpus |
| Complete sessions (5 artifacts) | 27 | Assessable for trace-level fields |
| Source-only sessions | 6 | Not assessable; explicit incomplete result |
| Bundles passing offline verification | 27 / 27 | Internal cross-artifact consistency only |
| Passports per complete session | 2 | Passport metadata survives to evidence |
| Country / jurisdiction declared | `CL` / `CL` | Declaration, not externally validated |
| Residency declarations observed | `CL`, `EU` | Declaration, not physical-residency proof |
| Complete traces with `injected.content` present | 0 / 27 | No prompt-level qualitative analysis possible |

### E. Interpretation

The corpus demonstrates that passport metadata can be **compiled and preserved into runtime evidence** and that complete evidence bundles are **offline-consistent**. It does **not** demonstrate real identity authentication, DID/VC verification, operational DNS/ANS resolution, legal compliance, or physical residency enforcement. The repeated structure (each complete session is highly uniform) supports pipeline reproducibility but weakens any claim about behavioral diversity across prompts, providers, jurisdictions, or attacks.

---

## VII. Security and Governance Analysis

We analyze threats as attack paths across trust boundaries, not as proof that a named control defeats every adversary.

**Threats.**
- *Identity spoofing.* A program may declare arbitrary identity/credential references. Local checks prevent *dangling* references but cannot establish that a referenced identity is genuine.
- *False residency declaration.* `permitted_residency` can be set to any in-set value; membership is checked but truthfulness is not.
- *Policy confusion.* A consumer reading only a coarse top-level status could overlook an unfavorable detailed policy result.
- *Metadata laundering.* Re-emitting a self-consistent bundle with attacker-chosen sovereign metadata is possible if the bundle is unsigned and both artifacts and bundle are replaced together.
- *Overreading.* The most pervasive risk is treating a *local declaration* as *external assurance*—reading `ans_name`, country, or residency as verified facts.

**Bounded mitigations.**
- *Semantic validation* rejects dangling agent/policy/identity references and out-of-set residency values (R1–R3, R6).
- *Explicit claim boundaries* (Table II) and figure/caption discipline keep declarative metadata from being promoted to assurance.
- *Evidence linkage* via semantic digests detects content change relative to a bundle and avoids formatting-dependent hashes.
- *Review states* preserve component-level policy status so a favorable aggregate cannot silently override an unfavorable detail.
- *Separation of local naming from operational discovery* prevents a validated `ans_name` from being mistaken for a resolved/authenticated identity.

**Remaining risks.** Malicious-but-well-formed declarations, fake credentials, resolver attacks (once a resolver exists), missing revocation, adapter compromise, and unsigned bundles all remain in scope for future testing. The strongest supported statement is conditional: *within the observed runtime path, requests are mediated by compiled policy and an executable-provider allowlist, denials are recorded, passport metadata is preserved, and complete artifacts are offline-consistent.* Security outside that path is unmeasured.

> **Table IV.** Principal threats and bounded mitigation claims (passport-focused).

| Threat | Bounded mitigation | Claim status |
|---|---|---|
| Dangling identity/policy reference | Semantic reference validation | Implemented |
| Residency value outside allowed set | Membership check (R3) | Implemented |
| Policy confusion (coarse vs. detailed) | Preserved component status / review state | Implemented (interpretation discipline) |
| Evidence tampering | Digest-linked bundle | Implemented (no producer authentication) |
| False/unverified external identity | — (verifier not implemented) | Not claimed |
| Physical residency enforcement | — | Not claimed |

---

## VIII. Related Work

We compare carefully and claim no equivalence.

- **DID Core and Verifiable Credentials** [w3c_did_core_2022], [w3c_vc_data_model_2025] provide external identity and credential models with cryptographic control and issuer signatures. The Agent Passport stores only *local declarations* and validated references; it performs no DID resolution or credential verification unless a verifier is added.
- **DNS / naming systems / NANDA-style discovery** [mockapetris1987dnsconcepts], [mockapetris1987dnsimplementation], [projectnanda2026], [raskar2025beyonddns] provide operational resolution and verified agent facts. The passport's `ans_name` is *local metadata*; sovereign DNS/ANS discovery is proposed future work, not implemented here.
- **MCP and A2A** [mcp_spec_2025], [a2a_spec] standardize interaction surfaces. Passport metadata can bind to MCP/A2A boundary declarations, but this paper implements no live interoperability.
- **OPA / policy-as-code** [opa_docs] is an external decision engine decoupling policy decision from enforcement. ArgorixLang instead *compiles* policy metadata and passport bindings together within the program.
- **in-toto / provenance** [torresarias2019intoto] provides signed, farm-to-table supply-chain guarantees. ArgorixLang EvidenceBundles are *local, unsigned, semantic-digest* bundles unless extended with signatures and transparency.
- **WASI / capability systems** [wasi_intro] provide host-level, capability-based isolation. The passport model makes no host-isolation claim; runtime denials are VM-level control-flow properties, not OS containment.
- **Agent languages (Jason / AgentSpeak)** [bordini2007jason] and AI risk-management guidance [tabassi2023airmf], and LLM-application security guidance [owasp_llm_top10], provide complementary framing for agent programming and governance risk.

*Verify citation metadata before submission.* No `[X]` placeholder is required here because all comparisons reuse keys present in the source preprint's bibliography; confirm DOIs/URLs and add any venue-specific citations the target conference requires.

---

## IX. Limitations

We state limitations strictly, to prevent overclaiming.

- **No external identity verification.** Identity/credential references are declarative; no party is authenticated.
- **No DID/VC resolver.** No DID is resolved and no credential is verified.
- **No cryptographic proof of control.** No handshake, challenge–response, or key-control proof is executed.
- **No legal-compliance certification.** Declared country/jurisdiction/residency are not certified against any legal regime.
- **No physical residency enforcement.** A residency declaration of `CL` or `EU` does not prove where bytes were stored.
- **No production-isolation proof.** Runtime denials are VM-level control-flow properties; OS/process/network/secret-store containment is untested.
- **No controlled adversarial experiment.** The corpus contains no prompt text (`injected.content` absent in all 27 complete traces) and no controlled attacks; no injection success rate is reported.
- **No live provider federation.** Only a `simulated` provider is executable in the studied configuration; external execution is recorded as blocked.
- **Source integrity outside bundle verification.** The verifier starts at bytecode; it does not assert that `session.argx` produced the bytecode. Bundles are unsigned.
- **Low behavioral diversity.** Complete sessions are structurally uniform, limiting generalization across prompts, jurisdictions, and policy sets.

---

## X. Future Work

- **DID/VC resolver integration** to turn declarative identity/credential references into verifiable claims (introducing new trust anchors, replay, revocation, privacy, and network-failure concerns).
- **Signed passport attestations** and **signed EvidenceBundles** to add producer authentication and non-repudiation beyond content-change detection.
- **Source digest inclusion** so bundle verification can extend to `session.argx`.
- **Transparency log** for independent witnessing of bundles.
- **Operational ANS / Sovereign DNS prototype** to give `ans_name` real resolution and discovery semantics.
- **Policy decision canonicalization** computing one canonical tri-state decision from typed component results, eliminating coarse/detailed status confusion.
- **Controlled adversarial evaluation**, including prompt-injection studies once prompt content is available under an allowlist.
- **Host-level sandbox measurements** for process/filesystem/network/secret isolation.
- **Cross-provider runtime tests** with live, executable adapters across jurisdictions.

---

## XI. Conclusion

The Agent Passport provides a *compiled, evidence-visible metadata boundary* for governed AI agents. It represents sovereign metadata—country, jurisdiction, permitted residency, capabilities, policy bindings, and a local `ans_name`—as a typed, semantically validated declaration; it lowers validated metadata into IR and bytecode; and it preserves that metadata into traces, security reports, and offline-verifiable evidence bundles. An observational study of an existing corpus shows that passport metadata survives compilation and appears in 27/27 complete, offline-consistent sessions, each carrying two passports with declared Chilean country/jurisdiction and `CL`/`EU` residency. The construct's value is not external truth. It is **explicit representation, semantic validation, runtime visibility, and claim-bounded auditability**: a runtime consumer can inspect which sovereign metadata and policy bindings accompanied an operation, while the paper is explicit that these remain local declarations rather than verified identity, certified compliance, or physical-residency proof.

---

## Tables (IEEE-friendly summary)

**Table II.** Claim-boundary taxonomy (sovereign-metadata view).

| Concept | Implemented | Declarative | Proposed | Not claimed |
|---|---|---|---|---|
| Local naming | `ans_name` binding/metadata, preserved to artifacts | sovereign naming metadata | operational ANS / Sovereign DNS | operational DNS/registry resolution |
| Identity / credentials | reference validation (no dangling) | identity/credential references, ATrust maps | DID/VC resolver, signed attestations | external identity authentication, credential verification |
| Jurisdiction / residency | preservation into evidence; residency membership check | country/jurisdiction/residency declarations | — | legal-compliance certification, physical residency proof |
| Policy binding | reference resolution; binding into runtime profile | policy metadata associations | policy decision canonicalization | binding-as-approval |
| Evidence | semantic-digest bundle, offline verification | trace/report metadata fields | signed bundles, transparency log | producer authentication, non-repudiation, immutability |
| Provider boundary | fail-closed VM path, simulated-only allowlist | external-provider contract metadata | live federation | external execution, production isolation |

*(Table I appears in §III; Table III in §VI; Table IV in §VII.)*

---

## Figure captions and diagram descriptions

- **Figure 1 — Agent Passport binding model.** Directed graph: `Agent → Passport → Policy bindings → Runtime profile → {Trace, SecurityReport, EvidenceBundle}`. The Passport node lists its fields (country, jurisdiction, permitted residency, capabilities, `ans_name`). Edge labels read "validated reference / preserved metadata." A side legend distinguishes "validated in-program reference" from "verified real-world relationship (not claimed)."
- **Figure 2 — Boundary between local metadata and external verification.** A vertical divider splits the plane. Left ("Implemented / local"): `ans_name`, passport metadata, policy bindings. Right ("Proposed / not implemented"): DID/VC resolver, operational Sovereign DNS/ANS, remote discovery, live federation, signed attestations. Items crossing the divider are drawn as dashed, unbuilt edges.
- **Figure 3 — Evidence path.** Left-to-right pipeline `Source → Compiler → IR/Bytecode → VM → Trace → SecurityReport → EvidenceBundle`, with the `Source → Compiler` edge dashed and labeled "outside bundle verification," and a shaded region around Bytecode/Trace/Report labeled "offline-verifiable (semantic digests + ledger digest)." Passport metadata is annotated flowing from Source through Bytecode into Trace and SecurityReport.

---

## XII. Differences from the original ArgorixLang preprint

This paper is a *derived, narrowed* work. The original preprint, *"ArgorixLang: Compiled Governance, Sovereign Agent Metadata, and Offline-Verifiable Runtime Evidence,"* covers the full language and compiler architecture, the fail-closed runtime and provider boundary, the ATrust/DCP-AI conceptual framing, the complete trace/report/evidence model, and a five-research-question observational study spanning governance, runtime constraint, evidence verification, and roadmap. **This paper** isolates one construct—the **Agent Passport** and its sovereign-metadata evidence path—and develops its representation, semantic validation rules, compilation/lowering behavior, runtime evidence surfacing, claim-boundary taxonomy, and the subset of corpus observations that bear specifically on passports (passport counts, country/jurisdiction/residency declarations, offline bundle consistency, and prompt-content availability). All empirical values are reused unchanged from the source preprint; no new experiment, benchmark, deployment, or security guarantee is introduced. Where the original paper treats passports as one of many construct families, this paper treats the passport as the unit of analysis and foregrounds the *naming-vs-identity-vs-credential-vs-policy* distinction and the implemented/declarative/proposed/not-claimed boundary as its organizing contributions.
