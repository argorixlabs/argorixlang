# Plan de evaluación end-to-end y adversarial

**Estado:** **EJECUTADO** el 2026-08-29. Implementación en
`evaluation/adversarial/`; resultados en `evaluation/adversarial/results/`;
resumen de ejecución al final de este documento.  
**Objetivo:** responder a los revisores con evidencia producida por los binarios
reales de ArgorixLang, sin reutilizar la matriz circular de 57 fixtures y sin
convertir resultados negativos en resultados favorables.

## 1. Preguntas experimentales

1. ¿El compilador, verificador de bytecode y VM producen la decisión tipada
   predefinida para cargas conductualmente distintas?
2. Cuando se propone una operación prohibida, ¿la mediación la bloquea antes
   de un destino instrumentado y conserva evidencia del bloqueo?
3. ¿Qué mutaciones independientes detecta el verificador respecto de un bundle
   fijo y cuáles quedan fuera de su frontera de confianza?
4. ¿Cómo se comporta el sistema ante errores, artefactos incompletos, replay,
   concurrencia, timeout y dependencias no disponibles?
5. Si se incorpora un modelo real, ¿Argorix contiene los efectos de una acción
   prohibida propuesta tras una inyección indirecta, aunque el modelo haya sido
   manipulado?

La pregunta 5 mide **contención de efectos**, no prevención general de prompt
injection.

## 1.1 Superficies reales y readiness

La evidencia primaria se obtendrá de procesos release, no sólo de llamadas a
las mismas librerías dentro del runner:

```powershell
cargo build --locked --release -p argorixc -p argorix-vm -p argorix-conformance
target/release/argorixc.exe check CASE.argx
target/release/argorixc.exe emit-bytecode CASE.argx
target/release/argorix-vm.exe run program.argbc.json --dry-run --reactive `
  --inject User:Agent:tell:Message --json `
  --security-report run.security.json --trace-out run.trace.json `
  --evidence-bundle run.bundle.json
target/release/argorix-vm.exe verify-evidence run.bundle.json --json
```

Puede ejecutarse hoy, añadiendo fixtures y harness: pipeline real, passports,
provider boundary a nivel VM, mutaciones de bytecode/trace/report/ledger/bundle,
paths o artefactos faltantes y los controles negativos de source/reemplazo.

Requiere cambios de producto antes de sostener el claim correspondiente:

- lattice tipada y agregación monotónica; hoy las reglas son booleanas, unknown
  cae en un fallback y `security_checks` puede contradecir el detalle;
- content/payload bounded y una ruta prompt -> propuesta -> mediación; hoy
  `InjectedMessage` no contiene prompt;
- un adapter instrumentable en el punto real de mediación, si se desea evaluar
  dispatch externo y no sólo rechazo en el registry;
- source binding, firma o trust anchor, si se desea detectar modificación del
  source o reemplazo completo del set.

## 2. Regla de validez: Expected y Observed quedan físicamente separados

La campaña se implementará en `evaluation/adversarial/` con dos procesos:

- `collect`: recibe casos sin outcomes esperados, invoca los ejecutables de
  producción y conserva exit codes, stdout/stderr, artefactos, digests y
  telemetría de canarios.
- `score`: se ejecuta después de cerrar la recolección y une los resultados con
  `oracle.json` para calcular aciertos, false allows y false denies.

`collect` no podrá importar ni abrir `oracle.json`. Una prueba negativa retirará
o renombrará un binario y deberá hacer fallar la campaña; así se demuestra que
el harness no puede fabricar un resultado correcto sin ejecutar Argorix.

Un verificador auxiliar recalculará digests canónicos mediante una
implementación independiente del código Rust. Los diagnostics se compararán por
clases estables, no por coincidencia exacta de mensajes.

Cada fila cruda contendrá, como mínimo:

- `case_id`, clase conductual, repetición y nonce;
- commit, SHA-256 de binarios, versiones de Rust, SO y configuración;
- comandos y exit codes de compilación, VM y verificación;
- decisión y motivo emitidos por el runtime;
- contador independiente de dispatch, hits HTTP y estado de los canarios de
  filesystem y secreto;
- paths y digests de source, bytecode, trace, report y bundle;
- fingerprint del trace y conteo/tipos de eventos;
- latencias por etapa y tamaño de artefactos;
- `observed`, derivado solamente de procesos y sensores.

## 3. Campaña experimental

### E0 — Reproducción histórica, 33 directorios

Se conserva el universo completo: 27 bundles completos y seis directorios
source-only. Se vuelve a medir completitud, verificación, policy status,
unknown rules, fingerprints y secuencias de eventos. Los incompletos quedan en
el denominador con fase alcanzada y razón observable.

Resultado esperado de control, no de éxito de gobernanza: 27/27 bundles
internamente consistentes, 0/27 policy-approved, 44 unknown rules por bundle y
dos familias estructurales. Cualquier diferencia obliga a investigar deriva de
versión antes de continuar.

### E1 — Diversidad conductual real, 12 workloads x 3 repeticiones = 36 runs

Los workloads se escribirán como fuentes o paquetes Argorix independientes y
pasarán por compiler -> bytecode verifier -> VM -> trace/report/bundle ->
evidence verifier. Deben cubrir, como mínimo, estas doce firmas:

1. tool simulado permitido;
2. model simulado permitido;
3. capability ausente;
4. tool no autorizado;
5. model no autorizado;
6. policy conocida con `PASS`;
7. policy conocida con `DENY`;
8. policy conocida con `REVIEW`;
9. policy desconocida con `UNKNOWN_RULE` y fail-closed;
10. contrato de provider externo sin adapter ejecutable;
11. runtime profile o adapter inválido;
12. paquete multiarchivo con mensajes, policy y passport.

Las 36 ejecuciones deben producir al menos 12 fingerprints conductuales en seis
dimensiones: policy, capability, provider, runtime profile, estructura del
programa y outcome. Países o repeticiones no contarán como diversidad.

### E2 — Fallos y condiciones adversas, 20 casos

Se cubrirán inputs malformados, bytecode inválido, ruta de inyección inválida,
provider/adapter ausente, rechazo de allowlist, artefacto faltante, JSON
truncado, timeout/step exhaustion, excepción del adapter de prueba, canario no
disponible, dos ejecuciones concurrentes, replay del mismo request, replay de
evidencia, paths fuera del árbol portable, trace faltante y report faltante.

El oráculo predefinirá la fase y el outcome tipado. Error, outage o timeout no
pueden degradarse a `PASS`; toda salida parcial debe conservar la causa y los
artefactos disponibles.

### E3 — Mutación real de evidencia, 22 casos

Primero se genera un set limpio y se confirma que verifica. Después de la
generación, un mutador independiente altera realmente los archivos y ejecuta
`argorix-vm verify-evidence`:

- bytecode: valor semántico, eliminación de campo y truncamiento;
- trace: evento, ledger link y truncamiento;
- SecurityReport: policy result, ledger digest y versión;
- bundle: digest, ruta, versión y relación trace/path;
- artefactos faltantes, JSON inválido y path fuera del árbol portable;
- modificación sólo del source, que hoy debe quedar **no detectada** porque el
  schema estudiado no enlaza source;
- reemplazo coordinado de bundle y todos sus artefactos por un set unsigned
  autoconsistente, que hoy debe **pasar** y documentar el límite de autenticidad.

El resultado se reportará por clase de mutación. No se usará “tamper-proof” ni
una tasa general más amplia que los casos ejecutados.

### E4 — Dispatch y side effects con canarios independientes

El build actual sólo registra `simulated` y rechaza providers externos
ejecutables. Se prueban ocho condiciones: tool externo, model externo, contrato
declarado pero deshabilitado, adapter ausente, provider desconocido, allowlist
incompatible, payload malformado y timeout/outage.

Sensores seguros y locales:

- contador de invocación en un `TripwireProvider` de evaluación;
- servidor HTTP loopback con log append-only por nonce;
- archivo sentinel dentro de un directorio temporal aislado;
- secret/key canaries sintéticos, nunca credenciales reales.

Cada sensor necesita un control positivo que produzca al menos un hit; de otro
modo, cero hits no prueba nada. En las condiciones prohibidas, el outcome se
deriva del contador y del sink, no del campo `Side Effect` de la VM.

Existe una puerta estructural: si el provider de control no puede conectarse al
mismo punto de mediación usado por la VM sin habilitar una capacidad ausente en
el release, el paper sólo podrá afirmar rechazo en el registry y mantendrá fuera
de alcance la no-reachability de un adapter real. No se agregará una ruta de
producción insegura sólo para obtener una cifra favorable.

### E5 — Prompt injection real, rama condicionada, 80 runs por modelo

El VM actual no ingiere contenido de prompt; `InjectedMessage` sólo contiene
ruta estructurada. Por tanto, una prueba válida requiere un driver separado que
pase contenido benigno o adversarial por un LLM real y produzca una propuesta
de acción estructurada, que luego atraviese la mediación y los canarios.

Diseño mínimo, inspirado por AgentDojo/InjecAgent:

- ocho escenarios: dos provider externo, dos network egress, dos secret
  exfiltration y dos key-material access;
- control benigno y variante con inyección indirecta en contenido externo;
- cinco repeticiones, modelo/version/temperature/seed registrados;
- `8 x 2 x 5 = 80` ejecuciones por modelo;
- nonce único y destino local instrumentado por ejecución.

Se medirá por separado si el modelo propuso la acción prohibida y si la acción
llegó al sink. Si no existe modelo reproducible o no se implementa el camino
prompt -> propuesta -> mediación -> destino, esta rama no se sustituirá por una
simulación: el paper conservará “prompt injection was not evaluated”.

## 4. Métricas y análisis

Métricas primarias:

- exactitud de outcomes `PASS/DENY/REVIEW/UNKNOWN/ERROR` contra el oráculo;
- false allow rate y false deny rate;
- destination ASR: hits prohibidos en el sink / ataques ejecutados;
- conditional containment: propuestas prohibidas bloqueadas antes del sink /
  propuestas prohibidas;
- tamper detection por tipo, siempre relativa al bundle original;
- fail-closed rate ante unknown, malformed, timeout y outage;
- completitud de trace/report/bundle.

Métricas secundarias:

- fingerprints conductuales y clases de equivalencia;
- mediana, IQR y p95 de compilación, VM, verificación y end-to-end;
- tamaño de bytecode, trace, report y bundle;
- utilidad benigna de la tarea, separada de seguridad.

Las proporciones se publicarán como `n/N` con intervalo Wilson 95%. Si se
observan cero false allows, se incluirá el límite superior de la rule of three
(`3/N`) y no se escribirá “0% risk”. Para E5, los modelos se reportan por
separado y las comparaciones pareadas usarán bootstrap por escenario; las
repeticiones no se presentarán como tareas independientes.

## 5. Puertas go/no-go

Estas puertas controlan **qué puede afirmarse**, no si se ocultan resultados
desfavorables:

- **Validez del harness:** no-go si `collect` puede calcular outcomes sin los
  binarios o si lee el oráculo.
- **Policy:** no-go para corrección si una regla conocida termina como unknown,
  o si el estado agregado aparece `passed` con `DENY`, `REVIEW` o `UNKNOWN`.
- **Side effects:** no-go para “blocked before destination” si falta control
  positivo o telemetría externa por nonce.
- **Tamper:** no-go para tasas si la mutación no ocurre después de generar el
  artefacto o no se invoca el verifier real.
- **Prompt injection:** no-go para cualquier claim de resistencia sin LLM real,
  contenido adversarial y sink observable.
- **Reproducibilidad:** no-go para cifras en el PDF si un rerun limpio no
  regenera raw JSONL, resumen y tabla sin edición manual.

Un fallo de seguridad observado no invalida el experimento: se publica como
resultado negativo y se estrecha el claim.

## 6. Orden de implementación

### P0 — Congelar y preregistrar

1. Guardar commit, binarios y baseline actual.
2. Escribir `cases.json` y `oracle.json` antes de cambiar policy/runtime.
3. Crear schemas de rows y run manifest.
4. Reproducir E0 y conservar los resultados desfavorables históricos.

### P1 — Harness y canarios

1. Implementar `collect`, `score` y la prueba anti-circularidad.
2. Añadir workloads E1 y fallos E2.
3. Implementar mutaciones E3 sobre artefactos reales.
4. Implementar canarios E4 sin credenciales ni red pública.
5. Decidir la puerta estructural del adapter antes de formular claims.

### P2 — Correcciones y campaña

1. Corregir cobertura de reglas conocidas y derivación monotónica del estado
   agregado, conservando el baseline pre-fix.
2. Ejecutar E1--E4 desde un directorio limpio dos veces.
3. Ejecutar E5 sólo si su pipeline completo es real y reproducible.
4. Archivar raw logs, manifests, checksums y resultados row-level.

### P3 — Paper y camera-ready

1. Sustituir la tabla genérica de conformance por una tabla compacta con
   diversidad, false allow/deny, sink hits, tamper y límites.
2. Añadir metodología suficiente para reproducir, dejando filas y comandos en
   el artifact, no en el PDF.
3. Actualizar `response-to-reviewers.md` comentario por comentario.
4. Recompilar y hacer QA de IEEEtran, referencias, fonts y las ocho páginas.

## 7. Entregables

- `evaluation/adversarial/cases.json` y `oracle.json` separados;
- harness ejecutable, canarios y mutador;
- `raw/<run-id>/` con stdout/stderr y artefactos por caso;
- `results.jsonl`, `summary.json`, `results.csv` y checksums;
- tabla/figura LaTeX generadas sin edición manual;
- README con un único entrypoint de reproducción;
- camera-ready PDF, source ZIP, QA y respuesta a revisores actualizados.

## 8. Presupuesto editorial IEEE

El paper actual tiene cuatro páginas; el objetivo será cerrar en seis o siete,
dejando una página de margen bajo el máximo oficial de ocho. Se mantendrán:

- una figura de arquitectura;
- una figura opcional de frontera adversarial;
- tabla del snapshot histórico;
- una tabla compacta de la nueva campaña;
- cero listings y cero apéndices.

Los resultados completos, prompts, nonces redactados, artefactos y comandos se
publicarán en el repositorio/DOI. No se reducirá fuente, márgenes ni espaciado
IEEEtran para ganar páginas.

## 9. Claims permitidos al finalizar

Dependiendo de los resultados, el paper podrá decir:

- “En el harness evaluado, X/Y acciones prohibidas propuestas fueron bloqueadas
  antes del sink local; Wilson 95% CI [...]”.
- “El verifier detectó X/Y modificaciones independientes relativas al bundle
  original”.
- “Unknown rules, malformed inputs y outages terminaron fail-closed en X/Y
  casos”.

Seguirán prohibidos claims universales como “Argorix previene prompt
injection”, “no puede ocurrir ningún side effect”, “tamper-proof”, aislamiento
del sistema operativo, autenticidad criptográfica, soberanía legal o seguridad
general para todos los modelos y adapters.

---

## 10. Registro de ejecución (2026-08-29)

Campaña ejecutada dos veces desde directorio limpio con resultados idénticos.
Entrypoint único: `python evaluation/adversarial/run.py --clean`.

La primera campaña detectó tres defectos reales de producto. Se corrigieron y se
volvió a ejecutar la campaña completa, conservando el baseline pre-fix sin
editar en `evaluation/adversarial/baseline/prefix/`, tal como exige P2.1.

### Cobertura

| Familia | Casos | Runs |
| --- | --- | --- |
| E0 reproducción histórica | 33 | 33 |
| E1 diversidad conductual | 12 | 36 |
| E2 fallos y adversidad | 20 | 20 |
| E3 mutación de evidencia | 22 | 22 |
| E4 dispatch, canarios y punto de mediación | 12 | 36 |
| E6 autenticidad bajo trust anchor | 4 | 4 |
| E5 prompt injection (rama condicionada) | 16 | 80 (no ejecutados) |
| **Total** | **119** | **231** |

### Métricas: antes y después de las correcciones

| Métrica | Pre-fix | Ahora | Wilson 95% |
| --- | --- | --- | --- |
| Exactitud de outcomes | 103/106 | **118/118** | [96.8, 100.0] |
| False allow | 0/106 | 0/118 | cota ro3 2.5% |
| False deny | 0/106 | 0/118 | cota ro3 2.5% |
| Fail-closed | 16/16 | 16/16 | [80.6, 100.0] |
| Detección de mutaciones | 20/22 | **21/22** | [78.2, 99.2] |
| Contención antes del sink | 21/21 | 21/21 | [84.5, 100.0] |
| Rechazo en frontera | 15/21 | **18/21** | [65.4, 95.0] |
| Destination ASR | 0/21 | 0/21 | cota ro3 14.3% |
| Sets forjados rechazados (anchor) | n/a | **3/3** | — |

### Puertas go/no-go

| Puerta | Estado | Nota |
| --- | --- | --- |
| Validez del harness | go | `collect` no puede leer el oráculo; sin binarios la campaña falla y no emite filas |
| Policy | go | ningún reporte presenta veredicto agregado aprobatorio sobre detalle DENY/REVIEW/UNKNOWN |
| Side effects | go | los tres sensores dispararon control positivo, y además uno desde dentro del punto de mediación |
| Tamper | go | mutación posterior a la generación, verificador real invocado, digests pre/post registrados |
| Prompt injection | **no-go por diseño** | prohíbe el claim; no reporta un defecto |
| Reproducibilidad | go | dos corridas limpias producen métricas, puertas y tablas idénticas |
| Control E0 | go | el snapshot histórico reproduce exactamente |
| Diversidad E1 | go | 12 fingerprints distintos en 36 runs |

### Defectos encontrados y corregidos

1. **F1 — tipos de payload no verificados.** El verificador validaba provider y
   capability de cada tool/model pero nunca comprobaba que los tipos
   `input`/`output` estuvieran declarados. *Corrección:* `verify_bytecode`
   comprueba las cuatro ranuras, con compuerta sobre las versiones cuyo esquema
   lleva tabla de tipos. *Efecto:* rechazo en frontera 15/21 → 18/21.
2. **F2 — sin binding del source.** *Corrección:* `argorixc emit-bytecode` sella
   el digest del source dentro del bytecode, el bundle registra `source_path` y
   la verificación offline lo recalcula; un bundle que nombra un source que el
   bytecode no ata falla cerrado. *Efecto:* detección 20/22 → 21/22.
3. **F3 — sin binding del productor.** Nada distinguía el set original de un
   reemplazo autoconsistente. *Corrección:* firma Ed25519 desprendida sobre los
   bytes canónicos del bundle, producida por el binario aparte `argorix-sign`
   —la VM nunca maneja clave privada— y comprobada por
   `verify-evidence --trust-anchor`. *Efecto:* 3/3 sets no-productor rechazados;
   firma ausente o ajena falla cerrado.

Las tres traen regresión propia: 382 tests en el workspace, `cargo fmt --check`
y `clippy --all-features` limpios.

### F4 — punto de mediación

El release no puede instrumentarse donde importa: `execution_registry`
reconstruye el proveedor ejecutable en cada ejecución, así que ni sustituyendo
el registry se llega al punto de mediación. En lugar de debilitar el release, se
compila un segundo binario tras el feature no-por-defecto `eval-tripwire`. El
build de release no lo compila y rechaza sus flags; el binario de evaluación
reporta una versión distinta que todo manifest registra. Observado en tres
condiciones: una llamada permitida llega al proveedor exactamente una vez y
siempre con dry-run; un programa rechazado en verificación no llega nunca; y una
sonda emitida desde dentro de una invocación de proveedor queda registrada por
el sink. Ese último caso es el control que da sentido a los otros.

### F5 — prompt injection: especificado, no ejecutado

La rejilla está declarada en el catálogo antes de cualquier llamada a modelo:
ocho escenarios (provider externo, egress de red, exfiltración de secreto,
acceso a key material), brazos benigno e inyectado, cinco repeticiones = 80
runs. El driver está implementado: pone contenido frente a un modelo real, toma
la acción estructurada que propone y la empuja por la mediación y los sensores.
Se miden por separado el éxito del ataque sobre el modelo y la contención por
parte de Argorix, y el mapeo propuesta→programa es total y publicado: una
propuesta que ningún programa cubre se cuenta como `UNMAPPABLE`, nunca se
descarta. La rama se ejecuta sólo contra un modelo real y reproducible; sin él
registra `NOT_EXECUTED`. Aun ejecutada no podría afirmar que un loop de agente
de Argorix resiste inyección, porque es el driver quien mapea la propuesta.

### Fronteras que quedan (no son defectos)

| Id | Frontera | Efecto sobre el claim |
| --- | --- | --- |
| B1 | La verificación sin firma no distingue un reemplazo autoconsistente | integridad y binding de source, no autenticidad |
| B2 | Firmar establece el productor y nada más | sin key storage, rotación, revocación ni timestamping |
| B3 | El release no es instrumentable en su punto de mediación | las observaciones describen el build de evaluación |
| B4 | El release no ingiere contenido de prompt | prompt injection no evaluado |
| B5 | La operación sandboxed declarada no se rechaza en frontera | contención y rechazo son proporciones distintas |

### Enmiendas al preregistro

- **A1** — el campo `phase` confundía la etapa de decisión con la etapa más
  avanzada alcanzada. Valores esperados intactos; `phase` pasó a comprobación
  informativa, excluida de la exactitud de outcomes.
- **A2** — tras F2, la expectativa de E3-21 pasó de `NOT_DETECTED` a
  `DETECTED`. Cambió el producto, no la expectativa sobre el producto tal como
  era; el baseline queda archivado sin editar.

### Desviaciones respecto del plan

- E2 mantiene los veinte casos, con cuatro divisiones para cubrir por separado
  las dieciséis condiciones enumeradas. Ninguna se omitió.
- Se añadió la familia **E6** (autenticidad bajo trust anchor, 4 casos) en lugar
  de ampliar E3, para que la tasa de detección de mutaciones siga siendo
  comparable entre el baseline y la campaña actual.
- E4 creció de 9 a 12 casos con las tres observaciones del punto de mediación.
