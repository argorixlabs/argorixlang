# Contrato de independencia de ArgorixLang

Estado: normativo para la arquitectura R1 (ESP-003). Base: `5d73d663bf5b85fd71bdd0cbc1ac1636959d7e64`. Este documento define qué significa construir y usar ArgorixLang sin Rust; no afirma que la implementación actual lo cumpla.

## IND-A — Definición verificable

Una distribución es **independiente de Rust** sólo si, en Ubuntu 24.04 x86-64 y Windows 11 x86-64 limpios:

1. construye desde fuentes `.argx` publicadas usando una semilla Argorix publicada y dependencias permitidas inventariadas;
2. recompila compilador, runtime, verificador, conformidad, firma y gestor local de paquetes;
3. compila y ejecuta el corpus obligatorio, verifica su evidencia y repite el bootstrap;
4. no dispone ni consulta `rustc`, Cargo, rustup, crates, fuentes Rust, caches Cargo, `.rlib`, `.rmeta`, bibliotecas estáticas Rust o servicios implementados en Rust;
5. el backend normal emite objeto nativo para el destino, no Rust, C, WebAssembly que dependa de runtime Rust ni una petición a un compilador remoto;
6. todo linker, biblioteca, dato Unicode, syscall/Win32 API y shim residual aparece en el manifiesto del candidato con versión, hash, licencia, función y responsable.

Un binario histórico Rust puede ser una semilla arqueológica para crear la primera generación fuera del ensayo final. No puede estar presente en el entorno de build normal R1 ni ser necesario para actualizar el compilador. Distribuir un ejecutable estático Rust, cambiarle el nombre, envolverlo desde `.argx`, llamarlo por proceso, consultarlo como servicio o enlazar sus objetos **no** satisface este contrato.

## IND-B — Componentes esenciales

La puerta cubre exactamente estos componentes de producto; omitir uno falla el candidato:

| ID | Componente | Responsabilidad mínima |
| --- | --- | --- |
| `compiler` | Compilador y backend nativo | fuente, módulos, tipos, lowering y objetos nativos |
| `runtime` | VM/runtime y scheduler | ejecución, políticas, límites y adaptadores soportados |
| `verifier` | Verificador offline | bytecode, evidencia, firma/ancla y rechazo fail-closed |
| `conformance` | Runner de conformidad | suites, mutaciones, resultados y controles negativos |
| `signer` | Firma separada | generación/importación de clave y firma, fuera de la VM |
| `package_manager` | Gestor local mínimo | manifest, lock, resolución y verificación reproducible |

La CLI, stdlib y backend pertenecen a `compiler`; no se ocultan como herramientas externas. Tests históricos, paper y demo web pueden quedar fuera de la distribución esencial, pero sus casos obligatorios se portan al runner independiente.

## IND-C — Dependencias permitidas

Se permiten sólo capacidades que Argorix no pretende reemplazar, siempre declaradas:

- kernel y ABI del sistema soportado;
- linker del sistema (`ld`/`lld` o `link.exe`/`lld-link`) para unir objetos emitidos por el backend Argorix;
- formatos ELF y PE/COFF documentados;
- bibliotecas no Rust revisadas para criptografía, TLS o compresión mediante shim estrecho y versionado, cuando una tarea posterior seleccione la implementación;
- archivos de datos versionados, por ejemplo tablas Unicode o certificados raíz, con procedencia y hash;
- un compilador C **únicamente** en las etapas transitorias B1/B2 descritas en la arquitectura, nunca en build, uso, actualización o reconstrucción normal de R1.

Cada shim debe enumerar símbolos, ABI, ownership de memoria, códigos de error, tamaños por destino, consumidor, pruebas y plan de retiro o permanencia. “Biblioteca del sistema” sin nombre/versión no es inventario. Implementar toda la toolchain en C detrás de un shim tampoco es válido.

## IND-D — Dependencias prohibidas y detección

El manifiesto del candidato y el análisis del árbol/binarios deben rechazar:

- ejecutables o invocaciones `cargo`, `rustc`, `rustup`;
- archivos `.rs`, `.rlib`, `.rmeta`, Cargo manifests/lock/caches en el paquete o inputs del build R1;
- librerías estáticas/dinámicas o blobs cuya procedencia incluya Rust, aunque su nombre no lo diga;
- procesos locales, contenedores, RPC/HTTP o servicios remotos que ejecuten lógica esencial Rust;
- backend que emita Rust o requiera transpilar por Rust;
- wrapper Argorix/shell/C que delegue una fase esencial a un binario Rust;
- descarga dinámica de toolchains o dependencias no fijadas;
- compilador C en el camino normal del candidato final.

La ausencia de nombres Rust no basta. ESP-024 deberá combinar manifiesto, interceptación de procesos/archivos/red, imports y símbolos, SBOM/procedencia, entorno señuelo y controles positivos. Si el sensor no detecta un canario Rust introducido deliberadamente, la prueba es inválida.

## IND-E — Semilla y bootstrap

La release publicará dos objetos distintos:

1. **semilla Argorix por plataforma**, un binario previamente auditado y firmado que acepta las fuentes del compilador; y
2. **fuentes autoritativas `.argx`**, manifest/lock, tablas y shims permitidos.

Cada semilla registra plataforma, formato, SHA-256, firma, ancla, versión de bytecode/ABI, revisión fuente que la generó y cadena de bootstrap. La confianza en una semilla no se resuelve por auto-compilación; se reduce mediante hashes reproducibles, reconstrucción diversa cuando sea viable, procedencia y auditoría. La reconstrucción arqueológica desde stage0 Rust se conserva como procedimiento excepcional separado del build normal.

El bootstrap estable exige que stage2 compile stage3 desde las mismas fuentes y que la comparación definida por versión sea idéntica. Hasta fijar canonicalización completa, se comparan objeto normalizado, interfaces, corpus y comportamiento; cualquier diferencia queda explicada y aprobada, nunca ignorada.

## IND-F — Puertas de auditoría

| Puerta | Evidencia requerida | No demuestra |
| --- | --- | --- |
| `G-ARCH` | política legible por máquina y arquitectura validadas | código implementado |
| `G-BOOT` | stage2→stage3 estable y corpus Core | ausencia de Rust final |
| `G-NATIVE` | backend emite ELF/COFF y no requiere C | seguridad del runtime |
| `G-NORUST` | reconstrucción instrumentada sin Rust en ambos perfiles | producción completa |
| `G-R1` | seis componentes, paquete firmado, instalación/uso/actualización | R2/R3 |

Un pase en Windows no se extrapola a Linux. Un CI verde, un HTTP 200, un ejecutable que inicia o un diff vacío entre dos outputs no sustituye las puertas. El estado de cada afirmación será `PLANNED`, `EXECUTED_PASS`, `EXECUTED_FAIL` o `NOT_EXECUTED`, con artefacto y commit.

## IND-G — Compatibilidad y retiro

La migración conserva el contrato [ArgorixLang 1.0](language/current-v1.md) y aplica [compatibility.md](compatibility.md). Stage0 Rust queda congelado como referencia histórica; no recibe nuevas capacidades salvo lo indispensable para arrancar Core y cada adición tiene reemplazo Argorix. Se retira del camino operativo sólo después de ESP-024; se conserva fuente/hashes para arqueología y reproducción histórica, fuera del paquete R1 normal.

Cambiar backend, ABI, seed o formato estable requiere versión y guía de migración. Ningún artefacto declarativo de gobernanza, identidad o seguridad se convierte por esta arquitectura en garantía ejecutada.
