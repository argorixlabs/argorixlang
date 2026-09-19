# ESP-001 — Inventario técnico y mapa de migración

Fecha de inspección: 2026-09-17. Commit de código: `5d73d663bf5b85fd71bdd0cbc1ac1636959d7e64`.
Rama: `feat/adversarial-evaluation-campaign`. Los planes/fichas nuevos estaban sin commit al comenzar. El código versionado del producto no se modificó.
Estado de implementación: toolchain Rust; migración a Argorix pendiente.

## Resultado

Hay **12 paquetes del workspace, cuatro binarios únicos y 49 archivos Rust de implementación**. Cargo.lock contiene **75 paquetes, 63 externos**; esos 63 incluyen variantes opcionales/de plataforma y no representan necesariamente todo lo enlazado en un binario concreto. Los manifiestos del workspace declaran **ocho dependencias externas directas**.

El lenguaje existente sirve para contratos y ejecución controlada de agentes, pero su superficie ejecutable no basta para escribir su propio compilador. Sus llamadas de handler son intrinsics, herramientas o modelos; no funciones de usuario generales. La migración debe añadir Core antes de portar el parser.

Entregables relacionados: [datos completos](dependencies.json), [colector reproducible](collect-inventory.ps1), [validación](validate-inventory.ps1), [ficha](../tasks/espada/ESP-001.md).

## Método y cobertura

1. `cargo metadata --offline --locked --format-version 1 --no-deps`: miembros, targets, features, dependencias directas y consumidores inversos.
2. Inventario de fuentes versionadas mediante `git ls-files`, hashes SHA-256 y declaraciones públicas léxicas con archivo/línea.
3. Lectura de entrypoints, exports, AST, lexer, bytecode, handlers, CLI, wrappers, scripts, workflows y lockfiles.
4. Lectura offline de imports PE de binarios locales, sin ejecutarlos ni reconstruirlos.
5. Parseo acotado de Cargo.lock para nombres, versiones, checksums y aristas registradas; no es resolución completa de features.

`dependencies.json` conserva **584 candidatos de declaraciones públicas**: estructuras, enums, exports y funciones, incluidos elementos cfg/test en src. No se afirma que todos sean API estable o alcanzable públicamente. Los exports de cada crate y APIs principales fueron revisados aparte. Campos multilinea/firmas completas se consultan en su fuente enlazada.

El grafo Cargo completo offline no estuvo disponible: falta `fiat-crypto v0.2.9` en cache. El inventario está cerrado al nivel de fuentes/lockfile; la selección exacta y procedencia de librerías enlazadas debe medirse al construir el baseline ESP-002 y auditar independencia ESP-024.

## Componentes y destino

| `argorixc` | CLI compilador, paquetes y gráficos | `Cli`, `Command`, `compile`, `run` | compiler/main.argx (ESP-014, ESP-018, ESP-022) |
| `argorix_bytecode` | Lowering y verificación de bytecode | `lower_ir`, `verify_bytecode`, `source_digest`, `BytecodeProgram`, `Instruction` | compiler/ir + runtime/bytecode (ESP-007, ESP-013, ESP-018, ESP-019) |
| `argorix_ir` | AST a IR serializable | `IrProgram::from`, `IrModule`, `IrModuleImport`, `resolved_provider` | compiler/ir (ESP-007, ESP-013, ESP-018) |
| `argorix_parser` | Lexer, AST, spans y parser de contratos | `lex`, `parse_source`, `is_valid_module_name`, `Program`, `Diagnostic` | compiler/lexer + compiler/parser + compiler/ast (ESP-006, ESP-010, ESP-011, ESP-018) |
| `argorix_module` | Manifest local, grafo de imports y unión de módulos | `parse_manifest`, `resolve_package`, `merge_package`, `check_package`, `package_ir` | compiler/modules + tools/packages (ESP-012, ESP-020, ESP-022) |
| `argorix_semantics` | Símbolos y validaciones de seguridad/semántica | `check_program`, `check_program_with_options`, `CheckOptions` | compiler/types + compiler/resolve + reglas de agentes (ESP-006, ESP-012, ESP-018) |
| `argorix_provider` | Contrato y registro; ejecución simulated; tripwire opt-in | `Provider`, `ProviderRegistry`, `AdapterContract`, `SimulatedProvider` | runtime/providers (ESP-019, ESP-021) |
| `argorix_vm` | VM, scheduler, políticas, reportes, evidencia y firma verificada | `Vm::run_dry`, `Vm::run_reactive_outcome`, `Vm::run_runtime_profile`, `verify_evidence_with_anchor`, `SecurityReport::from_outcome`, `ReactiveScheduler`, `Scheduler` | runtime/vm + runtime/evidence + runtime/scheduler (ESP-019, ESP-020) |
| `argorix-vm` | CLI de ejecución y verificación | `Cli`, `Command`, `RunArgs`, `run` | runtime/main.argx (ESP-019, ESP-020, ESP-022) |
| `argorix-conformance` | Runner y CLI de conformidad, validación y mutación de fixtures | `run_suite`, `validate_suite`, `resolve_fixture_path`, `apply_mutation` | tools/conformance (ESP-002, ESP-022, ESP-023) |
| `argorix-sign` | CLI separada de generación de claves y firma | `Command::Keygen`, `Command::Sign`, `keygen`, `sign` | tools/sign (ESP-020, ESP-022) |
| `argorix-lang` | Host de integración; reexporta bibliotecas para tests | `parser`, `semantics`, `ir`, `bytecode`, `vm` | tests/compatibility (ESP-002, ESP-023) |

La tabla es una asignación de migración, no una propuesta de conservar wrappers. Para cada componente, el JSON registra manifiesto, targets, dependencias, consumers, formatos, entrypoint y tareas destino. El paquete raíz `argorix-lang` es host de integración: no es un quinto compilador.

### Superficies de CLI que preservar

- `argorixc`: check, emit-ir, graph, capabilities, emit-bytecode, verify-bytecode, check-package, emit-ir-package, emit-bytecode-package, graph-package. `verify-bytecode` recibe fuente y compila/verifica: no confundirlo con cargar directamente JSON hostil en la VM. Flag legacy-capabilities modifica compatibilidad.
- `argorix-vm`: run y verify-evidence. Flags de dry-run/reactive/inject, reportes, trace, bundle, source, runtime, adapter, operation y sandboxed-external; trust-anchor al verificar.
- `argorix-conformance`: run sobre suite y workspace de casos; lógica de pipeline en bibliotecas, no solo shell a otros ejecutables.
- `argorix-sign`: keygen y sign, separado de la VM para mantener claves privadas fuera del runtime.

Los targets de tests no son herramientas de producto: preservar sus casos y portarlos a conformidad; no exigir mantener Cargo para ejecutarlos en la entrega nueva.

## Dependencias externas

| anyhow | 1.0.102 | CLI y errores | Sustituir por errores Result de Core y formateo de stdlib |
| clap | 4.6.1 | CLI | Implementar parser de argumentos en Argorix manteniendo comandos/códigos |
| serde | 1.0.228 | Serialización | Serializadores Argorix y esquemas versionados; no copiar macros |
| serde_json | 1.0.150 | Serialización | Codec JSON Argorix con canonicalización compatible |
| sha2 | 0.10.9 | Criptografía/digests | Primitiva no Rust revisada o binding acotado; preservar test vectors |
| ed25519-dalek | 2.2.0 | Criptografía/firma | Primitiva no Rust revisada, separada de política de claves |
| getrandom | 0.2.17, 0.3.4 | Sistema operativo/entropía | ABI host a entropía del SO con errores explícitos |
| thiserror | 1.0.69 | Errores/desarrollo proc-macro | Tipos/diagnósticos Argorix; macros Rust desaparecen |

Además de crates, el producto usa Rust std: colecciones y asignación, filesystem/rutas, IO y argumentos, formateo, errores y sincronización. Migración prevista: stdlib Argorix y ABI de host acotada. Los tipos Vec/String/BTreeMap de la implementación no son funciones ya disponibles para usuarios del lenguaje.

Cargo.lock registra entre otras transitivas macros/proc-macros, soporte de terminal, componentes de hash/firma y plataformas. Están listadas con sus aristas en JSON. No copiar la dependencia transitiva Rust para resolver el port de una dependencia directa: sustituir la capacidad necesaria y volver a auditar.

No recomendar una biblioteca criptográfica concreta sin revisar su plataforma, licencia y propiedades en la tarea correspondiente. La obligación es conservar semántica/canonicalización y vectores de verificación, no traducir algoritmos de memoria.

## Formatos y contratos observados

| Formato | Productor/consumidor actual | Obligación de migración |
| --- | --- | --- |
| .argx | parser/compiler | Semántica y diagnósticos; nueva sintaxis versionada |
| argorix.toml | module/manifest | Subconjunto TOML propio: nombre, versión, entry; paquetes locales sin registry/dependencias externas |
| IR JSON | IrProgram + serde | Versionado y orden/estructura documentados |
| .argbc.json | lower_ir + verify_bytecode + VM | Rechazo de versiones/opcodes/referencias inválidas; datos históricos |
| trace/security/evidence JSON | VM/report/evidence | Consistencia de digests, canonicalización y alcance de cada versión |
| firma detached .sig.json | signer / signature verifier | Firma sobre representación definida, ancla y errores |
| suite JSON y fixtures | conformance | Validación, mutación, resultado y materialización de artifacts |
| JSONL/CSV/LaTeX | harness/paper | Evidencia histórica y separación observación/oráculo |

La emisión individual asigna digest de fuente; el soporte de paquetes aún necesita vinculación de manifest y módulos completos. Evidencia firmada autentica respecto de un ancla suministrada, no gestiona por sí sola claves, revocación o timestamps confiables.

## Superficie real del lenguaje

**Campos de Program, inventariados exhaustivamente desde AST:** `module`, `imports`, `providers`, `harnesses`, `features`, `secrets`, `adapters`, `adapter_profiles`, `cryptos`, `crypto_boundaries`, `did_methods`, `atrust_boundaries`, `atrust_identities`, `atrust_credential_contracts`, `atrust_handshakes`, `trust_ledgers`, `mcp_bridge_contracts`, `a2a_bridge_contracts`, `atrust_evidence_maps`, `governance_profiles`, `regulatory_mappings`, `third_party_verifiers`, `public_conformance_reports`, `runtime_hardening_profiles`, `threat_models`, `spec_freezes`, `release_candidates`, `runtime_execution_profiles`, `sandboxed_provider_adapters`, `assertions`, `policies`, `failures`, `capabilities`, `enums`, `types`, `tools`, `models`, `agents`, `protocols`, `passports`.

Esto incluye identidad, gobernanza, mapeos regulatorios, contratos MCP/A2A y release metadata; su presencia sintáctica no implica ejecución de protocolos o identidad autenticada.

**Tipos de campo:** String, Bool, Int, Float y marcador Unknown. Son contratos de mensajes; no una biblioteca general de tipos/operadores computacionales.

**Lexer:** `Ident`, `StringLiteral`, `IntegerLiteral`, `LeftBrace`, `RightBrace`, `LeftParen`, `RightParen`, `LeftBracket`, `RightBracket`, `Comma`, `Colon`, `Arrow`, `Eof`.

**Handlers:** `Emit`, `Trace`, `Halt`, `IntrinsicCall`, `CallTool`, `AskModel`. Ver parser.rs:7053 y ast.rs:1569. La forma `nombre(binding)` representa IntrinsicCall; no declara una función de usuario. El scheduler usa efectos implementados en Rust y mantiene estado/checkpoints internos.

**Bytecode:** `DeclareProviderContract`, `DeclareAgent`, `DeclareCapability`, `DeclareProtocol`, `DeclareAssertion`, `DeclareFailure`, `VerifyAssertion`, `PolicyReport`, `DeclareTool`, `AuthorizeTool`, `DeclareModel`, `AuthorizeModel`, `DeclareHandler`, `EmitMessage`, `TraceValue`, `HandlerHalt`, `InvokeIntrinsic`, `CallTool`, `AskModel`, `EndHandler`, `SendMessage`, `RequireCapability`, `RequireApproval`, `Trace`, `Halt`, `End`, `Unknown`. Son 26 variantes reconocidas más Unknown; no hay instrucciones generales de suma, carga/almacenamiento, llamada de función, retorno o salto condicional.

### Carencias mínimas de Core

| Necesidad de self-hosting | Evidencia actual | Trabajo necesario |
| --- | --- | --- |
| Funciones, parámetros, retorno y recursión | Program no tiene declaraciones de función; handler limitado | ESP-004/006, MAT-003 |
| Expresiones y operadores generales | Lexer sin operadores aritméticos generales; bytecode sin ALU | ESP-004/007 |
| Condicionales y bucles de usuario | Handler enum no los contiene | ESP-004/006/007 |
| Colecciones, strings manipulables y árboles | Vec/String pertenecen a Rust host, no a stdlib Argorix | ESP-005/009 |
| Memoria, lifetime, handles y límites | No modelo de memoria Core ni instrucciones de acceso | MAT-004, ESP-005/008 |
| Tipos recursivos y variantes con payload | EnumDecl contiene nombres; campos primitivos de mensajes | ESP-004/012 |
| IO controlado para compilar archivos | Actualmente fs y paths en Rust | ABI de host ESP-005, stdlib ESP-009 |
| Errores generales y resultados | Diagnostic/VmError implementados en Rust | Core/stdlib y CLI ESP-004/009/014 |
| Backend nativo/ABI de programas Core | Emisión actual es IR/bytecode JSON | ESP-008 transitorio y ESP-016/017 nativo |
| Paquetes remotos/lockfile de Argorix | Manifest local mínimo | MAT-016, no confundir Cargo.lock con lockfile Argorix |

Las ausencias están sustentadas en la superficie actual AST/lexer/instrucciones; no son conclusiones sobre si Rust contiene esas operaciones internamente. No basta cambiar extensiones a .argx.

## Caminos de construcción y ejecución

| cargo-build | cargo build/test -> rustc + linker -> cuatro binarios y libs | core-required | [Fuente](../Cargo.toml) |
| compiler | CLI -> parser -> semantics/module -> IR -> bytecode -> stdout | core-required | [Fuente](../crates/argorixc/src/main.rs) |
| runtime | CLI -> bytecode verify -> VM/provider -> trace/report/evidence; verify-evidence -> signature | core-required | [Fuente](../crates/argorix-vm/src/main.rs) |
| conformance | suite JSON -> bibliotecas Rust in-process -> workdir/resultados | core-required | [Fuente](../crates/argorix_conformance/src/runner.rs) |
| signer | CLI -> OS randomness o seed explícito -> Ed25519 -> archivos de claves/firma | core-required | [Fuente](../crates/argorix-sign/src/main.rs) |
| demo | Next route -> execFileSync argorixc/argorix-vm -> plan -> fetch en TypeScript | optional-demo | [Fuente](../demo/argorix-chatbot-runtime/lib/argorix/runArgorix.ts) |
| evaluation | Python subprocess -> collector -> release CLIs + eval-tripwire + signer -> score/render | research | [Fuente](../evaluation/adversarial/run.py) |
| legacy-matrix | Python fixtures/oráculo interno -> tablas (no sustituye evidencia independiente) | research | [Fuente](../scripts/run_controlled_matrix.py) |
| paper | make -> PowerShell -> Python + cargo/VM para verificar -> Tectonic + Poppler -> PDF | research | [Fuente](../paper/Makefile) |
| ci | GitHub Actions -> toolchain Rust -> fmt/clippy/build/test/conformance | development | [Fuente](../.github/workflows/ci.yml) |
| security-ci | cargo-deny action -> revisión dependencias/licencias | development | [Fuente](../.github/workflows/security.yml) |
| paper-ci | GitHub Actions -> TeX Live/latexmk -> PDF (otra workflow para fail-closed) | research | [Fuente](../.github/workflows/build-agent-passport-paper.yml) |
| editor | Extensión declarativa: gramática/config referenciadas no presentes; no LSP | optional-tooling | [Fuente](../tools/argorix-vscode/package.json) |

### Dependencias indirectas que no se deben perder de vista

- La demo usa `execFileSync` con rutas por defecto a `target/debug/*.exe` y overrides ARGORIXC_BIN/ARGORIX_VM_BIN. Cambiar frontend web no elimina la dependencia Rust.
- La petición HTTP del modelo sale de `lib/openai/callOpenAI.ts`, invocada por route.ts después del recorrido de planificación. No es evidencia de proveedor HTTP dentro de la VM.
- El lockfile de la demo incluye 34 entradas nativas opcionales SWC/sharp/libvips. Son selección por plataforma, no 34 librerías instaladas. SWC y Tectonic son puntos de auditoría de procedencia adicional si se incluyen en el build de entrega; este inventario no inspecciona su código upstream ni certifica su composición.
- La investigación usa Python pero sus colecciones invocan binarios Rust, incluida una variante eval-tripwire. No confundir lenguaje del harness con implementación del producto.
- El build del paper usa PowerShell/Python/Tectonic/Poppler; la verificación histórica llama Cargo y VM. Puede conservarse como investigación fuera del build obligatorio, con límites documentados.
- VS Code tiene package.json declarativo, pero sus rutas `language-configuration.json` y `syntaxes/argorix.tmLanguage.json` no están presentes en el directorio inspeccionado. No hay servidor LSP implementado en esos archivos.

## Bibliotecas enlazadas observadas

Se leyeron cinco ejecutables locales existentes: cuatro de release y una VM eval-tripwire. Todos son PE x86-64 e importan directamente DLL de Windows/UCRT, VCRUNTIME140 y bcryptprimitives. La VM eval-tripwire también importa WS2_32; su fuente tripwire usa TcpStream.

Los nombres completos y hashes de cada binario están en `scan.existing_binaries`. No hay directorio de delay imports en estos cinco artefactos. **No encontrar una DLL Rust no prueba independencia:** los crates pueden estar enlazados estáticamente. No se ejecutaron esos binarios ni se afirmó que correspondan a HEAD. Tampoco la ausencia de WS2_32 en un listado basta para demostrar ausencia de toda red o carga dinámica.

## Reproducción y validación

Desde la raíz, con PowerShell 7, Git y Cargo instalados:

```powershell
./bootstrap/collect-inventory.ps1
./bootstrap/validate-inventory.ps1
```

El colector es de lectura: emite JSON a stdout, no construye, no ejecuta programas Argorix, no accede a claves ni descarga paquetes. La validación compara nombres/targets, hashes de fuentes/soporte, lockfile y superficie con la captura guardada. Si cambió código, exige refrescar y revisar el inventario; no marca aceptación por mero conteo.

No ejecutados aquí: build limpio, tests de lenguaje, campañas, auditoría upstream de dependencias nativas, grafo exacto por feature/target y CI remota. Son trabajos de baseline/independencia posteriores, no resultados atribuibles al inventario.

## Handoff

Próxima tarea recomendada: MAT-001 (contrato del producto), ya lista; ESP-002 (baseline) también depende solo de ESP-001. Antes de ejecutar baseline, resolver cache incompleta y seleccionar features/targets explícitos. Preservar la evidencia del paper y no usar el número de tests histórico como resultado actual.

La decisión de memoria y backend sigue pendiente de MAT-003/004 y ESP-003/005. Este inventario determina qué hay que migrar; no aprueba una arquitectura nueva ni elimina código Rust.

