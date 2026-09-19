# Política de secretos Argorix S1

Estado: contrato MAT-007; no existe todavía un broker de secretos aplicado por el runtime.

## S1-01 — Referencias, no valores

Fuente, AST, IR, bytecode, manifests, capacidades, mensajes, resultados, evidencia y trazas sólo contienen `secret_ref` opacas. El valor se resuelve exclusivamente en la frontera del adapter autorizado, después de autenticar identidad y validar capability A1. La VM y el modelo no reciben el valor.

Una referencia incluye provider lógico, nombre opaco, versión solicitada, propósito y clase; no incluye URI con credenciales. No implementa impresión, igualdad por valor, serialización ni interpolación.

## S1-02 — Acceso mínimo y leases

El broker comprueba sujeto, adapter, operación, destino, entorno, versión y deadline. Entrega un lease de corta duración, single-purpose y no delegable; evita materializar el secreto cuando el proveedor permite credenciales efímeras. Se prohíbe wildcard por defecto. El lease se invalida al expirar, revocar capability, rotar versión o cerrar contexto.

Cada acceso registra IDs, versión, propósito, decisión y digest no reversible; nunca valor, bearer token, private key, header completo ni cuerpo sensible. Contextos distintos no comparten handles o cache.

## S1-03 — Salidas y redacción

Los sinks de log, error, trace, evidencia y respuesta aplican schema allowlist primero y redacción secundaria. Cualquier campo marcado sensible se elimina, no se enmascara parcialmente. Un detector de canarios corre antes de persistir/publicar artefactos; coincidencia bloquea publicación, marca incidente y exige rotación. La redacción no convierte en seguro un host comprometido ni reemplaza aislamiento.

Errores no incorporan request/response raw. Dumps, debug y crash reports están deshabilitados para buffers sensibles. Donde el lenguaje/runtime lo permita, buffers son acotados y borrados best-effort; no se promete zeroization frente a copias del OS, GC o proceso comprometido.

## S1-04 — Ciclo e incidentes

- alta: valor generado fuera del repositorio y almacenado en provider aprobado;
- uso: por referencia y lease, con mínimo privilegio;
- rotación: nueva versión, transición acotada, revocación de la anterior y verificación del consumidor;
- filtración sospechada: bloquear publicación/efecto afectado, revocar/rotar, invalidar sesiones, preservar sólo evidencia redactada y revisar alcance;
- baja: revocación, retención según política y prueba negativa de acceso.

Claves/credenciales de prueba usan namespace y ancla exclusivos, no acceden a recursos reales y llevan marcador `ARGORIX_TEST_ONLY`. Nunca se promueven ni se reutilizan en producción.

## S1-05 — Límites

Argorix no puede proteger un secreto frente a kernel, administrador, debugger o adapter que ya controla su memoria. Un adapter que necesita plaintext constituye frontera confiada y debe aislarse en MAT-011/013. Ausencia de filtración en un scanner no prueba confidencialidad; se requieren sensores y pruebas de integración posteriores.

