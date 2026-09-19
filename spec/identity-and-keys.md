# Identidad y ciclo de claves Argorix I1

Estado: contrato MAT-007 y modelo reducido; **no implementado aún como autenticación del runtime**. MAT-012 implementará y probará este contrato.

## I1-01 — Identidad verificada

El identificador visible, nombre, pasaporte, país, jurisdicción, DID o VC son metadatos. Una identidad sólo queda `VERIFIED` para una operación cuando coinciden:

1. un `identity_id` estable y no reutilizable;
2. una clave pública activa registrada para esa identidad;
3. una prueba de posesión fresca sobre `challenge + audience + request_digest + expiry`;
4. una cadena hacia un ancla explícitamente confiada por el perfil local;
5. estado, propósito, algoritmo, tiempo y versión de política válidos.

Si falta cualquiera, el resultado es `UNKNOWN` o `DENY`; nunca se infiere identidad desde texto. La identidad autenticada se entrega a A1 como `subject_identity`. El perfil CLI/local inicial usa un trust store administrado por el operador y una clave Ed25519 guardada por un keystore del sistema o archivo protegido; DID/VC es una extensión resoluble por política, no requisito de arranque.

## I1-02 — Primitivas y formatos

Argorix no diseña primitivas criptográficas. El perfil inicial usa Ed25519 para firmas, SHA-256 para digests y CSPRNG del sistema para desafíos/identificadores, mediante bibliotecas mantenidas. El dominio firmado incluye versión, propósito, audiencia y digest canónico para impedir confusión entre protocolos. Algoritmo o versión desconocidos fallan cerrados.

Una seed determinista sólo se permite en fixtures marcados `TEST-ONLY`; está prohibida en releases y entornos operativos. Las claves privadas nunca entran en fuente, bytecode, bundle, argumentos CLI, variables impresas o trazas.

## I1-03 — Registro y estados

El registro versionado conserva `identity_id`, `key_id`, public key, algoritmo, propósito, `not_before`, `expires_at`, estado, issuer/anchor, sequence, predecessor, motivo y evidencia de aprobación. Estados de clave: `PENDING`, `ACTIVE`, `SUSPENDED`, `REVOKED`, `EXPIRED`, `COMPROMISED`, `RETIRED`.

- Alta: aprobación local explícita + prueba de posesión; activar es operación auditada.
- Rotación: nueva clave demuestra posesión y es aprobada por clave activa o procedimiento de recuperación. La anterior pasa a `RETIRED` tras overlap acotado; no se reactiva.
- Revocación/compromiso: monotónica e inmediata en store local; invalida sesiones/tickets no despachados y alimenta A1. Una firma histórica sólo puede validarse para tiempo de firma si el perfil conserva estado temporal y el motivo no exige invalidación total.
- Expiración: una clave expirada no inicia sesiones ni firma artefactos nuevos.
- Suspensión: reversible sólo mediante evento posterior autorizado; nunca restaura una clave revocada/comprometida.

## I1-04 — Recuperación

La recuperación crea una clave nueva y una nueva secuencia; jamás "desrevoca" la anterior. El perfil inicial exige un recovery policy pre-registrado con al menos dos factores independientes entre: clave offline de recuperación, aprobación de dos operadores distintos o ancla corporativa. El procedimiento fija ventana, identidad humana de aprobadores, challenge fresco y causa. Sin quorum o con ancla no disponible: `RECOVERY_PENDING`, cero autoridad.

Después de recuperar: revocar/comprometer claves previas, invalidar leases/sesiones, rotar secretos potencialmente alcanzados, publicar nuevo snapshot firmado y revisar eventos desde la última confianza conocida.

## I1-05 — Verificación y revocación

La evaluación devuelve `VERIFIED`, `DENY`, `REVIEW` o `UNKNOWN`. La decisión se ata a identidad, key, challenge, audience, request digest, policy epoch y expiry. Antes de emitir capability o despachar efecto se revalida estado fresco. Clave extranjera, expirada, aún no válida, revocada, comprometida, de propósito incorrecto, prueba repetida o ancla desconocida no autentican.

El store local es autoritativo para CLI/runtime local. Una futura federación requiere snapshot/feed autenticado y freshness explícito; indisponibilidad o rollback detectado produce `UNKNOWN`. Cache nunca prolonga validez ni revocación.

## I1-06 — Límites

Una clave válida prueba posesión, no nombre civil, ciudadanía, intención, seguridad del host ni honestidad. Si el host controla el proceso puede robar material accesible, alterar el binario o falsificar observaciones antes de una frontera externa. Si el ancla está comprometida puede introducir identidades falsas. La respuesta es detener emisión/efectos, reemplazar ancla fuera de banda, rotar descendientes y reconstruir desde evidencia externa; el software no puede autoatestiguar que el host comprometido está limpio.

