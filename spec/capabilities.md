# Capacidades, delegación y revocación Argorix A1

Estado: contrato MAT-006 y modelo reducido; **no implementado en la VM, identidad ni host actuales**. Los nombres de capability 1.0 son declaraciones de compatibilidad, no valores A1 no falsificables.

## A1-01 — Grant y solicitud

Un grant contiene al menos:

```text
grant_id, issuer_identity, subject_identity, audience/profile,
resource_kind, resource_scope, operations,
budget_total, budget_remaining, budget_unit,
issued_at, not_before, expires_at,
policy_version, revocation_epoch,
delegable, max_delegation_depth, parent_grant_id, lineage_digest,
conditions, status, format_version
```

`grant_id` es opaco y no adivinable; una representación portable está autenticada por issuer/ancla. Un handle local es no falsificable por construcción. El solicitante aporta identidad autenticada, grant handle/token, recurso exacto, operación, costo, nonce único, deadline y causal request ID. Strings, imports, metadatos, DID/passport no verificado o `approval granted` no crean autoridad.

Un grant es inmutable excepto contabilidad y estado monotónicos. Operaciones desconocidas, campos ausentes, versión no soportada o constraints no comprendidos producen `UNKNOWN`/rechazo, nunca defaults permisivos. El ciclo observable de un efecto usa `PROPOSED`, `AUTHORIZED`, `DISPATCHED`, `COMPLETED`, `FAILED` o `UNCERTAIN`; sólo el tercer estado confirma que la frontera de host recibió el dispatch.

## A1-02 — Delegación atenuada

Delegar requiere subject del parent autenticado, operación `delegate`, flag delegable y profundidad restante. El child cumple simultáneamente:

- operations ⊆ parent operations; nunca recupera una operación quitada por policy;
- resource scope igual o más estrecho (path/target set subset, no sólo prefix textual sin canonicalizar);
- audience/profile igual o más estrecho;
- `not_before_child >= not_before_parent` y `expires_child <= expires_parent`;
- budget child se reserva atómicamente del parent y no excede remaining;
- conditions child = parent conditions ∧ nuevas restricciones;
- lineage enlaza parent y toda revocación ancestral lo invalida;
- depth decrementa; el child no puede volver delegable si parent no lo es.

A1 no devuelve presupuesto reservado al revocar/cerrar un child; es conservador y evita double-spend/recovery ambiguo. Una versión posterior podría introducir reclaim explícito con ledger y nuevo policy epoch, nunca por borrar el child.

## A1-03 — Tiempo y expiración

Dentro de proceso, duración usa reloj monotónico del host. Tokens portables llevan UTC firmado más intervalo/error máximo de la fuente. Evaluación conservadora:

- válido sólo si `trusted_now - max_clock_error >= not_before`;
- no expirado sólo si `trusted_now + max_clock_error < expires_at`.

Por tanto la tolerancia nunca extiende autoridad. Perfil inicial objetivo: error máximo documentado ≤30 s para tokens interproceso; si falta tiempo confiable, excede ese error o rollback es detectado, resultado `UNKNOWN` y cero dispatch. El modelo usa ticks exactos (`max_clock_error=0`) y no demuestra reloj real.

## A1-04 — Revocación y policy changes

Revocar un grant incrementa `revocation_epoch` y afecta al grant y descendientes. Latencia soportada objetivo:

- local/in-process: antes del siguiente dispatch, sin transición autorizada adicional;
- transporte soportado: feed de revocación con edad máxima 1 segundo; si supera 1 s, toda operación afectada es `UNKNOWN` hasta refresh.

Una decisión `ALLOW` previa no reserva derecho a ejecutar. Antes de dispatch se vuelven a comprobar identidad, lineage, tiempo, policy version, revocation epoch, scope, operación, nonce y budget en una transacción lógica. Revocación/policy change entre evaluación y commit invalida el ticket.

Después de `DISPATCHED`, revocación no puede deshacer el mundo: se solicita cancelación si el adapter la soporta y se registra `COMPLETED|FAILED|UNCERTAIN`. No se afirma rollback. Operaciones long-running deben revalidar en checkpoints definidos; sin checkpoint soportado, su alcance/duración debe ser bounded por grant.

## A1-05 — Cache y fallo de autoridad

Cache key incluye grant/lineage digest, subject, audience, resource, operation, cost class, policy version, revocation epoch y time bucket. `ALLOW` cacheado es sólo hint para evaluación; **commit siempre consulta estado autoritativo local/fresco**. `DENY` puede cachearse hasta el menor expiry/policy change, pero sigue registrándose. `REVIEW` y `UNKNOWN` nunca se convierten ni cachean como permiso.

Si authority/policy/revocation/time no está disponible o fresh, la decisión es `UNKNOWN`. No existe fail-open por disponibilidad. La implementación puede ofrecer operaciones puramente locales que no requieren esa autoridad sólo si su effect signature es vacía.

## A1-06 — Decisiones y revisión humana

Resultados de evaluación: `ALLOW`, `DENY`, `REVIEW`, `UNKNOWN`. Sólo `ALLOW` produce ticket de commit corto y ligado a request. `REVIEW` exige aprobación separada por identidad distinta/rol permitido, bound a request+digest+scope+expiry y single-use; la aprobación genera una nueva evaluación, no muta REVIEW en booleano global. `DENY` y `UNKNOWN` no despachan.

Razones mínimas: `SubjectMismatch`, `OperationDenied`, `ScopeDenied`, `Expired`, `NotYetValid`, `Revoked`, `AncestorRevoked`, `BudgetExceeded`, `Replay`, `PolicyDenied`, `PolicyReview`, `PolicyUnknown`, `AuthorityUnavailable`, `StaleRevocation`, `VersionUnsupported`, `ConstraintUnknown`.

## A1-07 — Nonce, replay y presupuesto

Nonce/request ID se vincula al digest completo. Mismo nonce+digest puede recuperar la misma decisión no-dispatch, pero un commit es single-use. Mismo nonce con digest distinto es `ReplayCollision`. Budget se reserva en delegación y se consume atómicamente en commit; dos tickets concurrentes no pueden gastar la misma unidad. Fallo antes de commit no consume; resultado incierto de efecto sí consume porque hubo dispatch.

Eventos registran request/grant IDs no secretos, lineage digest, versions/epochs, decisión/razón, costo, ticket y estado de efecto. Nunca registran secreto o token bearer completo.

## A1-08 — Integración y límites

- E1 usa A1 para capabilities dinámicas; C1 transporta referencias/digests inmutables y obliga recheck antes de efecto.
- MAT-007 fija secretos/procedencia; MAT-012 identidad/keys; MAT-013 el punto de mediación; MAT-011 aislamiento host.
- Autenticidad criptográfica del token, storage durable, feed distribuido, clock real y enforcement no se prueban en MAT-006.

El modelo bajo `conformance/capabilities/` explora árboles de delegación y carreras lógicas de revoke/commit. No prueba que la VM Rust o un servicio remoto aplique A1.
