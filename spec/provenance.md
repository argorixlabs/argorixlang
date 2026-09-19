# Procedencia de paquetes, bootstrap y releases Argorix P1

Estado: contrato MAT-007; no constituye todavía una cadena de suministro de release implementada.

## P1-01 — Unidad verificable

Cada paquete/release vincula por digest SHA-256: fuentes completas, manifest normalizado, dependencias resueltas, lockfile, compilador/runtime, plataforma, comandos de build, policy profile, artefactos y resultados de pruebas. La declaración usa in-toto Statement v1/SLSA provenance cuando corresponda y firma separada Ed25519 mediante biblioteca mantenida.

Una firma sólo es aceptable si `key_id`, propósito `release|package|bootstrap`, algoritmo, tiempo y ancla coinciden con la política I1. Hash correcto sin ancla demuestra consistencia, no autoría. Firma extranjera, clave expirada/revocada al tiempo aplicable, subject digest distinto, dependencia no declarada o material incompleto rechazan promoción.

## P1-02 — Bootstrap independiente

Mientras el compilador independiente no exista, Rust y su toolchain forman parte explícita del trusted computing base de etapa 0. La cadena prevista es:

1. `stage0`: commit fuente + lockfile + toolchain Rust fijado construyen el compilador de transición;
2. `stage1`: stage0 construye el primer compilador Argorix autocontenido;
3. `stage2`: stage1 recompila las mismas fuentes Argorix;
4. se comparan artefactos normalizados stage1/stage2 y se ejecuta conformance en artefactos distribuidos.

Hasta completar ESP-017/023 y MAT-023, la procedencia debe declarar `bootstrap_dependency: rust`; no se permite etiquetar el release como independiente. Diferencias reproducibles se investigan; no se firma/promueve un resultado no explicado.

## P1-03 — Firma, rotación y revocación de release

La clave de release es distinta de identidad, desarrollo y pruebas; preferentemente hardware/offline y con aprobación dual. La rotación publica un trust-store versionado firmado por la clave anterior y, para recuperación, por ancla offline. Revocación bloquea artefactos nuevos y activa advisory para artefactos históricos según tiempo/motivo. Una nueva clave no rehabilita firmas de una clave comprometida.

CI produce materiales no promovibles; promoción requiere verificación independiente de digests, builder permitido, inputs completos, pruebas y firma. El verificador consume trust roots configuradas fuera del artefacto verificado.

## P1-04 — Evidencia y límites

La evidencia puede probar integridad y vínculo con una clave/ancla. No prueba que el código sea seguro, que el builder no esté comprometido o que el efecto observado haya ocurrido. Un host comprometido puede falsificar evidencias locales; se requieren transparencia/attestation externa en MAT-023/026. Material privado, bearer tokens y valores de secreto están prohibidos en provenance y logs.

