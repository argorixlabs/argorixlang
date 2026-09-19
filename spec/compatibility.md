# Política de compatibilidad de ArgorixLang

Estado: propuesta normativa MAT-003 para releases futuras. La implementación observada en `5d73d66` todavía no cumple todas estas puertas; el [baseline ESP-002](../bootstrap/baseline.json) congela su comportamiento histórico sin transformarlo en promesa de soporte indefinido. Las cláusulas de fuente 1.0 están en [language/current-v1.md](language/current-v1.md); Core nuevo permanece [DRAFT](language/core-draft.md).

## Ejes de versión independientes

| Eje | Identificador y regla de lectura | Garantía / límite |
| --- | --- | --- |
| Fuente `.argx` | Versión de lenguaje/release y perfil de fuente declarado en manifest o CLI; 1.0 histórico por defecto sólo durante transición explícita | No reinterpretar sintaxis vieja como Core. Migración con herramienta y casos de prueba. |
| `argorix.toml` | Versión de esquema a introducir antes de dependencias remotas | Hoy sólo package.name/version y entry.main locales; extensiones deben fallar de forma visible en parsers viejos. |
| Bytecode `.argbc.json` | `bytecode_version` obligatorio, validado antes de VM | `verify_bytecode` reconoce 0.3, 0.5–0.36 y 1.0 en el baseline; reconocer una versión no promete ejecutar cualquier programa de ella ni preservarla eternamente. |
| IR/AST | Identidad interna de compilador, no wire pública | Puede cambiar con el compilador; no distribuir como formato estable sin especificación, versionado y tests propios. |
| Trace/report/bundle | Campo de versión de cada esquema y algoritmo de digest/firma | Un consumidor debe verificar según versión y ancla; hash solo no autentica. Fuente multiarchivo aún no vinculada completamente. |
| API/ABI nativa y host | Triple de destino + ABI + versión de runtime/stdlib | Aún por fijar en ESP-005/016/017; objeto de Windows no equivale a Linux. |
| MCP/A2A, DID/VC y ledger | Versión concreta de protocolo/perfil y operaciones soportadas | Metadatos actuales no implican implementación ni compatibilidad universal. |

## Reglas de evolución

1. Cada propuesta declara ejes afectados, requisito MAT-001, cláusula, fixture positivo/negativo, impacto de fuente, bytecode, ABI, seguridad y rollback. Se registra antes de cambiar el parser/emisor. Un mismo nombre de versión no puede adquirir nuevas capacidades de efecto sin invalidar el baseline del candidato.
2. Cambio aditivo sin reinterpretar programas existentes puede entrar en release menor, con ejemplo y test de compatibilidad. Cambio de semántica, nombre, tipo, autorización, esquema obligatorio o representación pública exige versión incompatible y guía de migración; no se esconde bajo default o feature flag que conceda autoridad.
3. Una forma marcada `deprecated` sigue aceptándose durante **dos releases estables consecutivas** después del aviso, con diagnóstico y herramienta o receta de migración. MAT-029 fijará calendario de soporte. Una corrección urgente de seguridad puede acortar el período sólo con advisory, impacto, mitigación y reversibilidad documentados; no se presenta como compatibilidad completa.
4. Lectores nuevos pueden aceptar versiones antiguas expresamente enumeradas. Lectores viejos deben rechazar artefactos nuevos que no entiendan, especialmente campos de identidad, efectos, autorización y evidencia; nunca degradar a `allow`. El conjunto exacto de versiones aceptadas se publica por release y se prueba en ambos perfiles.
5. El corpus de compatibilidad conserva casos positivos, rechazos, fuente, paquete, bytecode y evidencia. La línea base Rust de ESP-002 es un oráculo de **regresión**; la especificación y las observaciones externas son oráculos distintos. Si ambos difieren, abrir discrepancia y decidir mediante cambio versionado, no reescribir silenciosamente el esperado.
6. `source → IR → bytecode → VM → evidencia`: cada paso registra versión de entrada/salida. No se acepta truncamiento de metadata de confianza; si el backend nuevo no puede representar un campo, falla o emite una advertencia incompatible que detiene publicación, no descarta el campo.

## Matriz de transición inicial

| Productor → consumidor | Estado esperado | Prueba mínima |
| --- | --- | --- |
| Fuente 1.0 → stage0 Rust fijado | Baseline histórico: suite v100 y versiones anteriores según `bootstrap/baseline.json` | Conformance 21 archivos, 19 aceptados, dos rechazos documentados |
| Fuente 1.0 → compilador nuevo | Obligatorio para los casos normativos compatibles; divergencias explícitas | MAT-003/ESP-023: corpus por cláusula y comparación cruzada |
| Fuente Core propuesta → stage0 Rust | **Rechazo esperado**, no defecto de stage0 | Fixtures `PLANNED` de Core, no contados como pases del producto |
| Fuente Core versionada → compilador self-hosted | Pendiente ESP-006–015 | Casos válidos/invalidos, bootstrap stage2/stage3, semántica y objetos |
| Bytecode viejo soportado → VM nueva | Sólo versiones incluidas en matriz publicada; no autorizar más efectos | Fixtures históricos y sensor externo si hay efecto |
| Bytecode/ABI nuevo → VM vieja | Rechazo explícito antes de ejecutar | Caso negativo con versión futura/field de efecto nuevo |
| Bundle sin ancla → verificador | Consistencia interna solamente | Alteración coordinada y ancla extranjera; nunca decir autenticidad |
| Paquete multiarchivo → evidencia de fuente completa | Pendiente, NO soportado como claim de binding integral | Hash del manifest, todos los módulos y orden reproducible |

## Guía obligatoria para un cambio incompatible

La guía por release incluye: versión origen/destino, ejemplos antes/después, transformación automática o manual, ambigüedades que exigen decisión humana, verificaciones post-migración, impacto de datos/keys/estado, backup y rollback, y advertencia sobre efectos externos inciertos. La herramienta de migración debe ser idempotente o detectar que ya se aplicó. Una migración que toca estado durable no se ejecuta automáticamente al abrir un proyecto. Versiones de protocolo MCP/A2A y de host ABI se tratan por separado de la versión de fuente.

Los umbrales de aceptación R1–R3 y el significado de «soportado» pertenecen a [product-contract.md](product-contract.md). Este documento define la política, no certifica la release actual.
