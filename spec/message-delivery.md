# Entrega de mensajes Argorix D1

Estado: contrato MAT-005, todavía no transporte implementado. D1 distingue aceptación, transporte, procesamiento y efecto; nunca usa “delivered” para significar todos ellos.

## D1-01 — Envelope e identidad

```text
MessageId = (run_id, authenticated_sender, stream_id, accepted_sequence)
Envelope = { id, sender, receiver, type_version, payload_digest,
             payload_bytes, authority_refs, causal_parent, deadline_tick }
```

El ID se asigna sólo al aceptar en la cola del sender. El receptor valida identidad/canal (MAT-012/015), esquema, digest, deadline y authority antes de admission. Un duplicado con mismo ID y bytes se descarta/acknowledge según estado dedup; mismo ID con bytes/authority distintos es `MessageIdCollision` y falla cerrado.

Dedup tiene scope `(receiver, run_id)` y retención mayor o igual al máximo de retry/replay soportado. Cuando expira, el producto no promete dedup histórica: sender debe iniciar nueva run/ID o usar protocolo de aplicación. La política de retención y checkpoint pertenece a MAT-010.

## D1-02 — Estados y transiciones observables

| Estado | Transiciones permitidas | Resultado observable |
| --- | --- | --- |
| `CREATED` | `ENQUEUED`, `REJECTED` | intento y motivo |
| `ENQUEUED` | `LEASED`, `CANCELLED`, `EXPIRED`, `DEAD_LETTER` | ocupación de mailbox |
| `LEASED` | `ACKED`, `RETRY_QUEUED`, `CANCEL_REQUESTED`, `DEAD_LETTER` | attempt/lease |
| `RETRY_QUEUED` | `LEASED`, `CANCELLED`, `EXPIRED`, `DEAD_LETTER` | reason y backoff lógico |
| `CANCEL_REQUESTED` | `CANCELLED`, `ACKED`, `DEAD_LETTER` | punto observado; efectos aparte |
| `ACKED` | terminal | handler terminó; no prueba efecto externo |
| `REJECTED` | terminal | nunca entró a mailbox |
| `CANCELLED` | terminal | no nuevos efectos después de observación |
| `EXPIRED` | terminal | deadline vencido antes de fase protegida |
| `DEAD_LETTER` | terminal | razón final y referencia al envelope |

Cada transición produce evento durable cuando exista persistencia; en el modelo produce event log. Estado desconocido o transición no listada falla `InvalidTransition`.

## D1-03 — Garantías por frontera

| Frontera | Garantía objetivo | Límite explícito |
| --- | --- | --- |
| mismo event loop | enqueue FIFO y una admisión por ID | crash antes de persistencia puede perder estado hasta MAT-010 |
| proceso local durable | at-least-once lease + dedup dentro de retención | handler puede repetirse tras crash |
| transporte autenticado | at-least-once, reorder buffer por stream, dedup | partición/replay requieren MAT-012/015 |
| efecto externo | `at-most-one dispatch` sólo si adapter+store atómico lo demuestran; por defecto estado incierto | **no exactly-once** sin protocolo/idempotency/reconciliation del sink |

“Effectively once” sólo puede declararse para un handler idempotente y ventana dedup fijada. No equivale a exactly-once global. Ack de mensaje no prueba que un pago, archivo o request externo ocurrió una sola vez.

## D1-04 — Reordenamiento, duplicación y partición

Reorder buffer retiene sequence mayor a `next_expected`; entrega sólo el siguiente. Gap vence en deadline y produce `GapTimeout`, no salta silenciosamente. Duplicado de un mensaje ACKED devuelve duplicate-ack sin reejecutar. Duplicado de LEASED no abre segundo lease. Partición conserva pending/retry hasta deadline/cuota; reconexión no reinicia sequence.

Payload corrupto, identidad no autenticada, grant revocado o schema incompatible produce `REJECTED`, evento y cero handler/effect. Un receiver no puede “arreglar” el envelope adivinando campos.

## D1-05 — Efectos y crash windows

Estados de efecto: `PROPOSED` → `AUTHORIZED` → `DISPATCHED` → `COMPLETED` | `FAILED` | `UNCERTAIN`. Crash antes de `DISPATCHED` permite retry. Crash después de `DISPATCHED` y antes de prueba terminal produce `UNCERTAIN`; el runtime consulta idempotency key/status del sink o solicita resolución humana según política. Nunca reintenta automáticamente una operación no idempotente incierta.

Message ID puede alimentar idempotency key, pero el sink debe respetarla. Sin confirmación del sink, la evidencia dice `UNCERTAIN`, no “exactly once”.
