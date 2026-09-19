# Arquitectura de bootstrap independiente

Decisión ESP-003: adoptar **stage0 Rust congelado → backend C transitorio → bootstrap Argorix → backend nativo Argorix**. El backend C reduce el salto inicial, pero tiene fecha de retiro estructural: no aparece en el grafo normal R1. Reescribir toda la toolchain en C, conservar un backend Rust o distribuir sólo un binario Rust estático fueron rechazados porque trasladan u ocultan la dependencia.

## Etapas y artefactos

```text
B0  stage0 Rust congelado + extensión Core temporal
      │ genera C sólo para arrancar las primeras fuentes compiler/*.argx
      ▼
B1  stage1 Argorix/C + runtime mínimo C inventariado
      │ compila las mismas fuentes Argorix; produce stage2
      ▼
B2  stage2 Argorix/C → stage3 Argorix/C + comparación estable
      │ demuestra self-hosting inicial, todavía no independencia nativa
      ▼
N1  backend nativo Argorix emite ELF x86-64 y PE/COFF x86-64
      │ linker de sistema declarado; corpus y ABI por plataforma
      ▼
N2  stageN construye stageN+1 sin Rust ni compilador C
      │ recompila seis componentes esenciales y stdlib
      ▼
R1  semilla firmada + fuentes .argx + manifest/lock + shims + evidencia
```

| Etapa | Rust | Compilador C | Backend | ¿Cuenta como R1? |
| --- | --- | --- | --- | --- |
| B0 | permitido sólo en entorno arqueológico | opcional | Rust/temporal | No |
| B1 | prohibido para stage1, pero B0 lo produjo | requerido y fijado | C | No |
| B2 | ausente del build stage2→3 | requerido y fijado | C | No |
| N1 | prohibido | sólo para comparación transitoria, no dependencia del objeto nativo | ELF/PE-COFF | No |
| N2/R1 | prohibido y sensor activo | prohibido en camino normal | nativo Argorix | Sí, si pasan todas las puertas |

## Grafo de componentes

`compiler` consume fuente/manifest/lock y produce IR interna, bytecode versionado u objeto nativo. `verifier` valida antes de `runtime`. `runtime` sólo obtiene efectos por ABI host y capacidades explícitas. `conformance` ejerce compiler/verifier/runtime y conserva controles negativos. `signer` vive fuera del runtime. `package_manager` resuelve contenido fijado y alimenta al compiler; no ejecuta scripts arbitrarios.

Los seis se implementan en `.argx`. Shims nativos se ubican en un directorio de plataforma separado, nunca contienen parser, checker, lowering, scheduler, policy engine, evidence engine o resolver de paquetes. El manifest de release asigna cada símbolo del shim a una capacidad permitida.

## Formatos y estabilidad

| Formato | Etapa que lo congela | Compatibilidad |
| --- | --- | --- |
| fuente `.argx` y `argorix.toml` | MAT-003/ESP-004 | estable por versión |
| AST/IR de compilador | ESP-007/013 | interno, no ABI pública |
| bytecode y verifier | ESP-007/018 | versión explícita, rechazo de unknown |
| ABI host/memoria | ESP-005 + MAT-004 | por plataforma y versión |
| ELF/PE-COFF emitido | ESP-016/017 | ABI del destino fijada |
| manifest/lock/SBOM | ESP-020/022 + MAT-016 | estable y firmado |
| trace/report/evidence | ESP-020 + MAT-018 | versión, fuente y ancla explícitas |
| seed manifest | ESP-024/025 | inmutable por release |

No se promete estabilidad de layouts AST/IR ni igualdad byte a byte antes de que su tarea fije canonicalización. La comparación de bootstrap registra qué regiones normaliza; nunca normaliza instrucciones, símbolos, constantes, imports o metadatos de seguridad.

## Perfiles de host

- `compiler-host`: lectura confinada a paquete/dependencias, escritura a output temporal, memoria/CPU acotadas, reloj sólo para metadatos no reproducibles, entropía sólo para operaciones separadas de firma; invocación del linker permitido como operación nominada.
- `agent-runtime`: no hereda capacidades del compilador. Red, FS, procesos, secretos y herramientas se conceden por operación y quedan mediados/observados.
- `verification-host`: sólo lectura de artefactos y ancla; no red ni efectos de programa.

Estas separaciones implementarán TB-01/02/03/05/08/11. Son requisitos de diseño, no garantías presentes. Un linker exitoso no autoriza ejecutar el resultado; verificación y política siguen siendo pasos separados.

## Bootstrap reproducible y fallo cerrado

Cada etapa produce un manifest con inputs, hashes, herramientas, entorno, comandos, outputs y comparación. Si falta una dependencia, firma, hash, versión o sensor, la etapa termina `NOT_EXECUTED` o `FAIL`; no reutiliza un artefacto de cache no declarado. Los directorios de build son nuevos y el ensayo final bloquea red tras preparar inputs firmados.

Los controles positivos insertan, por separado: invocación `rustc`, `.rlib`, proceso/servicio Rust, emisión `.rs`, acceso a cache Cargo y compilador C en R1. Cada sensor debe detectarlos. La política legible por máquina y su validador se encuentran en `bootstrap/independence-policy.json` y `bootstrap/validate-independence-policy.py`.

## Destinos y linker

- Ubuntu 24.04 x86-64: ELF64, System V AMD64; linker exacto se fija en el candidato.
- Windows 11 x86-64: PE/COFF, Windows x64 ABI; `link.exe` o `lld-link` exacto se fija en el candidato.

El linker es dependencia externa permitida porque combina objetos; no analiza Argorix ni implementa su semántica. Si se usa `lld`, su procedencia/binario se inventaría como dependencia no Rust de distribución o del host. Elegir linker definitivo, CRT y estrategia de TLS/crypto requiere evidencia en ESP-016/017/020; este contrato no inventa versiones aún no probadas.

## Semilla, actualización y recuperación arqueológica

La semilla normal es la generación Argorix anterior firmada por plataforma. Una actualización verifica manifest, versión, firma/ancla y rollback antes de compilar. Si todas las semillas se pierden, el procedimiento arqueológico puede reconstruir desde el stage0 Rust congelado en un entorno aislado y volver a producir una semilla auditada; esa excepción no forma parte del uso normal ni prueba por sí sola reproducibilidad diversa.

## Handoff

ESP-004 definió Core 0.1 y ESP-005 concretó/prototipó memoria, efectos y ABI secuencial. ESP-006 puede implementar el frontend temporal contra esos oráculos. Ninguna tarea puede marcar independencia hasta N2/R1 en ambos perfiles. Las selecciones aún abiertas —linker exacto, biblioteca criptográfica, CRT, canonicalización de objeto y formato final de seed— tienen propietario posterior y no son dependencias ocultas aceptadas.
