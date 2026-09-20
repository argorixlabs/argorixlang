# Plan maestro de ArgorixLang: independencia, funcionalidad y producción

Fecha: 2026-09-19. Estado: EN EJECUCIÓN; ESP-001–008, MAT-001–007 y MAT-029 cerradas como inventario, baseline, arquitectura, especificación/prototipo Core, frontend stage0, IR verificado, ejecución C transitoria, contratos/modelos y gobernanza; self-hosting e independencia completa aún no están implementados.
Base de inspección: `5d73d66`, workspace `1.0.0`. Comprobar revisión, ramas y CI al comenzar.
Este es el punto de entrada para continuar el proyecto. Prevalece sobre el plan Espada y el backlog AL donde cambie prioridades, dependencias o criterios de cierre.

## 1. Resultado buscado

Entregar un lenguaje para comunicación y ejecución gobernada de agentes, con compilador escrito en ArgorixLang, autocompilación, runtime y herramientas independientes de Rust; utilizable por terceros y apto para operar los perfiles que se validen.

La independencia de Rust es una condición de arquitectura. La madurez requiere además semántica estable, aplicaciones reales, seguridad medida, recuperación, herramientas, soporte y validación externa. Un backend nativo propio se conserva como meta técnica explícita, pero no equivale a demostrar seguridad ni aptitud para producción.

Este plan amplía la planificación solicitada: no inicia una reescritura silenciosa ni afirma que completar documentos complete la implementación.

## 2. Cómo está dividido el trabajo

- **25 tareas de independencia ESP-001 a ESP-025**, detalladas en [Plan Espada](PLAN_ESPADA_INDEPENDIENTE.md).
- **30 tareas adicionales MAT-001 a MAT-030**, cada una con ficha individual, dependencias, pasos, entregables y pruebas.
- **55 tareas activas de primer nivel.** ESP-026 queda SUSTITUIDA: su mezcla de editor, puentes y agentes se reparte en MAT-013 a MAT-020, MAT-027 y las validaciones asociadas. No se cuenta como tarea 56.
- [Registro estructurado del backlog](tasks/madurez/BACKLOG.json): estado MAT y dependencias adicionales sobre ESP.
- Primeros cierres: [ESP-001](tasks/espada/ESP-001.md) como inventario, [ESP-002](tasks/espada/ESP-002.md) como [baseline histórico](bootstrap/baseline.json), [ESP-003](tasks/espada/ESP-003.md) como [arquitectura de independencia](bootstrap/architecture.md), [ESP-004](tasks/espada/ESP-004.md) como [especificación Core](spec/core/README.md), [ESP-005](tasks/espada/ESP-005.md) como [memoria](spec/core/memory.md)/[ABI host](spec/host-abi.md), [ESP-006](tasks/espada/ESP-006.md) como frontend Core Rust stage0, [ESP-007](tasks/espada/ESP-007.md) como IR Core verificado, [ESP-008](tasks/espada/ESP-008.md) como backend C transitorio/runtime C1 ejecutable, [MAT-001](tasks/madurez/MAT-001.md) como [contrato de producto](spec/product-contract.md), [MAT-002](tasks/madurez/MAT-002.md) como [arquitectura de confianza](spec/trust-boundaries.md), [MAT-003](tasks/madurez/MAT-003.md) como [contrato normativo](spec/language/current-v1.md), [MAT-004](tasks/madurez/MAT-004.md) como [memoria abstracta](spec/memory-model.md)/[efectos](spec/effect-system.md), [MAT-005](tasks/madurez/MAT-005.md) como [concurrencia](spec/concurrency.md)/[entrega](spec/message-delivery.md), [MAT-006](tasks/madurez/MAT-006.md) como [capacidades A1](spec/capabilities.md), [MAT-007](tasks/madurez/MAT-007.md) como identidad, secretos y procedencia y [MAT-029](tasks/madurez/MAT-029.md) como [gobernanza](GOVERNANCE.md)/[mantenimiento](operations/maintenance.md). Las otras 39 tareas siguen pendientes.
- El plan AL anterior conserva contexto, pero no determina el orden de ejecución.

Las tareas MAT no son una lista para ejecutar únicamente después de Espada. Sus contratos iniciales deben definir el producto y los invariantes antes de la migración. Las implementaciones esperan al compilador/runtime necesarios para evitar duplicar código que luego deba portarse.

## 3. Tres entregas distintas

| Entrega | Qué demuestra | Lo que no permite afirmar |
| --- | --- | --- |
| R1 — Independiente | ESP-001–025 y prerrequisitos MAT aplicables; bootstrap nativo, VM y herramientas sin Rust | No implica producto completo ni aptitud de producción |
| R2 — Funcional completa | Aplicaciones obligatorias, agentes autenticados, ejecución gobernada, recuperación y herramientas; aceptación MAT-027 | No sustituye pruebas prolongadas, seguridad independiente ni beta externa |
| R3 — Producción acotada | MAT-030 con dependencias transitivas cerradas y evidencia del candidato distribuido | No promete ausencia universal de bugs ni seguridad fuera del perfil probado |

Una release R1 puede publicarse como tecnológica/experimental. No renombrarla como R3 porque se haya completado self-hosting. Si cambia el alcance obligatorio, registrar la decisión y qué requisito se retira: no quitarlo para conseguir un cierre aparente.

## 4. Contrato de producto a fijar primero

MAT-001 debe convertir estas propuestas en requisitos identificables:

1. Crear, compilar, ejecutar, depurar y verificar un proyecto desde una instalación limpia.
2. Ejecutar un agente con modelo y herramienta autorizada; impedir y registrar una acción no autorizada.
3. Comunicar dos agentes en procesos separados con identidad autenticada, tipos, delegación y revocación.
4. Recuperar un flujo persistente tras caída, distinguiendo efectos completados, no realizados e inciertos.
5. Actualizar y restaurar una versión soportada sin corrupción ni pérdida silenciosa de estado.
6. Recompilar compilador, VM y herramientas desde Argorix en un entorno sin Rust, usando dependencias explícitas.
7. Permitir que terceros reproduzcan las aplicaciones con documentación pública y artefactos de release.

Perfil inicial propuesto: Linux x86-64 y Windows x86-64; un mecanismo real de identidad con anclas explícitas; una superficie MCP y una comunicación A2A con versiones/cobertura fijadas al implementar. No es requisito construir blockchain, identidad universal, todas las variantes de protocolo o un registro público mundial de paquetes.

DID/VC, atributos de jurisdicción y contratos regulatorios deben indicar si son declarados o verificados. Ni firma ni metadatos prueban cumplimiento legal. Requisitos de identidad obligatorios deben tener autenticación efectiva aunque se elija un mecanismo distinto de DID.

## 5. Dependencias cruzadas obligatorias

Además de las dependencias originales ESP y las fichas MAT, aplicar estas:

| Tarea ESP | Prerrequisito adicional | Propósito |
| --- | --- | --- |
| ESP-003 | MAT-002 | La arquitectura de bootstrap respeta el modelo de confianza |
| ESP-004 | MAT-003 | Core forma parte de una especificación coherente |
| ESP-005 | MAT-004 | Memoria y efectos tienen invariantes explícitos |
| ESP-019 | MAT-005, MAT-006 | El runtime migrado respeta concurrencia y autoridad |
| ESP-020 | MAT-007 | Evidencia e identidad comparten una política de claves |
| ESP-021 | MAT-006 | El adaptador aplica delegación y revocación definidas |

Ruta inicial viable: ESP-001 y MAT-001 → ESP-002/MAT-002/MAT-029 → MAT-003 y ESP-003 → MAT-004/005/006 → MAT-007 y ESP-004/005 → resto del bootstrap según dependencias. Esta notación indica opciones listas, no exige paralelismo ni crear agentes.

No existe autorización implícita para delegar por aparecer tareas independientes. El ejecutor sigue las instrucciones de colaboración vigentes.

## 6. Backlog de madurez

Cada enlace abre una ficha ejecutable completa. MAT-001 a MAT-007 están HECHAS como contratos/modelos de definición y MAT-029 como política de gobernanza con simulacros; las otras MAT siguen PENDIENTES. El progreso se basa en aceptación demostrada, no en número de archivos generados.

| Tarea | Resultado | Prioridad | Dependencias | Estado |
| --- | --- | --- | --- | --- |
| [MAT-001](tasks/madurez/MAT-001.md) | Definir el producto final y sus contratos | P0 | — | HECHA |
| [MAT-002](tasks/madurez/MAT-002.md) | Arquitectura de confianza y catálogo de garantías | P0 | MAT-001, ESP-001 | HECHA |
| [MAT-003](tasks/madurez/MAT-003.md) | Especificación normativa completa y compatibilidad | P0 | MAT-002, ESP-002 | HECHA |
| [MAT-004](tasks/madurez/MAT-004.md) | Modelo de memoria, tipos y efectos con invariantes | P0 | MAT-003 | HECHA |
| [MAT-005](tasks/madurez/MAT-005.md) | Semántica de concurrencia y entrega de mensajes | P0 | MAT-003 | HECHA |
| [MAT-006](tasks/madurez/MAT-006.md) | Capacidades, delegación y revocación | P0 | MAT-002 | HECHA |
| [MAT-007](tasks/madurez/MAT-007.md) | Política de identidad, secretos y procedencia | P0 | MAT-006 | HECHA |
| [MAT-008](tasks/madurez/MAT-008.md) | Validación del compilador y backend nativo | P1 | ESP-017, MAT-004 | PENDIENTE |
| [MAT-009](tasks/madurez/MAT-009.md) | Scheduler concurrente y cuotas implementados | P1 | ESP-019, MAT-005, MAT-006 | PENDIENTE |
| [MAT-010](tasks/madurez/MAT-010.md) | Persistencia, checkpoints y recuperación | P1 | MAT-009 | PENDIENTE |
| [MAT-011](tasks/madurez/MAT-011.md) | Aislamiento y límites efectivos del host | P1 | ESP-021, MAT-006, MAT-009 | PENDIENTE |
| [MAT-012](tasks/madurez/MAT-012.md) | Identidad autenticada y ciclo de claves implementados | P1 | ESP-020, MAT-007 | PENDIENTE |
| [MAT-013](tasks/madurez/MAT-013.md) | Ciclo real de agentes con mediación completa | P1 | ESP-021, MAT-009, MAT-011, MAT-012 | PENDIENTE |
| [MAT-014](tasks/madurez/MAT-014.md) | Puente MCP ejecutable y verificable | P1 | MAT-013 | PENDIENTE |
| [MAT-015](tasks/madurez/MAT-015.md) | Comunicación autenticada entre agentes | P1 | MAT-013, MAT-010 | PENDIENTE |
| [MAT-016](tasks/madurez/MAT-016.md) | Dependencias reproducibles y paquetes confiables | P1 | ESP-022, MAT-007 | PENDIENTE |
| [MAT-017](tasks/madurez/MAT-017.md) | Biblioteca estándar de producción | P1 | ESP-022, MAT-004 | PENDIENTE |
| [MAT-018](tasks/madurez/MAT-018.md) | Depuración, trazas y profiling | P1 | ESP-022, MAT-009 | PENDIENTE |
| [MAT-019](tasks/madurez/MAT-019.md) | Editor y LSP coherentes con compilador | P1 | ESP-022, MAT-003 | PENDIENTE |
| [MAT-020](tasks/madurez/MAT-020.md) | Documentación y experiencia de aprendizaje | P1 | MAT-018, MAT-019, MAT-017 | PENDIENTE |
| [MAT-021](tasks/madurez/MAT-021.md) | Observabilidad y operación segura | P1 | MAT-009, MAT-010, MAT-011 | PENDIENTE |
| [MAT-022](tasks/madurez/MAT-022.md) | Actualizaciones y migración de estado | P1 | MAT-010, MAT-016 | PENDIENTE |
| [MAT-023](tasks/madurez/MAT-023.md) | Cadena de suministro y releases auditables | P1 | ESP-025, MAT-007, MAT-016 | PENDIENTE |
| [MAT-024](tasks/madurez/MAT-024.md) | Presupuestos de rendimiento y capacidad | P1 | MAT-008, MAT-010, MAT-013, MAT-021 | PENDIENTE |
| [MAT-025](tasks/madurez/MAT-025.md) | Pruebas prolongadas, caos y recuperación operativa | P1 | MAT-010, MAT-011, MAT-022, MAT-024 | PENDIENTE |
| [MAT-026](tasks/madurez/MAT-026.md) | Validación de seguridad independiente | P1 | MAT-008, MAT-011, MAT-012, MAT-014, MAT-015, MAT-016, MAT-023 | PENDIENTE |
| [MAT-027](tasks/madurez/MAT-027.md) | Aplicaciones completas de referencia | P1 | MAT-014, MAT-015, MAT-017, MAT-020, MAT-021, MAT-022 | PENDIENTE |
| [MAT-028](tasks/madurez/MAT-028.md) | Beta externa y usabilidad reproducible | P1 | MAT-027, MAT-024, MAT-025, MAT-026 | PENDIENTE |
| [MAT-029](tasks/madurez/MAT-029.md) | Gobernanza, soporte y mantenimiento del lenguaje | P0 | MAT-001, ESP-001 | HECHA |
| [MAT-030](tasks/madurez/MAT-030.md) | Release de producción y aceptación final | P1 | MAT-028, MAT-029, MAT-023, ESP-025 | PENDIENTE |

## 7. Puertas de aceptación

### G0 — Producto y semántica definidos

Exigir MAT-001 a MAT-007, inventario ESP-001 y baseline ESP-002. Entregables: requisitos, perfiles, amenazas, especificación, invariantes de memoria, concurrencia y autoridad. Los contratos se refinan con evidencia, pero no cambian silenciosamente.

La implementación del bootstrap puede avanzar cuando sus dependencias concretas estén cerradas; no se exige terminar todo G0 para cualquier tarea independiente.

### G1 — Independencia reproducible

Exigir R1 y reporte de ESP-024/025. Recompilación nativa en ambos destinos, sin Rust como compilador, biblioteca o servicio; lista de shims/dependencias permitidas y origen de semilla. Conservar referencia histórica de Rust como recurso de arqueología y comparación, fuera del build normal.

### G2 — Funcionalidad completa para el perfil

Exigir MAT-008 a MAT-022 y MAT-027, con aplicaciones reales y trazabilidad requisito → prueba. Los estados de delegación, revocación, cancelación, persistencia y fallo deben observarse en escenarios de extremo a extremo.

No sustituir invocaciones reales por un plan ni un agente remoto por un diccionario local. Proveedores de prueba deterministas se usan en CI, identificados como tales; las pruebas de integración reales tienen evidencia separada.

### G3 — Resiliencia y rendimiento

Exigir MAT-024/025. Establecer antes de ejecutar capacidad, p95/p99 pertinentes, memoria, tiempo de recuperación (RTO), pérdida máxima admisible (RPO), tamaños y concurrencia soportados.

Pisos propuestos para ratificar en MAT-001/024:

- Una campaña de 72 horas por perfil de plataforma obligatorio, con carga representativa y fallos inyectados.
- Cero efectos prohibidos observados y cero pérdidas silenciosas en escenarios obligatorios; sensores positivos demuestran capacidad de detección.
- Repeticiones, dispersión y ambiente de benchmarks registrados; ningún número inventado para llenar un dashboard.
- Recuperación de backups y upgrades probada, no solo creación del backup.
- Umbrales específicos de rendimiento fijados desde los casos de uso y baseline, antes de evaluar el candidato; no se eligen después para hacerlo pasar.

Cumplir 72 horas no garantiza fiabilidad ilimitada. El informe debe delimitar escenarios, carga y cobertura.

### G4 — Seguridad, supply chain y validación externa

Exigir MAT-023/026/028/029. Propuesta mínima: revisión de seguridad independiente y beta con al menos dos desarrolladores externos en dos proyectos distintos. Si no hay personas disponibles, el trabajo correspondiente sigue pendiente/bloqueado; no reemplazarlo por autodeclaración o una simulación de otro modelo.

No deben quedar hallazgos críticos o altos abiertos según clasificación acordada. Los menores necesitan tratamiento, responsable y plazo. No usar aceptación de riesgo para mantener un incumplimiento de una garantía obligatoria sin cambiar de forma explícita el alcance soportado.

Una evaluación externa puede requerir coordinación y presupuesto: preparar paquete reproducible y preguntas antes de pedir esa participación. Crear el plan no autoriza contratar, pagar ni contactar a terceros.

### G5 — Release de producción

Exigir MAT-030 y todas sus dependencias transitivas, además de matriz de requisitos sin huecos. Probar los binarios distribuidos, no únicamente el checkout de desarrollo. Documentar soporte, plataformas, riesgos residuales, migración y rollback.

Cambios posteriores a la campaña invalidan la evidencia afectada hasta revalidar; no reutilizar automáticamente aprobación de un commit anterior.

## 8. Trazabilidad y evidencia mínima

Cada requisito obligatorio debe registrar:

```text
requirement_id
capability / supported_profile
spec_clause
implementation_modules
task_ids
test_ids
candidate_commit
artifact_hashes
observed_result
limitations
owner
```

Cada ficha de tarea contiene criterios AC y registro de ejecución. Para marcar HECHA:

1. Todas las dependencias satisfechas, incluidas las cruzadas.
2. Cada criterio con resultado y prueba reproducible.
3. Compatibilidad, independencia y seguridad conservadas.
4. Documentación describe el comportamiento observado.
5. Evidencia sin secretos y vinculada al candidato.
6. Handoff y backlog sincronizados.

Estados permitidos: PENDIENTE, EN_CURSO, BLOQUEADA y HECHA. ESP-026 usa SUSTITUIDA como estado de planificación, no de ejecución. Un bloqueo enumera la condición faltante y qué trabajo independiente sigue disponible. No cerrar por falta de tiempo ni inventar resultados.

No sumar porcentajes de áreas inconmensurables para afirmar «95 % seguro». Mostrar tareas cerradas, garantías demostradas y brechas por separado.

## 9. Reglas de implementación

- Conservar Rust como referencia transitoria hasta pasar independencia; no borrar antes de las pruebas de reemplazo.
- Mantener la lógica nueva del producto en Argorix cuando exista bootstrap. FFI y librerías no Rust tienen alcance y procedencia explícitos.
- No implementar primitivas criptográficas propias para eliminar una dependencia. Elegir componentes apropiados y verificar sus propiedades/versiones durante la implementación.
- Especificar resultados de errores y efectos inciertos: no convertir ausencia de respuesta en ausencia de ejecución.
- Evitar reescribir el baseline del paper. Evaluar la toolchain nueva en resultados separados con revisión propia.
- Mantener casos de éxito legítimo además de denegaciones: un runtime que bloquea todo no satisface el producto.
- Los oráculos no pueden derivar resultados esperados de la implementación bajo prueba. Diferencial contra Rust es útil, pero insuficiente por sí solo.
- Toda feature nueva entra al gate que prueba ausencia de dependencias Rust, también transitivas.
- Probar documentación y herramientas como parte del producto.
- Verificar estándares/protocolos actuales al implementarlos con fuentes oficiales; este plan no congela por memoria una versión externa.

## 10. Qué iniciar y cuándo estimar

Primer ciclo de trabajo:

1. ESP-001: inventario técnico y de dependencias.
2. MAT-001: contrato del producto y casos obligatorios.
3. ESP-002: baseline reproducible.
4. MAT-002 y MAT-029: garantías y mantenimiento.
5. MAT-003: especificación y compatibilidad.
6. MAT-004/005/006/007: contratos de memoria, concurrencia y confianza antes del runtime nuevo.

Al cerrar ese ciclo, estimar el bootstrap usando el inventario y un prototipo de memoria. Después estimar por puertas G1–G5, con responsables, infraestructura y costo de validación externa. No hay una fecha defendible de producción todavía. Se trata de un programa de varios ciclos, no una edición breve de código.

Las fichas son suficientemente concretas para iniciar inspección, pero las tareas amplias pueden dividirse en subtareas (por ejemplo MAT-010.1) antes de implementar. Preservar criterios del padre y actualizar la matriz de dependencias; subdividir no reduce su alcance de aceptación.

## 11. Fuente de verdad y conservación

- Este documento define prioridades, puertas y alcance global.
- Plan Espada define pasos ESP-001–025.
- Fichas `tasks/madurez/MAT-NNN.md` definen pasos y aceptación de cada MAT.
- `tasks/madurez/BACKLOG.json` registra estados MAT y dependencias adicionales ESP. Mantenerlo sincronizado con fichas e índice al cerrar tareas.
- Si difieren dos documentos, resolver la discrepancia explícitamente antes de trabajo dependiente.
- ESP-001–008, MAT-001–007 y MAT-029 están HECHAS como inventario, baseline, arquitectura, especificación/prototipo Core, frontend stage0, IR verificado, ejecución C transitoria, contratos, modelos reducidos y gobernanza; las otras 39 tareas están PENDIENTES. Core ya ejecuta un perfil representativo mediante C sin enlazar runtime Rust, pero el emisor sigue siendo Rust stage0 y esto no demuestra self-hosting, backend nativo ni independencia completa. Ninguno de los 29 requisitos funcionales se ha aceptado por estos cierres. El registro incluye `esp_progress` para cierres ESP. No hay release nueva ni auditoría de seguridad completada.

Contexto verificado: MAT-029 alineó `SECURITY.md` y `GOVERNANCE.md` con las releases publicadas (v1.0.1 es la única soportada) y registró que el tag v1.0.1 lleva `Cargo.toml` en 1.0.0; la próxima release debe corregirlo. No inferir una release pública del número de Cargo.

## 12. Prompt para el siguiente modelo ejecutor

> Continúa ArgorixLang desde PLAN_MAESTRO_ARGORIXLANG.md. El objetivo es R1 independiente de Rust, R2 funcional para agentes y R3 apto para producción dentro del alcance validado. Lee Plan Espada, backlog y ficha de la primera tarea lista. Aplica también las dependencias cruzadas. Comprueba archivos, comandos, revisión y cambios existentes antes de editar. Ejecuta una tarea hasta sus criterios verificables; divide subtareas si hace falta sin recortar requisitos. No confundas autocompilación con madurez ni planificación con ejecución. Conserva baseline del paper, cambios ajenos y referencia histórica Rust. Documenta pruebas reales y límites, actualiza la ficha y el registro, y deja la siguiente tarea lista. Las verificaciones externas pendientes se registran, nunca se simulan como realizadas.
