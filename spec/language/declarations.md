# Catálogo normativo de construcciones ArgorixLang 1.0

Este catálogo cierra el inventario superficial de `Program` en la base `5d73d663bf5b85fd71bdd0cbc1ac1636959d7e64`. La cláusula enlazada define el significado; esta tabla impide que una migración omita silenciosamente una familia porque estuviera implementada sólo como `struct` Rust. Los campos anidados continúan sometidos a las restricciones semánticas fijadas por los casos normativos y por su cláusula.

| Campo de `Program` | Forma fuente | Cláusula | Efecto autorizado en 1.0 |
| --- | --- | --- | --- |
| `module` | `module Name` | L-01 | Identidad local del módulo |
| `imports` | `import Name` | L-01 | Resolución local; nunca red o autoridad |
| `enums` | `enum Name { ... }` | L-02 | Contrato nominal |
| `types` | `type Name { ... }` | L-02 | Contrato de mensaje |
| `capabilities` | `capability Name { ... }` | L-03 | Registro declarativo |
| `tools` | `tool Name { ... }` | L-03 | Contrato; no invoca host por sí solo |
| `models` | `model Name { ... }` | L-03 | Contrato; no invoca proveedor por sí solo |
| `agents` | `agent Name { ... }` | L-03 | Actor de VM simulada |
| `protocols` | `protocol Name { ... }` | L-03 | Orden contractual de mensajes |
| `assertions` | `assert Name ...` | L-04 | Condición verificable por etapa compatible |
| `policies` | `policy Name { ... }` | L-04 | Decisión declarativa fail-closed |
| `failures` | `failure Name { ... }` | L-04 | Respuesta declarada; no rollback del host |
| `providers` | `provider Name { ... }` | L-05 | Contrato de proveedor |
| `harnesses` | `harness Name { ... }` | L-05 | Contención declarativa |
| `features` | `feature Name { ... }` | L-05 | Flag declarativo |
| `secrets` | `secret Name { ... }` | L-05 | Handle/frontera; nunca material secreto |
| `adapters` | `adapter Name { ... }` | L-05 | Contrato declarativo |
| `adapter_profiles` | `adapter_profile Name { ... }` | L-05 | Perfil declarativo |
| `runtime_hardening_profiles` | `runtime_hardening_profile Name { ... }` | L-05 | Objetivos de hardening |
| `runtime_execution_profiles` | `runtime_execution_profile Name { ... }` | L-05 | Plan de ejecución; no ejecución externa |
| `sandboxed_provider_adapters` | `sandboxed_provider_adapter Name { ... }` | L-05 | Plan fail-closed |
| `cryptos` | `crypto Name { ... }` | L-06 | Registro de primitiva nombrada |
| `crypto_boundaries` | `crypto_boundary Name { ... }` | L-06 | Frontera declarativa |
| `did_methods` | `did_method Name { ... }` | L-06 | Método declarado; no resolución de red |
| `atrust_boundaries` | `atrust_boundary Name { ... }` | L-06 | Frontera declarativa |
| `atrust_identities` | `atrust_identity Name { ... }` | L-06 | Identidad declarada; no autenticación |
| `atrust_credential_contracts` | `atrust_credential_contract Name { ... }` | L-06 | Contrato declarativo |
| `atrust_handshakes` | `atrust_handshake Name { ... }` | L-06 | Protocolo declarado; no sesión autenticada |
| `trust_ledgers` | `trust_ledger Name { ... }` | L-06 | Registro declarativo |
| `passports` | `passport Name { ... }` | L-06 | Metadatos soberanos declarados |
| `mcp_bridge_contracts` | `mcp_bridge_contract Name { ... }` | L-07 | Contrato, no interoperabilidad probada |
| `a2a_bridge_contracts` | `a2a_bridge_contract Name { ... }` | L-07 | Contrato, no interoperabilidad probada |
| `atrust_evidence_maps` | `atrust_evidence_map Name { ... }` | L-07 | Mapeo, no prueba autenticada |
| `governance_profiles` | `governance_profile Name { ... }` | L-08 | Metadatos de gobernanza |
| `regulatory_mappings` | `regulatory_mapping Name { ... }` | L-08 | Mapeo, no cumplimiento legal |
| `third_party_verifiers` | `third_party_verifier Name { ... }` | L-08 | Verificador declarado, no ejecutado |
| `public_conformance_reports` | `public_conformance_report Name { ... }` | L-08 | Reporte declarado, no certificación |
| `threat_models` | `threat_model Name { ... }` | L-08 | Modelo declarativo |
| `spec_freezes` | `spec_freeze Name { ... }` | L-08 | Freeze declarado, no release firmado |
| `release_candidates` | `release_candidate Name { ... }` | L-08 | Candidato declarado, no aptitud productiva |

Conjuntos cerrados adicionales: 13 clases léxicas, 6 instrucciones de handler y 27 variantes de instrucción bytecode (incluida la variante de rechazo `Unknown`) están enumerados de forma legible por máquina en [coverage.json](coverage.json). Todo lo que Argorix Core necesita pero no existe en v1 aparece en `excluded_from_v1` con tarea propietaria; una implementación no puede inventarlo como extensión compatible.
