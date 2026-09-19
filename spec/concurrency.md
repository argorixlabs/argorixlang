# Semántica de concurrencia Argorix C1

Estado: contrato MAT-005 y modelo reducido; **no implementado en el scheduler Rust actual ni probado entre procesos**. C1 define un event loop lógico determinista con agentes concurrentes por interleavings explícitos. Un backend puede usar threads, procesos o async sólo si preserva estas transiciones.

## C1-01 — Eventos, causalidad y orden

Cada transición obtiene `event_id=(run_id, logical_tick, ordinal)` y registra `caused_by`. La relación happens-before incluye: orden de programa dentro de handler; enqueue antes de lease; lease antes de ack/retry; evento padre antes de mensajes hijos; autorización antes de dispatch; cancel request antes de cancel observado. Dos eventos sin relación causal pueden intercalarse en cualquier orden válido.

Garantías:

- FIFO por `(sender_identity, receiver_identity, stream_id)` para mensajes aceptados; no hay orden total entre senders;
- un handler ejecuta un quantum a la vez por actor en C1; actores distintos son conceptualmente concurrentes;
- scheduler de referencia usa round-robin determinista por orden de registro y un quantum por turno;
- equidad débil: actor continuamente runnable obtiene turno si no agotó presupuesto y el host progresa; no se promete latencia de pared;
- payload, identity, authority y causal parent son inmutables después de aceptación.

El scheduler Rust histórico procesa inmediatamente y no implementa este contrato. Sus tests son compatibilidad, no prueba C1.

## C1-02 — Presupuestos y backpressure

Cada mailbox fija `max_messages` y `max_bytes`; cada run fija ticks, mensajes, retries y bytes totales. `send` que excede cuota devuelve `Backpressure` observable y no crea mensaje ni incrementa secuencia aceptada. No hay drop silencioso, overwrite ni bloqueo sin deadline. El caller puede yield/reintentar dentro de su deadline o propagar el error.

Receptor lento conserva mensajes hasta límite; al alcanzarlo el productor recibe backpressure. Prioridad no evade cuotas y queda fuera de C1. Control messages usan reserva separada sólo si una versión posterior la especifica; no se inventa capacidad infinita para cancelación.

Estados de mailbox: `OPEN` → `DRAINING` → `CLOSED`. `DRAINING` rechaza nuevos mensajes y procesa los aceptados; `CLOSED` no tiene mensajes no terminales. Cerrar con mensajes exige `drain` o transición explícita de cada uno a `CANCELLED|DEAD_LETTER`, nunca pérdida.

## C1-03 — Cancelación, deadlines y cierre

Cancelación es cooperativa, idempotente y scoped a run/task/message. Un mensaje `QUEUED` se elimina y pasa `CANCELLED`. Uno `LEASED` recibe `cancel_requested`; antes de cualquier nuevo efecto el handler debe observarla y terminar `CANCELLED`. Si un efecto ya está `DISPATCHED`, cancelación no afirma rollback ni exactly-once: el efecto queda `COMPLETED|FAILED|UNCERTAIN` y requiere reconciliación.

Deadlines son ticks lógicos absolutos. Antes de lease y antes de dispatch se verifica `tick < deadline`; vencido produce `EXPIRED`. Tiempo externo se captura como input y se traduce a tick/decision registrada. Timeout no es éxito, ack ni cancelación retroactiva.

El scheduler no ofrece locks de usuario ni send bloqueante en C1, previniendo una clase de deadlocks. `await` sólo puede esperar IDs declarados; el runtime mantiene wait-for graph. Un ciclo se rechaza `DeadlockDetected` con participantes. Espera por input externo puede quedar `WAITING_EXTERNAL` hasta deadline, no deadlock ficticio.

## C1-04 — Fallos y propagación

Error de handler se clasifica `retryable|permanent|cancelled|resource|authorization`. Retryable reencola con attempt incrementado hasta `max_attempts`; al agotarse pasa `DEAD_LETTER`. Permanent/authorization no reintentan por defecto. Crash con mensaje leased revoca lease y reencola; por ello el handler puede ejecutarse más de una vez.

Un child failure no cancela automáticamente siblings: la supervision strategy declarada elige `one_for_one|fail_fast|collect`. C1 de referencia usa `one_for_one`; otras estrategias requieren casos normativos. Error nunca amplía authority ni se convierte en ack.

## C1-05 — Determinismo y replay

Para replay se registran: fuentes/config hashes; registro de actores; IDs y envelopes; orden de decisiones del scheduler; ticks/deadlines; inputs remotos y autenticación; random/clock; respuestas model/tool; decisiones de policy/capability; intentos; faults/crashes/cancel; y estados de efectos. En modo replay no se contacta host/model/tool: se consumen entradas registradas y se comprueba el siguiente evento esperado.

Misma fuente, estado, input log y schedule producen la misma proyección semántica. Timestamps físicos, PID y paths se excluyen sólo si la normalización versionada lo dice. Una divergencia de evento, authority, state o effect es fallo, no se vuelve a grabar automáticamente.

La suite `tests/models/scheduler/` explora interleavings pequeños y reproduce schedules por lista de acciones, sin sleeps. Eso prueba sólo el modelo secuencial reducido; no prueba threads, red, persistencia, transporte o aislamiento reales.

## C1-06 — Autoridad y memoria

Cada envelope transporta una referencia a grants verificados, nunca material secreto. Authority de un child debe ser subconjunto de la authority efectiva del handler; duplicar, reordenar, retry, replay, cancelar o fallar no la amplía. Cada efecto vuelve a verificar capability según E1 justo antes de dispatch.

Payloads son valores inmutables o handles read-only cuya vida cubre la cola; un mutable handle M1 no cruza mailbox. Enqueue reserva bytes antes de publicar; dequeue/terminal libera la cuota exactamente una vez. Transferencia entre procesos serializa valores con esquema/version, no direcciones ni borrow tokens.
