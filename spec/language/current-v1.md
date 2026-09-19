# ArgorixLang 1.0 — semántica normativa de compatibilidad observada

Estado: contrato normativo de compatibilidad MAT-003 para preservar el lenguaje existente al migrar. Base: `5d73d663bf5b85fd71bdd0cbc1ac1636959d7e64`. No declara implementado Argorix Core ni certifica seguridad. `DEBE`, `NO DEBE` y `PUEDE` rigen el comportamiento objetivo de compatibilidad; la [matriz de cobertura](coverage.json) vincula los 40 campos de `Program` con las cláusulas L-01–L-08. Toda divergencia observada entre este contrato y stage0 se registra como discrepancia, no se reescribe la cláusula para conseguir un pase.

Fuentes de evidencia actuales: `crates/argorix_parser/src/{lexer,parser,ast}.rs`, `crates/argorix_semantics/src/checker.rs`, `crates/argorix_ir/src/ir.rs`, `crates/argorix_bytecode/src/bytecode.rs`, `crates/argorix_vm/src/{vm,evidence}.rs`, suites bajo `conformance/` y [baseline ESP-002](../../bootstrap/baseline.json). Las reglas para Core propuesto están en [core-draft.md](core-draft.md) y **no** son sintaxis aceptada por 1.0.

## L-01 — Entrada léxica, módulo, importación y paquete

1. Una fuente `.argx` DEBE ser UTF-8 válido y comenzar con `module` seguido de un nombre de módulo. Un módulo o import válido consiste en segmentos ASCII `[A-Za-z_][A-Za-z0-9_]*` separados por un único punto; segmentos vacíos, alias de import y rutas relativas son inválidos en la compatibilidad v1.0. `import agents.research` nombra un módulo local, no una URL ni un permiso de red.
2. El lexer reconoce identificadores, literales de cadena entre comillas dobles sin secuencias de escape, literales enteros decimales sin signo en rango `u64`, `{ } ( ) [ ] , : ->`, espacios y comentarios `//` hasta fin de línea. Un carácter no reconocido, cadena sin cerrar, salto de línea dentro de una cadena o entero fuera de `u64` DEBE producir error léxico; no se interpreta silenciosamente. Los identificadores léxicos pueden contener Unicode alfabético, números, `_` y `.`, pero los nombres de módulo/import se restringen además al patrón ASCII anterior. No existe literal flotante de expresión en esta versión.
3. Un archivo `argorix.toml` local declara `[package] name`, `[package] version` y `[entry] main` como cadenas entre comillas. Claves desconocidas o `entry.main` ausente/vacío DEBEN rechazarse. El resolvedor DEBE rechazar rutas absolutas y componentes `..`; la contención efectiva frente a symlinks/junctions sigue como obligación pendiente de host (TB-02), no garantía ya demostrada.
4. `module`, imports y declaraciones posteriores son sensibles a mayúsculas/minúsculas; el orden de declaraciones se conserva donde afecta trazas, bytecode o mensajes. Ningún `import` concede autoridad o capacidad por sí mismo.

Forma superficial (no EBNF exhaustiva de los bloques):

```ebnf
source       = "module", module_name, { top_level } ;
module_name  = segment, { ".", segment } ;
segment      = (ASCII_letter | "_"), { ASCII_letter | ASCII_digit | "_" } ;
top_level    = "import", module_name | declaration ;
declaration  = identifier, identifier, [ block ] ;
block        = "{", { token | block }, "}" ;
```

La última producción sólo delimita forma léxica: cada bloque DEBE cumplir la gramática de su cláusula, no cualquier secuencia de tokens. El [catálogo de construcciones](declarations.md), estas cláusulas y el manifiesto de casos normativos son el oráculo de compatibilidad; el parser stage0 es una implementación bajo prueba. Una combinación anidada no cubierta por ese contrato no puede promoverse a interfaz estable sin versión, caso normativo y guía de migración.

## L-02 — Tipos y enumeraciones de mensajes

`type Name { field: string ... }` declara una forma de mensaje; `type Name` declara un tipo sin campos. Los tipos primitivos de campo son `string`, `bool`, `int`, `float`; un nombre no primitivo sólo es válido si resuelve a tipo declarado. Nombres de campo duplicados y referencias desconocidas DEBEN rechazarse. `enum Name { A B }` declara variantes nominales sin payload en 1.0. Estos tipos describen contratos de mensajes, no memoria de propósito general, operadores o construcción arbitraria de valores. `float` es un tipo de campo, no una promesa de reglas de redondeo aritmético de Core.

El orden de campos/variantes se preserva en la representación emitida. Una implementación nueva NO DEBE reinterpretar `int` actual como un entero Core de ancho definido sin versión/migración; representación en wire, rangos de valor y compatibilidad de esquemas anidados deberán concretarse antes de R2 (MAT-003/MAT-004/MAT-015).

## L-03 — Agentes, mensajes, protocolos y capacidades

`agent` puede declarar `receives`, `sends`, `capabilities`, `tools`, `models`, `security { approval granted|denied }` y handlers `on Type as binding { ... }`. Sin bloque `security`, aprobación efectiva es `denied`. Un mensaje o destino no declarado DEBE fallar en validación semántica; una declaración de capability no equivale a concesión desde el host. Los seis tipos de instrucción de handler 1.0 son `emit Type to Target`, `trace binding`, `halt`, `name(binding)` (intrínseco conocido), `call Tool with binding` y `ask Model with binding`. Sus instrucciones se leen en orden fuente; no hay expresiones generales, ramas, bucles ni función de usuario.

`protocol` contiene pasos `From -> To: Act MessageType`; participantes, mensaje y acto DEBEN ser conocidos. Un nombre de herramienta/modelo/proveedor en el código no habilita un efecto externo. El runtime actual sólo ejecuta proveedor simulado y puede planificar un perfil externo sin despacharlo; una VM nueva NO DEBE convertir por accidente `planned`, `DENY`, `REVIEW` o `UNKNOWN` en efecto real. La semántica normativa de entrega concurrente, cancelación, delegación y revocación de R2 requiere MAT-005/006 y no se infiere de la lista de handlers existente.

## L-04 — Políticas, aserciones y fallo

`assert`, `policy` y `failure` son declaraciones separadas. Un `policy` nombra reglas, criterios de violación y acción (`block`, `review`, `warn` o la acción definida por la versión); nombres de regla desconocidos o duplicados DEBEN rechazarse o producir resultado no aprobatorio explícito, nunca `ALLOW` implícito. Un `failure` describe la respuesta declarada al fallo, no hace retroceder por sí mismo un efecto del host. El resultado global de policy y sus detalles DEBEN mantenerse consistentes: una violación `block` no puede coexistir con un veredicto aprobatorio sin error explícito. `review` no se transforma en permiso hasta una autorización separada con identidad y ámbito verificados.

Una evaluación de política basada sólo en traza interna NO prueba ausencia de efecto externo. La prueba de esa ausencia requiere sensor fuera de la VM y control positivo según [trust-boundaries.md](../trust-boundaries.md). Las reglas concretas de autorización por operación de R2 están por completar en MAT-006/013.

## L-05 — Proveedores, adaptadores y límites declarativos

`provider`, `harness`, `feature`, `secret`, `adapter`, `adapter_profile`, `runtime_hardening_profile`, `runtime_execution_profile` y `sandboxed_provider_adapter` describen contratos, flags, límites y perfiles. En v1.0 no crean un conector real ni leen una clave. `secret` guarda handle y frontera; un valor de secreto embebido en fuente DEBE rechazarse. Un proveedor `external` declarado no se vuelve ejecutable por `enabled`, por un harness, por una feature ni por una aprobación textual. `simulated` es el único proveedor de producto ejecutable observado; `eval-tripwire` es un build experimental distinto.

`sandboxed_external` de la VM actual devuelve estado `planned` y `external_execution_enabled: false` cuando se acepta el plan; ese estado NO DEBE comunicarse como efecto realizado. Un futuro perfil ejecutable sólo puede activarse después de cumplir TB-04/TB-05, autorización final por operación y aislamiento host probado en cada plataforma. Ausencia de enforcement de host → capacidad deshabilitada.

## L-06 — Identidad, criptografía y trust metadata

`crypto`, `crypto_boundary`, `did_method`, `atrust_boundary`, `atrust_identity`, `atrust_credential_contract`, `atrust_handshake`, `trust_ledger` y `passport` son contratos o metadatos del programa 1.0. Su validez sintáctica/semántica no autentica a un emisor, no verifica DID/VC en red, no materializa un ledger confiable y no demuestra propiedades criptográficas de una primitiva nombrada. Campo `country`, `jurisdiction`, `risk_level`, `identity`, `post_quantum_ready` o `security_claims` NO DEBE usarse como autorización sin verificador, ancla y política externa explícitos.

Un handshake o firma sin ancla confiable, freshness y política de ciclo de claves no demuestra identidad de producción. La implementación efectiva queda para MAT-007/012/015. Inconsistencias en references/nombres se rechazan en check, no se corrigen adivinando la intención del emisor.

## L-07 — Contratos de puentes y evidencia

`mcp_bridge_contract`, `a2a_bridge_contract` y `atrust_evidence_map` son metadatos declarativos 1.0. Un campo `protocol mcp|a2a` no establece conformidad con una versión de esos protocolos; transporte `declared_only` no abre red. Un `atrust_evidence_map` organiza referencias, no sustituye verificación de bundle, firma, fuente y sensor. La cobertura ejecutable de MCP/A2A queda fuera de 1.0 y debe fijar versión/operaciones antes de R2 (MAT-014/015).

## L-08 — Gobernanza y release metadata

`governance_profile`, `regulatory_mapping`, `third_party_verifier`, `public_conformance_report`, `threat_model`, `spec_freeze` y `release_candidate` son declaraciones inspeccionables. No constituyen auditoría humana, cumplimiento legal, certificación independiente, release firmado ni aptitud de producción. Una nueva implementación DEBE preservar estos metadatos al serializar/emitir o rechazar el artefacto si no puede representarlos sin pérdida; no puede convertir una declaración en evidencia de que el control se ejecutó.

## Correspondencia de fases, formatos y fallos

`source UTF-8 → lexer/AST → chequeo semántico y resolución de módulos → IR → bytecode JSON → verificador → VM → traza/reporte/bundle`. Un fallo en cualquier fase impide atribuir éxito a fases posteriores. El bytecode admite versiones explícitas; `verify_bytecode` rechaza versión/opcode/estructura no admitidos. `source_digest` sólo vincula fuente individual cuando la CLI lo emite así; un paquete multiarchivo no tiene aún binding completo. Un digest sin ancla comprueba consistencia interna, no identidad de productor.

| Interfaz | Política de estabilidad desde MAT-003 | Estado actual |
| --- | --- | --- |
| Sintaxis `.argx` 1.0 y comportamiento de casos fijados | Compatibilidad de fuente para construcciones documentadas o migración versionada | Baseline normativo MAT-003; ampliaciones requieren caso y versión |
| `argorix.toml` local | Campos `package.name`, `package.version`, `entry.main` preservados; ampliaciones versionadas | Sólo manifest local mínimo |
| Bytecode `.argbc.json` con `bytecode_version` | Consumidor valida versión; no reinterpretación silenciosa ni downgrade | Esquema versionado pero no ABI estable prometido |
| IR JSON, AST interno y nombres de structs Rust | No API pública estable; no persistir como contrato de terceros sin decisión explícita | Interno |
| Trace, security report y EvidenceBundle | Esquema/versiones explícitos; verificabilidad según ancla y origen; extensiones no implican autenticidad | Parcial, fuente multiarchivo sin binding |
| Diagnósticos humanos | Clase/fase y rechazo observables; texto, ruta y posición exacta no son ABI | Texto inestable |
| MCP/A2A, DID/VC, políticas legales | Declaración sin interoperabilidad, identidad o cumplimiento automático | Fuera de soporte ejecutable 1.0 |

La salida JSON de bytecode/evidencia debe ser semánticamente verificable, pero no se promete igualdad de bytes por whitespace u orden de claves salvo canonicalización expresamente fijada por el verificador. Para cambios incompatibles rige [compatibility.md](../compatibility.md). Los casos de aceptación de esta versión están en `conformance/normative/`; ejemplos Core propuestos se marcan `PLANNED` y no se ejecutan contra stage0.
