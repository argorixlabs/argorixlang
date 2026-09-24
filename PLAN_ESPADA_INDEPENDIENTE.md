# Proyecto Espada: ArgorixLang independiente de Rust

> Plan rector: [Plan maestro de ArgorixLang](PLAN_MAESTRO_ARGORIXLANG.md). Este documento desarrolla la entrega R1 de independencia; no acredita por sí solo la madurez final. Aplicar las dependencias cruzadas del maestro. ESP-026 queda sustituida por fichas MAT independientes.

Estado: EN EJECUCIÓN. ESP-001–008 completadas como inventario, baseline, arquitectura, especificación Core, prototipo de memoria/ABI, frontend Rust stage0, IR Core verificado y ejecución mediante backend C transitorio/runtime C1. ESP-009 (biblioteca estándar mínima), ESP-010 (lexer), ESP-011 (parser) y ESP-012 (resolución, tipos y módulos en Argorix) están HECHAS; ESP-013 (lowering y emisión en Argorix) está EN_CURSO. Self-hosting e independencia completa siguen pendientes.
Fecha: 2026-09-17. Base inspeccionada: `5d73d66`, workspace `1.0.0`.
Última actualización de estado: 2026-09-22 sobre `main@9c77061`. El estado operativo del día (quién tiene cada carril, ramas, PRs e issues abiertos) vive en [WORKBOARD.md](WORKBOARD.md) y en los registros de `coordination/`; esta tabla solo cambia al reclamar o cerrar una tarea.
Este plan reemplazó la prioridad del backlog AL: self-hosting deja de estar diferido. El plan maestro vigente añade funcionalidad completa y producción como entregas obligatorias posteriores.

## 1. Mandato para el modelo ejecutor

Construir un ArgorixLang maduro cuyo compilador esté escrito en ArgorixLang, pueda recompilarse a sí mismo y produzca herramientas y programas que no necesiten Rust. Portar también el runtime, el verificador, las reglas semánticas y las herramientas esenciales. Conservar las garantías y funcionalidades existentes con evidencia de equivalencia y documentar cualquier incompatibilidad.

La metáfora del proyecto es «sacar la espada de la forja»: Rust sirve para fabricar la primera herramienta; deja de ser necesario para mantener, construir y utilizar la entrega independiente. No basta con esconder Cargo, distribuir un ejecutable Rust estático, generar Rust ni escribir una fachada `.argx` que llame a crates o procesos Rust.

Alcance de esta solicitud: crear este plan detallado. La ejecución de las tareas comienza cuando el usuario la solicite; no se presume que la creación del plan haya implementado el lenguaje.

## 2. Qué significa terminar

| Nivel | Evidencia exigida | ¿Cumple el objetivo final? |
| --- | --- | --- |
| Distribución autónoma | Un usuario ejecuta un binario sin instalar Rust | No; el código puede seguir siendo Rust |
| Compilador escrito en ArgorixLang | Fuentes del compilador en `.argx`, construidas inicialmente por stage0 | Parcial; revisar runtime y backend |
| Self-hosting | El compilador Argorix compila sus fuentes y la generación siguiente vuelve a compilarlas | Parcial; pueden quedar dependencias Rust |
| Independencia de Rust | Compilador, runtime y herramientas esenciales se construyen y usan sin Rust, Cargo, crates ni librerías Rust | Sí para independencia de Rust |
| Toolchain nativa propia | El backend emite objetos/código nativo sin transpilar a C ni requerir otro compilador de lenguaje | Meta final de este plan, junto con la fila anterior |

Se permiten interfaces del sistema operativo, un linker de sistema y bibliotecas externas no Rust, inventariadas y justificadas. «Independiente de Rust» no significa inventar un sistema operativo, un linker o criptografía. El núcleo de lógica del compilador y runtime debe estar en ArgorixLang; cualquier shim nativo residual tendrá funciones, tamaño, responsable y pruebas enumeradas. No trasladar la implementación completa a C.

Puerta de independencia R1: en un entorno limpio, usando una semilla Argorix publicada y dependencias no Rust declaradas, construir la toolchain desde sus fuentes `.argx`, recompilarla, compilar programas, ejecutar la VM y verificar evidencia sin consultar binarios, librerías, caches o fuentes Rust. Una compilación cruzada hecha en una máquina con Rust no sustituye esta prueba. Las puertas funcionales y de producción están en el plan maestro.

## 3. Base real y brechas

La inspección de [Cargo.toml](Cargo.toml) muestra compilador, parser, semántica, IR, bytecode, módulos, proveedor, VM, conformidad y firma en crates Rust. [AST actual](crates/argorix_parser/src/ast.rs) organiza declaraciones de agentes, protocolos, políticas y gobernanza. Debe inventariarse exactamente qué construcciones ejecutables faltan antes de fijar nueva sintaxis; este plan no afirma que ya exista un lenguaje general suficiente para escribir un compilador.

La campaña de [evaluación](evaluation/adversarial/README.md) ya incluye correcciones de payload, digest de fuente individual y firma de productor. No presentarlas como pendientes de implementar desde cero. El objetivo es preservar y ampliar su comportamiento en la implementación Argorix.

Brechas observadas que se conservan como trabajo: vinculación de paquetes multifichero, reproducción con checkout limpio, contradicción narrativa de B4 sobre E5, gestión de anclas de confianza, ejecución externa aún planificada en core y separación entre build de evaluación y release. Los resultados existentes pertenecen a su revisión histórica, no a la futura reescritura.

No se volvieron a ejecutar builds ni campañas para redactar este plan. El estado de GitHub/CI debe comprobarse al iniciar la ejecución.

## 4. Arquitectura de transición

Decisión de trabajo: implementar primero un subconjunto de sistemas llamado provisionalmente **Argorix Core**, dentro del mismo lenguaje y toolchain. No crear un lenguaje incompatible separado. Los contratos de agentes conservarán su semántica y usarán el mismo núcleo de tipos y ejecución cuando corresponda.

Camino propuesto:

```text
Compilador Rust congelado + extensión temporal de Core (stage0)
       │ compila fuentes del compilador escritas en Argorix Core
       ▼
Compilador Argorix stage1 → backend C transitorio → compilador C no Rust
       │ compila las mismas fuentes sin usar Rust
       ▼
Compilador Argorix stage2 → compila stage3 → comparación reproducible
       │ incorpora backend nativo escrito en Argorix
       ▼
Compilador nativo Argorix → objeto nativo → linker del sistema
       │ recompila compilador + runtime + herramientas esenciales
       ▼
Release independiente, construida y probada en un entorno sin Rust
```

El backend C es una etapa explícita de bootstrap para no exigir un compilador nativo completo antes de escribir el primer compilador Argorix. Debe tener retiro verificable como backend obligatorio. No se considera completada la meta nativa mientras dependa del compilador C para compilar Argorix.

Primer destino propuesto: Linux x86-64 con ABI y formato de objeto documentados. Segundo destino obligatorio para la entrega: Windows x86-64, por el entorno del proyecto. macOS y otras arquitecturas se anunciarán cuando tengan pruebas propias. No reducir silenciosamente plataformas que antes estaban soportadas: mantener la release histórica disponible y publicar la matriz de transición.

Modelo de memoria provisional de Core: valores y handles comprobados a arenas con vida explícita, sin punteros crudos públicos; liberación de arena invalida handles, y cada acceso verifica límites y generación. Es una decisión a validar con un prototipo en ESP-005, no una propiedad ya demostrada. Si impide implementar el compilador, registrar el cambio de diseño antes de ampliar la semántica.

Las capacidades de compilación (leer el paquete, escribir build, invocar linker permitido) son un perfil explícito del host. No otorgan a programas de agentes las mismas facultades. Cualquier FFI queda interna, limitada y auditada.

## 5. Protocolo de ejecución de cada tarea

1. Leer primero el plan maestro, luego este plan, instrucciones locales aplicables, estado de Git y las evidencias de las dependencias de la tarea. Aplicar las dependencias cruzadas MAT y conservar cambios ajenos.
2. Elegir la primera tarea `PENDIENTE` con dependencias `HECHA`. No dar por construidos los directorios o comandos propuestos aquí.
3. Abrir la ficha `tasks/espada/ESP-NNN.md`, creándola a partir de la plantilla al final. Registrar revisión base y alcance concreto antes de editar código.
4. Inspeccionar los módulos indicados; implementar únicamente la responsabilidad de la tarea y sus cambios indispensables. No traducir ciegamente Rust línea por línea.
5. Añadir pruebas de comportamiento, ejecutar verificaciones pertinentes y conservar logs con commit, plataforma y toolchain. Si algo no se ejecutó, registrarlo como tal.
6. Actualizar ficha y tabla; `HECHA` exige satisfacer todos los criterios, no solo compilar. Si hay bloqueo, conservar avances, dependencia y siguiente acción concreta; no debilitar aceptación para cerrar.
7. Dejar un handoff con archivos cambiados, comandos reales, resultados, riesgos y próxima tarea lista. Los commits/PR deben describir el resultado y no atribuir métricas históricas a código nuevo.

Rutas nuevas de trabajo propuestas: `spec/`, `bootstrap/`, `compiler/`, `stdlib/`, `runtime/`, `tests/selfhost/`, `tests/compatibility/`, `tasks/espada/`, `artifacts/espada/`. Las fuentes nuevas de implementación serán `.argx` tras aprobar la gramática. No mover los crates históricos hasta superar la puerta final.

Los artefactos grandes de CI se publicarán como artefactos de ejecución; versionar manifiestos y fixtures pequeños. No añadir binarios masivos ni resultados que contengan secretos. Este archivo está en la raíz porque `docs/` está ignorado actualmente por Git.

## 6. Índice de tareas y dependencias

ESP-001 a ESP-008 están HECHAS como inventario, baseline histórico, arquitectura, especificación/corpus Core, prototipo de memoria/ABI, frontend Core stage0, IR verificado y backend C transitorio con runtime C1, con evidencia en `tasks/espada/`. ESP-009 está HECHA (carril de Claude, PRs #47, #48 y #51, con las subtareas ESP-009.B a ESP-009.F). ESP-010 y ESP-011 están HECHAS (PRs #53 y #54), y ESP-012 también (PRs #55, #56 y #57), como ESP-013 (PR #58) ESP-014 (PR #59) y ESP-015 (PR #60). ESP-016 está EN_CURSO; ESP-017 a ESP-025 siguen PENDIENTES. ESP-026 está SUSTITUIDA por fichas MAT del maestro. Ordenar por dependencias, incluidas las cruzadas; el ID no autoriza saltarse una puerta.

Una subtarea `ESP-NNN.X` no recorta los criterios de su padre: los habilita o eleva su evidencia (regla de subdivisión del maestro, §10). El padre solo pasa a HECHA cuando cumple todos sus criterios.

| ID | Entregable | Depende de | Estado |
| --- | --- | --- | --- |
| ESP-001 | Inventario de implementación y dependencia Rust | — | HECHA |
| ESP-002 | Baseline reproducible y corpus de compatibilidad | 001 | HECHA |
| ESP-003 | Contrato de independencia y arquitectura | 001 | HECHA |
| ESP-004 | Especificación ejecutable de Argorix Core | 002, 003 | HECHA |
| ESP-005 | Memoria, efectos y ABI del host | 004 | HECHA |
| ESP-006 | Frontend Core temporal en stage0 | 004, 005 | HECHA |
| ESP-007 | IR ejecutable y verificador Core | 006 | HECHA |
| ESP-008 | Backend C temporal y runtime mínimo | 007 | HECHA |
| ESP-009 | Biblioteca estándar mínima | 008 | HECHA |
| ESP-009.B | Defectos del backend C que bloqueaban escribir Core | 008 | HECHA |
| ESP-009.C | Cobertura diferencial de esas construcciones | 009.B | HECHA |
| ESP-009.D | Cobertura diferencial de texto y techos de recursos | 009.C | HECHA |
| ESP-009.E | Ejecución repetida byte a byte idéntica | 009.D | HECHA |
| ESP-009.F | Gaps del backend (#39) e imports entre módulos (#45) | 009.B | HECHA |
| ESP-010 | Lexer y diagnósticos en Argorix | 009 | HECHA |
| ESP-011 | Parser y AST en Argorix | 010 | HECHA |
| ESP-012 | Resolución, tipos y módulos en Argorix | 011 | HECHA |
| ESP-013 | Lowering y emisión en Argorix | 012 | HECHA |
| ESP-014 | Primer compilador self-hosted completo | 013 | HECHA |
| ESP-015 | Bootstrap stage2/stage3 sin Rust | 014 | HECHA |
| ESP-016 | Backend nativo inicial | 015 | EN_CURSO |
| ESP-017 | Segundo destino y bootstrap nativo | 016 | PENDIENTE |
| ESP-018 | Lenguaje de agentes y políticas migrado | 015 | PENDIENTE |
| ESP-019 | VM y scheduler en Argorix | 018 | PENDIENTE |
| ESP-020 | Evidencia, firma y paquetes en Argorix | 019 | PENDIENTE |
| ESP-021 | Runtime externo gobernado | 019, 020 | PENDIENTE |
| ESP-022 | Herramientas y distribución en Argorix | 017, 020 | PENDIENTE |
| ESP-023 | Robustez y evaluación independiente | 017, 020, 021, 022 | PENDIENTE |
| ESP-024 | Prueba final sin Rust y procedencia | 023 | PENDIENTE |
| ESP-025 | Release Espada y retiro operativo de Rust | 024 | PENDIENTE |
| ESP-026 | Editor, interoperabilidad y evolución sostenida | Ver plan maestro | SUSTITUIDA |

### ESP-001 — Inventario de implementación y dependencia Rust

**Objetivo:** saber qué debe migrarse y qué comportamiento no se puede perder.
**Responsabilidad:** workspace, `crates/*`, `src/`, scripts de build, demo y CI; solo inventario en esta tarea.
**Pasos:**

1. Enumerar crate, función, API pública, formatos que lee/escribe y consumidores.
2. Clasificar dependencias: lógica de lenguaje, runtime, sistema operativo, serialización, criptografía, CLI y desarrollo.
3. Registrar procesos externos y librerías enlazadas; buscar dependencias Rust indirectas en wrappers y servicios.
4. Inventariar tipos, expresiones, instrucciones y funciones disponibles realmente en el AST y bytecode; anotar cada carencia de Core.

**Entregables:** `bootstrap/inventory.md` y `bootstrap/dependencies.json` con evidencia de archivo/símbolo y estado por componente.
**Aceptación:** cada miembro del workspace y cada binario tiene destino de migración; se identifican todos los caminos del build y ejecución actuales. No usar cantidad de líneas como prueba de cobertura funcional.

### ESP-002 — Baseline reproducible y corpus de compatibilidad

**Objetivo:** preservar una referencia histórica verificable antes de extender o portar.
**Responsabilidad:** `conformance/`, `evaluation/adversarial/`, `tests/compatibility/`, `bootstrap/`.
**Pasos:** fijar revisión, toolchain y dependencias; construir desde checkout limpio; ejecutar tests y suites declaradas; recopilar salidas válidas y errores; separar golden tests del motor que los produce. Registrar fixtures por versión y perfiles de normalización permitidos. Resolver la inconsistencia B4 en una nueva salida, preservando el resultado original.
**Entregables:** manifiesto de baseline, corpus versionado y runner de comparación.
**Aceptación:** cada fixture tiene origen y resultado esperado revisable; rerun offline reproduce lo que se declara determinista. E5 con API es una campaña separada con presupuesto y modelo registrado. No sobrescribir `e5-live-a` ni el camera-ready.

**Ejecución:** [ficha ESP-002](tasks/espada/ESP-002.md), [baseline](bootstrap/baseline.json), [corpus](tests/compatibility/README.md) y [validación](bootstrap/ESP-002-validation.json). Este cierre fija una referencia histórica de Windows; no demuestra todavía independencia de Rust ni conformidad normativa futura.

### ESP-003 — Contrato de independencia y arquitectura

**Objetivo:** convertir la meta en requisitos auditables y eliminar ambigüedades antes del backend.
**Responsabilidad:** `spec/independence.md`, `bootstrap/architecture.md`.
**Pasos:** adoptar o ajustar razonadamente la ruta C transitoria → nativa; definir formatos, destinos, linker, shims permitidos, semilla publicada y qué herramientas son esenciales; distinguir build del producto de tests históricos que usan Rust. Registrar alcance de compatibilidad y política de versiones.
**Entregables:** decisión de arquitectura y lista legible por máquina de dependencias permitidas/prohibidas.
**Aceptación:** criterios detectan wrapper Rust, servicio Rust, biblioteca estática Rust y backend que emite Rust. Compilador, VM, verificador, conformidad, firma y gestor local de paquetes quedan incluidos en la meta. Un linker externo no se oculta como «cero dependencias».

**Ejecución:** [ficha ESP-003](tasks/espada/ESP-003.md), [contrato](spec/independence.md), [arquitectura](bootstrap/architecture.md), [política](bootstrap/independence-policy.json) y [validación](bootstrap/ESP-003-validation.json). Este cierre valida el diseño y nueve controles sintéticos; no demuestra todavía un build sin Rust.

### ESP-004 — Especificación ejecutable de Argorix Core

**Objetivo:** definir el mínimo lenguaje necesario para implementar su propio compilador.
**Responsabilidad:** `spec/core/`, `tests/selfhost/spec/`.
**Pasos:**

1. Especificar funciones, parámetros, retornos, recursión, variables, alcance y orden de evaluación.
2. Especificar enteros de ancho fijo y overflow, booleanos, bytes, UTF-8, arrays/slices, estructuras, variantes etiquetadas y pattern matching exhaustivo.
3. Definir condicionales, bucles, acceso a campos, llamadas, errores recuperables y módulos. Diferir async, macros, clases y genéricos generales salvo necesidad demostrada.
4. Fijar mutabilidad, igualdad, representación y reglas para tipos recursivos mediante handles. Incluir EBNF, programas válidos, inválidos y resultados.
5. Definir relación con sintaxis de agentes: nuevas construcciones versionadas sin reinterpretar programas existentes silenciosamente.

**Entregables:** especificación y corpus de funciones que manipulan tokens, árboles y tablas de símbolos.
**Aceptación:** se puede expresar en papel un lexer, parser recursivo y recorrido de AST sin primitivas mágicas que invoquen Rust; cada construcción tiene semántica y al menos caso positivo/negativo. Sintaxis del plan no es sintaxis oficial hasta esta tarea.

**Ejecución:** [ficha ESP-004](tasks/espada/ESP-004.md), [especificación](spec/core/README.md), [EBNF](spec/core/grammar.ebnf), [corpus](tests/selfhost/spec/cases.json) y [validación](bootstrap/ESP-004-validation.json). El cierre congela Core 0.1 y demuestra cobertura estructural; no afirma que stage0 ya lo parsea o ejecuta.

### ESP-005 — Memoria, efectos y ABI del host

**Objetivo:** hacer viable Core sin introducir memoria insegura ni permisos implícitos.
**Responsabilidad:** `spec/core/memory.md`, `spec/host-abi.md`, prototipos aislados.
**Pasos:** validar arenas y handles con generación, límites y ownership de arena; definir OOM, profundidad, liberación y errores; definir ABI de texto/bytes y resultados; especificar lectura, escritura, reloj y proceso como capacidades. Diseñar perfil compiler-host y perfil agent-runtime separados. Probar prototipo de árbol y buffer expansible.
**Entregables:** contratos de memoria/FFI y pruebas de handles inválidos, después de liberar, límites y agotamiento.
**Aceptación:** ninguna operación de Core tiene comportamiento indefinido especificado como válido; datos del programa no obtienen FFI arbitraria. Límites y errores son comparables entre backends. Registrar costo de comprobaciones y techo de memoria del prototipo.

**Ejecución:** [ficha ESP-005](tasks/espada/ESP-005.md), [memoria Core](spec/core/memory.md), [ABI host](spec/host-abi.md), [contrato ABI](spec/core/memory-abi.json) y [validación](bootstrap/ESP-005-validation.json). El prototipo Rust seguro valida M1/H1 secuencial y sus límites; no constituye el runtime independiente.

### ESP-006 — Frontend Core temporal en stage0

**Objetivo:** permitir que la herramienta existente compile el subconjunto de arranque.
**Responsabilidad:** parser y semántica Rust actuales, bajo versión/feature explícita de bootstrap.
**Pasos:** implementar tokens, AST, resolución y chequeo de Core aprobado; conservar spans; incorporar pruebas de especificación; mantener separados errores sintácticos, tipos y políticas. Marcar cada módulo añadido como transitorio y vincular su reemplazo Argorix.
**Entregables:** stage0 extendido, pruebas y listado de compatibilidad.
**Aceptación:** corpus Core se acepta/rechaza correctamente y corpus histórico no retrocede; stage0 no contiene atajos que reconozcan el compilador por nombre y lo sustituyan por código Rust.

**Ejecución:** [ficha ESP-006](tasks/espada/ESP-006.md), frontend transitorio en `argorix_parser::core`/`argorix_semantics::core`, comando `argorixc core-check` y [validación](bootstrap/ESP-006-validation.json). Acepta 4/4 positivos, rechaza 12/12 negativos por categoría, detecta la mutación de versión y conserva 405 pruebas del workspace. Sigue siendo Rust stage0 y no genera IR ni ejecuta Core; ESP-007 es la siguiente puerta.

### ESP-007 — IR ejecutable y verificador Core

**Objetivo:** representar computación general y validar invariantes antes de emitir código.
**Responsabilidad:** IR/bytecode de bootstrap, `spec/core/ir.md`.
**Pasos:** definir funciones, bloques, operaciones tipadas, ramas, llamadas, memoria y efectos; elegir forma de IR explícitamente; implementar lowering y verificación; versionar serialización y mantener distinto el bytecode de agentes cuando corresponda; probar archivos malformados directamente.
**Entregables:** IR documentado, verificador y snapshots semánticos.
**Aceptación:** rechaza tipos inconsistentes, referencias inexistentes, control de flujo inválido y efectos no autorizados. Backend nunca recibe IR no verificado. Roundtrip conserva semántica, no solo sintaxis JSON.

**Ejecución:** [ficha ESP-007](tasks/espada/ESP-007.md), [contrato IR](spec/core/ir.md), esquema JSON, lowering/verificador `argorix_ir::core`, comandos `core-emit-ir`/`core-verify-ir` y [validación](bootstrap/ESP-007-validation.json). Los 4 módulos válidos verifican y conservan huella; 5/5 mutaciones malformadas se rechazan. Sigue siendo Rust stage0 y no ejecuta Core; ESP-008 es la siguiente puerta.

### ESP-008 — Backend C temporal y runtime mínimo

**Objetivo:** producir ejecutables Core fuera del runtime Rust para arrancar la migración.
**Responsabilidad:** emisor stage0 y `bootstrap/c/` con shim mínimo.
**Pasos:** mapear IR a C con aritmética, evaluación y memoria explícitas; no depender de overflow firmado ni orden indefinido de C; implementar ABI mínima; compilar con toolchain C declarada no Rust; impedir invocaciones de shell construidas con texto del programa.
**Entregables:** programas Core ejecutables, backend transitorio y manifiesto de dependencias enlazadas.
**Aceptación:** corpus Core produce resultados esperados, incluidos fallos; ejecutables no enlazan runtime Rust ni invocan Rust. Esta tarea prueba ejecución independiente, no self-hosting ni backend nativo propio.

**Ejecución:** [ficha ESP-008](tasks/espada/ESP-008.md), backend `argorix_ir::CoreCBackend`, runtime C1 en `bootstrap/c/`, 10 fixtures ejecutables, runner nativo en `conformance/core_c/` y [validación](bootstrap/ESP-008-validation.json). gcc, clang y un host Debian sin herramientas Rust ejecutaron 10/10 casos; inspección ELF no encontró dependencias Rust. El emisor sigue siendo Rust stage0 y C sigue siendo transitorio: no acredita self-hosting, backend nativo ni independencia completa.

### ESP-009 — Biblioteca estándar mínima

**Objetivo:** disponer de estructuras y utilidades para escribir el compilador en Argorix.
**Responsabilidad:** `stdlib/` en `.argx`, ABI host mínima.
**Pasos:** implementar bytes/texto, vectores, mapas deterministas, resultados, arenas, rutas y acceso acotado a archivos; añadir serialización de los formatos realmente usados; usar orden estable para emisión. Probar Unicode inválido, números límite, claves duplicadas, paths y tamaños excesivos.
**Entregables:** biblioteca con API documentada y ejemplos de tokenización/árbol de símbolos.
**Aceptación:** lógica de colecciones y serialización escrita en Argorix; no un wrapper a serde o utilidades Rust. Rendimiento y memoria suficientes para procesar una muestra representativa del compilador; registrar medidas.

**Estado (2026-09-23):** HECHA, en el carril de Claude (ramas
`claude/stdlib-*`, PRs #47, #48 y #51). Cerrado:

- La API/frontera S1 en [`spec/core/stdlib.md`](spec/core/stdlib.md).
- `Buffer<T>` y `Arena<T>`/`Handle<T>` ejecutables
  ([PR #26](https://github.com/argorixlabs/argorixlang/pull/26)).
- Semántica de movimiento y liberación determinista de recursos
  ([PR #47](https://github.com/argorixlabs/argorixlang/pull/47)).
- Ocho módulos `.argx` en `stdlib/`: `result`, `bytes`, `text` (UTF-8 según
  la tabla 3-7), `vector`, `ordered_map` (orden por bytes y duplicados
  rechazados), `arena` (árbol de ámbitos enlazado por handles), `path`
  (sólo rutas relativas normales) y `json` (escritor canónico y validador).

Catorce fixtures, positivas y adversarias (UTF-8 inválido, claves duplicadas,
traversal, desbordamiento, JSON mal formado, profundidad, rangos y handles
caducados, archivos ausentes y presupuestos), entre ellas un tokenizador y un
árbol de símbolos de ejemplo, pasan con gcc y clang, con y sin sanitizers.

La frontera de archivos del compilador (`stdlib.compiler_host`) también está
hecha:

- capacidades `PackageRead`/`BuildWrite` que sólo recibe `argorix_main`,
  con raíces canónicas y presupuestos de bytes;
- rechazo de traversal y de symlinks fuera de la raíz;
- ningún otro binario importa funciones de archivos.

Las medidas sobre una carga representativa están en
`bootstrap/ESP-009-measurements.json`: unos 2 MB de memoria con 16.000
miembros, pero tiempo cuadrático, por la validación de claves duplicadas de
JSON y la inserción en `ordered_map`. Todos los criterios de aceptación se
cumplen en la pila de PRs #48–#50; falta el CI sobre `main` tras la revisión.
Windows no tiene todavía la frontera de archivos.

Subtareas cerradas por el carril de Claude, ninguna recorta criterios del
padre:

- **ESP-009.B** ([ficha](tasks/espada/ESP-009.B.md),
  [PR #37](https://github.com/argorixlabs/argorixlang/pull/37)): los dieciséis
  defectos del issue #27 que impedían escribir Core —`if` como sentencia,
  bloques con ámbito, tipos anidados, shifts, negación con signo, dos
  resultados silenciosamente incorrectos y un SIGSEGV donde la especificación
  exige un trap tipado— están cerrados y conservados como corpus de regresión
  en `conformance/core_c/regression/`.
- **ESP-009.C** ([ficha](tasks/espada/ESP-009.C.md),
  [PR #40](https://github.com/argorixlabs/argorixlang/pull/40)): el generador
  diferencial produce ya esas construcciones en vez de evitarlas y las juzga
  contra un oráculo derivado de `spec/core/evaluation.md`. 1200 programas con
  gcc 12.2, clang 14.0.6 y sanitizers sin una sola divergencia. Dejó siete
  defectos registrados con repro mínimo en `conformance/core_c/gaps/`
  (issues [#38](https://github.com/argorixlabs/argorixlang/issues/38) y
  [#39](https://github.com/argorixlabs/argorixlang/issues/39)): `loop` como
  sentencia, `break` con valor, `match` sobre bool y sobre entero, guardas,
  constantes de módulo y `x = x;`.

- **ESP-009.D** ([ficha](tasks/espada/ESP-009.D.md),
  [PR #41](https://github.com/argorixlabs/argorixlang/pull/41)): las dos
  partes de ESP-009 que ya se ejecutan —bytes/UTF-8 y los techos de recursos
  de `Buffer` y `Arena`— pasan de una fixture escrita a mano cada una a
  entradas generadas y adversarias. 960 programas sin divergencia, con 38
  `UTF8_INVALID` y 41 `RESOURCE_LIMIT`; el validador UTF-8 del oráculo sale de
  la tabla 3-7 del estándar Unicode, no del runtime.

- **ESP-009.E** ([ficha](tasks/espada/ESP-009.E.md)): `spec/core/stdlib.md`
  exige que la ejecución repetida con gcc y clang produzca una salida idéntica
  byte a byte, y nadie lo comprobaba: cada caso se ejecutaba una vez. Ahora
  `--repeat` compara cada corrida con la primera, CI usa tres en los dos
  compiladores, y un control negativo que se ejecuta demuestra que la
  comprobación puede fallar.

- **ESP-009.F** ([ficha](tasks/espada/ESP-009.F.md)): el backend acepta ya
  `loop` como sentencia y con valor, `match` sobre bool y enteros con
  guardas, y constantes de módulo (issue #39), y un programa repartido en
  varios módulos se chequea y se ejecuta (issue #45), con un linker que
  resuelve `b.f(..)`, `b.C` y los tipos públicos importados. Es lo que hacía
  falta para escribir la stdlib en Core modular.

Las cinco subtareas prepararon el backend y el arnés; los módulos `.argx` de
la biblioteca llegaron después, sobre ese terreno (ver el estado de arriba).

### ESP-010 — Lexer y diagnósticos en Argorix

**Objetivo:** primera pieza del compilador implementada en su propio lenguaje.
**Responsabilidad:** `compiler/lexer/`, `compiler/diagnostics/`.
**Pasos:** portar semántica de tokens, comentarios, escapes, spans y recuperación; preservar línea/columna y bytes; producir diagnósticos estructurados; comparar con especificación y baseline, no solo con stage0.
**Entregables:** lexer `.argx`, tests y tabla de discrepancias resueltas.
**Aceptación:** tokeniza sus propias fuentes; entradas truncadas, Unicode y escapes inválidos producen errores acotados. El lexer Rust ya no se usa por debajo del lexer Argorix.

**Estado (2026-09-23):** HECHA en el carril de Claude ([ficha](tasks/espada/ESP-010.md),
PR #53). El lexer y sus diagnósticos están en `compiler/`. Lee sus fuentes
por la frontera de archivos del compilador, y su volcado canónico coincide
byte a byte con el del lexer stage0 en 49 muestras: entradas adversarias, sus
propias fuentes, la stdlib y el corpus de la spec.

### ESP-011 — Parser y AST en Argorix

**Objetivo:** construir el árbol de Core con código Argorix.
**Responsabilidad:** `compiler/parser/`, `compiler/ast/`.
**Pasos:** implementar precedencia, declaraciones, funciones, tipos y recuperación; resolver árboles recursivos con el modelo aprobado; conservar spans y límites; comparar AST normalizado del corpus.
**Entregables:** parser y AST `.argx`, fixtures de precedencia y errores.
**Aceptación:** parsea todas sus fuentes y las del lexer; rechaza ambigüedades/errores de acuerdo con la gramática; entradas profundas se limitan sin bloquear el proceso.

**Estado (2026-09-23):** HECHA en el carril de Claude ([ficha](tasks/espada/ESP-011.md),
PR #54). El parser y el AST están en `compiler/`, y su volcado canónico
(`spec/core/ast.md`) coincide byte a byte con el del parser stage0 en 135
archivos. Incluyen sus propias fuentes, la stdlib, los corpus de fixtures y
muestras de precedencia, recuperación y anidamiento profundo. El puerto
destapó que el parser stage0 no tenía límite de anidamiento; ambos cortan
ahora en 256 niveles con un diagnóstico.

### ESP-012 — Resolución, tipos y módulos en Argorix

**Objetivo:** trasladar decisiones semánticas de Core fuera de Rust.
**Responsabilidad:** `compiler/resolve/`, `compiler/types/`, `compiler/modules/`.
**Pasos:** implementar scopes, firmas, llamadas, recursión, tipos, exhaustividad, efectos y grafo de imports; resolver determinísticamente ciclos permitidos y prohibidos; limitar rutas al paquete y declarar permisos de compilación.
**Entregables:** frontend semántico `.argx` y diagnósticos comparables.
**Aceptación:** valida sus fuentes; detecta símbolos duplicados, tipos incorrectos, retornos ausentes, handles mal usados e imports inválidos; decisiones de error tienen pruebas independientes.

**Estado (2026-09-24):** HECHA en el carril de Claude ([ficha](tasks/espada/ESP-012.md)),
en tres subtareas:

- ESP-012.A (PR #55): el checker de declaraciones, tipos, scopes, nombres,
  expresiones, control de flujo, exhaustividad y literales, en
  `compiler/check.argx`;
- ESP-012.B (PR #56): el ownership de recursos (moves por cada camino, bucles,
  lugares, arenas) y las vistas;
- ESP-012.C (PR #57): el grafo de módulos y el linker, en `compiler/link.argx`.

Los diagnósticos coinciden byte a byte con los de stage0: 141 archivos sueltos
y 170 paquetes, entre ellos el propio compilador enlazado consigo mismo. CI
verde en `main@def7a46`.

### ESP-013 — Lowering y emisión en Argorix

**Objetivo:** completar el camino fuente Core → ejecutable desde fuentes Argorix.
**Responsabilidad:** `compiler/ir/`, `compiler/codegen/c/`.
**Pasos:** implementar IR, verificador, lowering y backend C en Argorix; respetar contratos de ESP-007/008; eliminar dependencia del emisor Rust en este recorrido; emitir manifiesto de fuentes y opciones.
**Entregables:** pipeline escrito en `.argx` y comparación de comportamiento por backend.
**Aceptación:** compila programas del corpus y componentes del compilador; no importa generar C idéntico a stage0, sí preservar semántica. Las diferencias de resultados deben explicarse contra especificación.

**Estado (2026-09-24):** HECHA en el carril de Claude ([ficha](tasks/espada/ESP-013.md)),
fusionada en el PR #58 con CI verde en `main@89d67d0`, en cuatro subtareas:

- ESP-013.A: el IR canónico en `compiler/ir.argx`, idéntico byte a byte al de
  stage0 en 170 paquetes.
- ESP-013.C: el backend C en `compiler/c_emit.argx`, que escribe el mismo C que
  stage0 y alcanza un punto fijo al compilarse a sí mismo.
- ESP-013.D: el pipeline en `compiler/pipeline.argx`, con un manifiesto de
  fuentes, orden de enlace, opciones y salida, con SHA-256 escrito en Argorix.
- ESP-013.B: el verificador del IR en `compiler/ir_verify.argx`, que lee el
  JSON como serde y coincide con stage0 en 248 documentos.

Las cuatro subtareas están hechas.

### ESP-014 — Primer compilador self-hosted completo

**Objetivo:** integrar todas las piezas Core en un compilador usable.
**Responsabilidad:** `compiler/main.argx`, configuración de bootstrap y CLI inicial.
**Pasos:** añadir lectura de proyecto, argumentos, diagnósticos y emisión; construir stage1 con stage0; ejecutar stage1 sobre todas sus fuentes; quitar dependencias accidentales a procesos/helper Rust.
**Entregables:** stage1 y manifiesto de su origen, fuentes, herramientas y hashes.
**Aceptación:** stage1 puede generar un compilador funcional a partir de sus fuentes completas; no solo compila un «hello world» o un parser. Cada fase del compilador es Argorix; C sigue declarado como backend temporal.

**Estado (2026-09-24):** HECHA en el carril de Claude (PR #59, CI verde en
`main@9054379`; [ficha](tasks/espada/ESP-014.md)), en cuatro subtareas:

- ESP-014.A: los diagnósticos con los mensajes de stage0, renderizados como
  los imprime `argorixc core-emit-c` (`compiler/report.argx`); coinciden byte
  a byte en los 189 paquetes del diferencial.
- ESP-014.B: el driver `compiler/main.argx`, que lee `argorix.build` (el perfil
  compiler-host no da línea de órdenes) y escribe el C, el manifiesto y los
  diagnósticos (`spec/core/stage1.md`).
- ESP-014.C: stage0 construye stage1, y stage1 compila sus 24 fuentes al mismo
  C que stage0; ese C, compilado, repite la construcción. En CI stage1 se
  construye y se compila a sí mismo en un contenedor sin Rust, con gcc y con
  clang, y `bootstrap/stage1.py` escribe el manifiesto de procedencia.
- ESP-014.D: los rechazos del backend C, con el mensaje de stage0.

Las cuatro subtareas están hechas.

### ESP-015 — Bootstrap stage2/stage3 sin Rust

**Objetivo:** demostrar el ciclo de autocompilación sin asistencia oculta.
**Responsabilidad:** `bootstrap/`, `tests/selfhost/` y CI aislada.
**Pasos:** stage1 compila las mismas fuentes a stage2; stage2 a stage3, con herramientas y flags fijados; ejecutar suite con stage2/3; controlar timestamps, rutas y orden; deshabilitar acceso a stage0, Rust, caches y red no requerida. Probar modificaciones reales a un diagnóstico y a una función del compilador para demostrar que no se copia una semilla preconstruida.
**Entregables:** logs de bootstrap, hashes, comparación y entorno reproducible.
**Aceptación:** stage2/3 tienen igualdad binaria en el entorno determinista acordado y conformidad; diferencias de metadatos deben resolverse, no normalizarse silenciosamente. Igualdad de bootstrap no prueba ausencia de un compilador malicioso: esa limitación queda explícita.

**Estado (2026-09-24):** HECHA en el carril de Claude (PR #60; [ficha](tasks/espada/ESP-015.md)).
`bootstrap/selfhost.py` lleva stage1 → stage2 → stage3 solo con un compilador C y
Python:

- los tres stages escriben el mismo C (el de stage0), manifiesto y
  diagnósticos, y sus ejecutables son idénticos byte a byte;
- mover las fuentes de directorio o invertir el orden de los módulos no
  cambia el C;
- los 87 casos de las suites C pasan con stage3, y stage2 escribe el mismo
  C para cada uno;
- editar un diagnóstico y una función del backend se refleja en los
  compiladores construidos desde las fuentes editadas, que llegan a su punto
  fijo.

En CI corre en un contenedor sin Rust y con `--network none`, con gcc y con
clang. La igualdad entre stages no prueba la ausencia de un compilador
malicioso; el informe lo deja explícito.

### ESP-016 — Backend nativo inicial

**Objetivo:** retirar el compilador C del camino obligatorio de compilación Argorix.
**Responsabilidad:** `compiler/codegen/native/`, target Linux x86-64, pruebas de ABI.
**Pasos:** especificar registros, stack, convenciones de llamada, layout/alineación, relocaciones y objeto; implementar selección de instrucciones, asignación de registros inicial y emisión de objeto; enlazar con linker declarado; portar o delimitar shim host; sin optimizaciones complejas hasta lograr conformidad.
**Entregables:** backend nativo `.argx`, fixtures de objeto/ABI y binarios.
**Aceptación:** compila Core y el compilador completo sin generar C ni invocar compilador C. Llamadas, enteros, ramas, arenas y errores coinciden con el backend temporal. Bibliotecas de sistema y linker quedan inventariados; dependencias para reconstruir shims se declaran por separado.

**Estado (2026-09-24):** EN_CURSO en el carril de Claude ([ficha](tasks/espada/ESP-016.md),
[especificación](spec/core/native-x86-64.md)). `compiler/native.argx`, con el
codificador `compiler/x86.argx` y el escritor de objetos `compiler/elf.argx`,
compila Core a código x86-64 en un objeto ELF64 que `ld` enlaza con el shim de
runtime:

- parte de las tablas y comprobaciones del backend C, así que rechaza los
  mismos programas con el mismo motivo y ejecuta el mismo programa;
- coincide con el backend C en los 191 paquetes del diferencial, en los 87
  casos de las suites y en 480 programas generados contra el oráculo;
- el compilador nativo se compila a sí mismo hasta un punto fijo, byte a
  byte, en un contenedor sin compilador C, sin Rust y sin red
  (`bootstrap/native.py`), y ediciones reales del diagnóstico y del backend
  se propagan;
- `bootstrap/native/toolchain.json` inventaría linker, biblioteca C y shim, y
  declara aparte el compilador C que solo reconstruir el shim necesita.

Se cierra al fusionarse con CI verde.

### ESP-017 — Segundo destino y bootstrap nativo

**Objetivo:** obtener autocompilación nativa en Linux y Windows soportados.
**Responsabilidad:** targets, host ABI Windows, CI por plataforma.
**Pasos:** añadir formato de objeto y ABI Windows x86-64, rutas/Unicode/IO; realizar bootstrap stage2/3 usando backend nativo en cada plataforma; comprobar referencias a bibliotecas y arquitectura; documentar depuración mínima y símbolos.
**Entregables:** dos matrices de bootstrap, binarios nativos y guía de construcción.
**Aceptación:** ambos entornos recompilan y ejecutan sin Rust ni compilador C para fuentes Argorix. No comparar hashes entre plataformas distintas; comparar dentro del mismo entorno fijado. Plataforma no validada permanece no soportada por la nueva release.

### ESP-018 — Lenguaje de agentes y políticas migrado

**Objetivo:** que self-hosting no signifique abandonar el producto ArgorixLang existente.
**Responsabilidad:** frontend, semántica de agentes, lowering y formatos heredados.
**Pasos:** migrar declaraciones y reglas inventariadas en ESP-001; cubrir protocolos, capabilities, módulos, typed messages, governance y versiones; implementar matriz de compatibilidad; corregir divergencias usando especificación y baseline. Reutilizar infraestructura Core sin habilitar efectos adicionales.
**Entregables:** compilador Argorix del lenguaje completo y matriz funcional por feature/versión.
**Aceptación:** todo elemento del inventario tiene implementación o incompatibilidad explícita aceptada dentro del alcance; corpus histórico aplicable pasa. DENY, REVIEW y UNKNOWN no se convierten en PASS por la migración.

### ESP-019 — VM y scheduler en Argorix

**Objetivo:** retirar el runtime Rust, no solo el frontend.
**Responsabilidad:** `runtime/vm/`, scheduler, mailboxes, provider registry y bytecode verifier.
**Pasos:** portar carga/verificación, estados, instrucciones, eventos y scheduling determinista; implementar límites de memoria/pasos; centralizar autorización final; mantener modos separados; probar bytecode hostil sin pasar por compilador.
**Entregables:** VM `.argx`, CLI y comparación diferencial de trazas.
**Aceptación:** ejecuta y verifica el corpus con resultados equivalentes; denegación no alcanza proveedor; simulación/dry-run no abren red. No ejecuta la VM Rust como subproceso ni enlaza crates. Los dos backends se usan para detectar discrepancias durante la transición.

### ESP-020 — Evidencia, firma y paquetes en Argorix

**Objetivo:** conservar verificabilidad y cerrar límites de fuentes/producer en la toolchain nueva.
**Responsabilidad:** `runtime/evidence/`, herramientas de firma/verificación y paquetes.
**Pasos:** portar canonicalización, digests, reportes y trust ledger; vincular manifest y módulos transitivos con rutas portables; definir anclas, rotación y revocación; integrar primitiva criptográfica no Rust existente y revisada mediante FFI limitada si corresponde. No diseñar criptografía propia para cumplir self-hosting.
**Entregables:** herramientas `.argx`, manifiesto de biblioteca criptográfica y fixtures históricos/nuevos.
**Aceptación:** modificaciones de fuente individual/paquete, firma ausente/ajena y reemplazo coordinado se comportan según política; evidencia antigua conserva límites explícitos; claves privadas fuera de VM y artefactos. Verificación cruzada entre implementaciones cuando el formato es compatible.

### ESP-021 — Runtime externo gobernado

**Objetivo:** madurar la ejecución útil sobre el runtime ya migrado.
**Responsabilidad:** adaptador opcional, operación inicial, demo y pruebas de sinks.
**Pasos:** definir transporte no Rust y una operación; validar runtime/adapter/provider/operation/capability/policy antes del efecto; añadir timeout, cancelación, cuotas, límites de salida y secretos; probar primero servidor local y después proveedor real con presupuesto; medir aislamiento por plataforma con sensores positivos.
**Entregables:** integración real, contrato de permisos y evidencia por invocación.
**Aceptación:** operación permitida completa recorrido y denegada no llega al destino; fallos parciales no reportan éxito; ninguna dependencia Rust se introduce vía adaptador. No llamar sandbox a una allowlist sin aislamiento efectivo probado.

### ESP-022 — Herramientas y distribución en Argorix

**Objetivo:** que el uso diario no requiera mantener un segundo toolchain Rust.
**Responsabilidad:** comandos de proyecto, build, run, check, format, conformidad y packaging.
**Pasos:** implementar CLI estable y códigos de salida, formateador idempotente y gestor de paquetes locales; fijar dependencias sin descargas implícitas; producir paquetes instalables y documentación; trasladar runner esencial de conformidad a Argorix. Python u otros lenguajes pueden seguir en investigación auxiliar, declarados fuera del build obligatorio.
**Entregables:** herramientas `.argx` y tres ejemplos instalables (Core, agentes offline, ejecución gobernada).
**Aceptación:** runner limpio instala la release y crea/compila/ejecuta/verifica un proyecto; no requiere checkout ni Cargo. Formateo conserva semántica y se aplica a las propias fuentes.

### ESP-023 — Robustez y evaluación independiente

**Objetivo:** demostrar madurez de la reescritura y detectar regresiones de garantías.
**Responsabilidad:** corpus, fuzzing, pruebas generativas, evaluación adversarial y rendimiento.
**Pasos:** atacar parser, tipos, IR, loader, memoria y evidencia; generar casos desde especificación además de comparar contra Rust; cubrir límites y fallos del host; repetir campañas aplicables sobre binarios nuevos; medir compilación propia, VM y verificación. Determinar umbrales desde baseline antes de optimizar.
**Entregables:** informe por revisión, casos reproducibles, regresiones y matriz de capacidades.
**Aceptación:** cero divergencias sin explicación en corpus obligatorio; controles positivos de sensores funcionan; errores y casos no ejecutados cuentan en resultados. Un fallo compartido entre compiladores puede escapar a la comparación diferencial; mantener pruebas independientes. Gates cuantitativos de tiempo/memoria se fijan y registran antes de cerrar.

### ESP-024 — Prueba final sin Rust y procedencia

**Objetivo:** demostrar independencia real del producto y su construcción normal.
**Responsabilidad:** CI limpia, auditoría de dependencias y bootstrap de release.
**Pasos:**

1. Preparar entornos Linux/Windows con solo semilla Argorix verificada, linker y dependencias permitidas; sin rustc, Cargo, rustup, crates ni caches montadas.
2. Bloquear fallback de descarga y llamadas a hosts/servicios Rust; registrar procesos, archivos y dependencias estáticas/dinámicas.
3. Recompilar toolchain nativa y runtime desde `.argx`; producir stage2/3 y ejecutar corpus, aplicaciones y evidencia.
4. Revisar SBOM y provenance de dependencias, incluido código enlazado estáticamente. Ausencia de cadenas «rust» en un binario no es prueba suficiente.
5. Documentar origen de la semilla y, cuando sea viable, realizar compilación diversa independiente. Diferenciar bootstrap normal sin Rust de reconstrucción arqueológica de stage0, que puede conservarlo.

**Entregables:** `bootstrap/independence-report.json`, logs auditables, hashes, SBOM y cadena de procedencia.
**Aceptación:** ninguna dependencia Rust de build o ejecución del producto; lógica principal en Argorix; backend nativo obligatorio; shims residuales explícitos. La prueba no se limita a quitar Rust del PATH en una máquina que aún lo contiene.

### ESP-025 — Release Espada y retiro operativo de Rust

**Objetivo:** entregar la nueva herramienta con mantenimiento y actualización independientes.
**Responsabilidad:** release, CI por defecto, documentación, archivo histórico y migración.
**Pasos:** seleccionar versión según compatibilidad; publicar binarios/semilla/checksums y fuentes; hacer del build Argorix el predeterminado; mover referencia Rust a ubicación histórica o tag preservado después de revisar consumidores; ofrecer guía de migración y rollback. Mantener instrucciones de reconstrucción histórica separadas.
**Entregables:** release verificada, matriz de plataformas, notas, guía y política de soporte.
**Aceptación:** una segunda instalación limpia recompila la release y trabaja con proyectos existentes; CI de producto no usa Rust. No borrar crates antes de ESP-024 ni quitar tests para lograr verde. Actualizar README: Rust pasa a origen histórico con enlace a su referencia.

### ESP-026 — Editor, interoperabilidad y evolución sostenida

**Estado: SUSTITUIDA, no ejecutarla como una tarea adicional.** Su alcance se desarrolla ahora en MAT-013 (ciclo de agentes), MAT-014 (MCP), MAT-015 (comunicación autenticada), MAT-018 (depuración), MAT-019 (editor), MAT-020 (documentación) y MAT-027 (aplicaciones), con validaciones MAT-023 a MAT-030. Estas capacidades son obligatorias para el perfil de madurez del plan maestro, aunque la entrega intermedia R1 pueda publicarse antes.

## 7. Puertas de madurez y estrategia de trabajo

| Puerta | Tareas | Demostración necesaria |
| --- | --- | --- |
| G0: contrato congelado | 001–005 | Alcance, corpus, Core y memoria definidos |
| G1: semilla útil | 006–009 | Core produce ejecutables sin runtime Rust |
| G2: self-hosting | 010–015 | Compilador `.argx` reproduce stage2/3 sin Rust, con backend C declarado |
| G3: compilación nativa | 016–017 | Bootstrap nativo en Linux/Windows |
| G4: producto migrado | 018–022 | Lenguaje de agentes, VM, evidencia y herramientas portados |
| G5: espada independiente | 023–025 | Release auditada sin dependencias Rust |

Ruta crítica del compilador: 001 → 002/003 → 004 → 005 → 006 → 007 → 008 → 009 → 010 → 011 → 012 → 013 → 014 → 015 → 016 → 017. La rama de runtime 018–021 también debe completarse antes de la independencia final. Self-hosting del frontend por sí solo no cierra G5.

Planificar por puertas, no prometer la reescritura completa en dos semanas. Es un programa de ingeniería de varios ciclos; estimar duración de cada puerta después de medir ESP-001/002 y el prototipo ESP-005. El primer ciclo vigente está en el plan maestro e incluye requisitos MAT antes de fijar la arquitectura. No iniciar implementaciones sin sus dependencias.

## 8. Correspondencia con el backlog anterior

| Backlog anterior | Tratamiento actual |
| --- | --- |
| AL-001–003: documentación, reproducción, CI | ESP-001/002/015/023 |
| AL-004: especificación | ESP-003–007 y ESP-018 |
| AL-005: operaciones | ESP-019/021 |
| AL-006–007: paquetes y confianza | ESP-020 |
| AL-008–010: ejecución externa | ESP-021, después de migrar runtime |
| AL-011–013: herramientas y distribución | ESP-022/025/026 |
| AL-014–016: puentes y agentes | ESP-026, con evaluación ESP-023 ampliada |
| AL-017–018: robustez y release | ESP-023–025 |

La prioridad anterior de diferir self-hosting queda revocada por este mandato. Las funciones útiles del producto se preservan; su implementación futura debe pasar los controles de independencia.

## 9. Plantilla obligatoria de ficha ejecutable

Copiar para cada tarea al comenzar, usando rutas y comandos que existan realmente:

```markdown
# ESP-NNN — Título
Estado: PENDIENTE | EN_CURSO | BLOQUEADA | HECHA
Revisión base:
Dependencias y evidencia de cierre:
Objetivo observable:
Responsabilidad: archivos/módulos que se modificarán
Entradas: especificación, fixtures y contratos
Decisiones: alternativas, opción y motivo
Pasos:
- [ ] Inspección y baseline
- [ ] Implementación
- [ ] Pruebas de aceptación específicas del plan
- [ ] Compatibilidad y revisión de dependencias
- [ ] Documentación y handoff
Comandos realmente ejecutados:
Resultados y enlaces a logs/manifiestos:
Criterios de aceptación: uno por uno con evidencia
Dependencias no Argorix y justificación:
Dependencias Rust restantes:
Limitaciones, fallos y pruebas no ejecutadas:
Commit/PR si existe:
Próxima tarea lista y contexto necesario:
```

## 10. Prompt de continuidad para otro modelo

> Empieza por PLAN_MAESTRO_ARGORIXLANG.md y aplica sus dependencias cruzadas. Para tareas ESP usa este documento como detalle de implementación de independencia R1. R2 y R3 requieren también las fichas MAT; ESP-026 está sustituida. Verifica el estado real, conserva baseline y cambios ajenos, ejecuta criterios y registra evidencia sin inventar resultados. No confundas una release independiente con una release de producción.
