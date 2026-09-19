# Sistema de tipos y efectos Argorix Core E1

Estado: contrato MAT-004 con gate host secuencial prototipado en ESP-005; aún no implementado en stage0/runtime ni aplicado por aislamiento OS. Extiende M1 sin conceder capacidades por nombre o metadata.

## E-01 — Firmas y conjuntos de efectos

Una función declara `fn f(args) -> R ! {effects}`. Función pura usa conjunto vacío. Los efectos mínimos son `memory.allocate`, `memory.mutate`, `fs.read`, `fs.write`, `net.connect`, `process.spawn`, `clock.read`, `random.read`, `model.invoke`, `tool.invoke`, `sign.use` y `linker.invoke`. Un efecto desconocido es error estático, no permiso futuro.

Cada efecto de host exige un valor no falsificable `Capability<Kind, Scope>` y consume presupuesto. El conjunto inferido del cuerpo debe ser subconjunto del declarado; el caller debe declarar/propagar la unión de efectos transitivos. Polimorfismo de efectos queda fuera de E1. `Result` no borra efectos: una operación fallida sigue registrada como intento.

## E-02 — Autoridad dinámica

Antes de cada operación el host comprueba kind, scope/target, subject, operation, expiry/revocation, nonce/replay y presupuesto. Resultado `ALLOW` está ligado a esa invocación; `DENY`, `REVIEW`, `UNKNOWN`, regla ausente o verificador no disponible no despachan. Un string, import, provider, DID, passport, feature o aprobación textual no crea `Capability`.

Compiler-host, agent-runtime y verification-host tienen tipos de capability incompatibles. `linker.invoke` del compilador no permite `process.spawn` desde un agente. Los handles de secreto son opacos y no implementan igualdad, impresión o serialización.

## E-03 — Reglas de composición

- orden observable: izquierda a derecha y sólo ramas evaluadas;
- una función pura no llama una función con efectos;
- una closure/función almacenada conserva su fila de efectos;
- un callback host no obtiene capacidades del caller salvo parámetro explícito;
- error de tipos, bounds, borrow, cuota o autorización ocurre antes del despacho;
- el lowering conserva tipos, effect-set y capability operands; no puede reemplazarlos por flags booleanos;
- `unsafe`, FFI arbitraria, reflexión de handles y dynamic effect names no existen en E1.

## E-04 — Errores y evidencia

Errores estáticos: `UnknownEffect`, `EffectNotDeclared`, `CapabilityTypeMismatch`, `EffectLeak`, `PureCallsEffectful`. Errores dinámicos: `CapabilityMissing`, `ScopeDenied`, `Expired`, `Revoked`, `Replay`, `BudgetExceeded`, `PolicyDenied`, `HostUnavailable`. Cada intento de efecto genera evento con ID causal, sujeto, capability ID no secreto, operación, alcance redactado, decisión y estado `proposed|denied|dispatched|completed|failed|uncertain`.

La traza interna prueba sólo lo que observó el punto de mediación. Ausencia de efecto requiere sensor externo y control positivo según MAT-002. Un error posterior no convierte un efecto ya despachado en no realizado.

## E-05 — Propiedades del modelo reducido

1. **No amplification:** una transición no crea capabilities salvo operación `delegate` expresamente autorizada y atenuada por [A1](capabilities.md).
2. **Effect preservation:** el efecto observado pertenece al conjunto declarado y a una capability válida del contexto.
3. **Fail closed:** decisión distinta de ALLOW no produce `dispatched`.
4. **Separation:** perfiles de host no convierten capabilities entre sí.
5. **Memory before effect:** validación de argumentos/handles ocurre antes del despacho.

La suite reducida prueba `unauthorized_effect`, `effect_not_declared` y `memory_error_before_effect`. MAT-006 define delegación/revocación A1 y MAT-005 concurrencia C1 como modelos todavía no implementados; la mediación host efectiva permanece en MAT-011/013.
