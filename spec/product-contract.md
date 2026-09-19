# Contrato de producto de ArgorixLang

Estado: **contrato de planificación MAT-001**. Fecha: 2026-09-18. Base de implementación observada: `5d73d66`. No equivale a funcionalidad entregada ni a aprobación de producción. Los criterios fijados aquí se revisan por cambios explícitos de alcance con pruebas y responsable.

Fuentes del contrato: [plan maestro](../PLAN_MAESTRO_ARGORIXLANG.md), [inventario ESP-001](../bootstrap/inventory.md), [README actual](../README.md), [evaluación histórica](../evaluation/adversarial/README.md). La matriz íntegra está en [requirements.json](requirements.json).

## Propósito y personas

ArgorixLang debe servir para escribir, construir y ejecutar comunicaciones entre agentes con tipos, autoridad explícita, límites de recursos y evidencia que un tercero pueda verificar. El producto final incluye compilador y runtime escritos en ArgorixLang, capaces de construir la entrega normal sin Rust. Su alcance de producción se limita a los perfiles probados.

Personas implicadas:

- **Desarrollador:** instala herramientas, escribe módulos, compila, prueba, depura y empaqueta.
- **Propietario de políticas:** define capacidades, delegación, revocación y criterios de revisión.
- **Operador:** configura destinos y secretos, ejecuta, observa, respalda, recupera y actualiza.
- **Agente:** envía/recibe mensajes tipados y solicita acciones dentro de autoridad concedida.
- **Verificador externo:** comprueba artefactos y resultados mediante anclas, fuentes, logs y sensores independientes.
- **Responsables técnicos:** mantienen lenguaje, backend, runtime, seguridad y releases. Los roles de la matriz asignan rendición de cuentas por requisito; MAT-029 deberá nombrar personas antes de R3.

## Tres entregas y criterio de cierre

| Entrega | Criterio necesario | Naturaleza de la entrega |
| --- | --- | --- |
| R1 — independiente | IND-001 a IND-004 y tareas ESP aplicables terminadas; bootstrap, binarios y herramientas reconstruidos en ambos perfiles sin Rust | Toolchain independiente validada, todavía no producción |
| R2 — funcional | Requisitos R1 y R2, tres aplicaciones de referencia con éxito, denegación y fallo observados | Producto funcional dentro del alcance acotado |
| R3 — producción | Requisitos R1–R3, pruebas prolongadas, validación independiente, beta externa y soporte | Producción sólo para perfiles y límites que pasaron |

Todos los requisitos marcados `mandatory` son obligatorios para su entrega y todas las posteriores. Una tarea cerrada no sustituye el test del requisito. Si una capacidad se elimina del alcance, debe cambiarse este contrato y explicarse la pérdida para usuarios; no se puede mover a «opcional» durante la evaluación sólo para conseguir un pase.

## Perfiles de plataforma

Los perfiles de aceptación iniciales son **Ubuntu 24.04 x86-64** (ELF/System V AMD64) y **Windows 11 x86-64** (PE/COFF/Windows x64), ambos con host de referencia de al menos 4 núcleos lógicos y 8 GiB de RAM. Esa cifra define el entorno de ensayo, no un mínimo de instalación demostrado. Imagen/digest exactos, versión de kernel/build, linker y bibliotecas permitidas se congelarán para cada candidato. El plan exige en ambos plataformas instalación, bootstrap, compilación, runtime, evidencia, recuperación y herramientas que correspondan a cada entrega. Otras plataformas permanecen sin soporte declarado hasta tener matriz y pruebas propias.

El entorno de R1 debe carecer de rustc, Cargo, crates, caches y servicios Rust disponibles para el build normal. Un linker de sistema y bibliotecas no Rust inventariadas pueden estar presentes. El backend propio debe producir objetos nativos; un transpiler C temporal sólo sirve antes del cierre de R1.

## Tres aplicaciones de referencia

| Aplicación | Éxito | Denegación | Fallo y recuperación |
| --- | --- | --- | --- |
| APP-01, agente local | Modelo propone acción permitida, herramienta se invoca una vez y llega resultado; evidencia une propuesta, autorización y efecto | Propuesta prohibida o contenido hostil se registra y no alcanza sink | Timeout o salida inválida produce fallo explícito y no consume más presupuesto del permitido |
| APP-02, dos agentes | Dos procesos autenticados intercambian al menos diez mensajes tipados con correlación | Emisor extranjero o revocado no ejecuta acción; causa verificable | Partición, duplicado y reinicio producen recuperación o fallo explícitos, sin éxito ficticio |
| APP-03, flujo persistente | Estado y presupuesto sobreviven reinicio y upgrade soportado | Backup/estado manipulado se rechaza antes de continuar | Corte durante commit o migración conserva estado válido; efecto incierto se reconcilia, sin repetición automática |

El perfil mínimo contiene un agente, un modelo y una herramienta para APP-01; dos procesos y diez mensajes para APP-02; diez transiciones y un efecto externo controlado para APP-03. No equivale a un límite máximo del producto. MAT-024 fijará presupuestos y capacidad adicionales antes de evaluar la release. Un proveedor de prueba sirve para CI; los casos que afirman ejecución real usan servidor/sensor externo y registran si se llamó a un modelo real.

La identidad debe autenticarse mediante un mecanismo definido en MAT-007/012. Campos de pasaporte, DID, jurisdicción o regulación que sólo sean metadatos no conceden autoridad. La cobertura MCP/A2A será una versión y superficie concreta fijadas al implementarlas, sin prometer todo el protocolo.

## Requisitos trazables

Cada fila apunta a uno o más tests cuyo procedimiento, resultado esperado, artefacto futuro y estado están en JSON. **Todos esos tests están PLANNED y sin evidencia producida.** `owner_role` es el rol responsable previsto; MAT-029 asignará responsables concretos.

| ID | Capacidad | Entrega | Rol responsable | Tests |
| --- | --- | --- | --- | --- |
| IND-001 | Autocompilación estable de compiler+stdlib+tools | R1 | Maintainer de compilador | TC-IND-001 |
| IND-002 | Build y uso sin Rust | R1 | Responsable de releases | TC-IND-002 |
| IND-003 | Backend nativo propio | R1 | Maintainer de backend | TC-IND-003 |
| IND-004 | Paridad de construcción Linux/Windows | R1 | Maintainer de plataforma | TC-IND-004 |
| LAN-001 | Semántica normativa y tipos generales | R2 | Responsable de lenguaje | TC-LAN-001 |
| LAN-002 | Compatibilidad versionada | R2 | Responsable de compatibilidad | TC-LAN-002 |
| LAN-003 | Dependencias y paquetes reproducibles | R2 | Responsable de paquetes | TC-LAN-003 |
| DEV-001 | Instalación y proyecto inicial | R2 | Responsable de experiencia de desarrollo | TC-DEV-001 |
| DEV-002 | Depuración y diagnósticos | R2 | Responsable de herramientas | TC-DEV-002 |
| DEV-003 | Documentación comprobada por terceros | R3 | Responsable de documentación | TC-DEV-003 |
| AGT-001 | Agente con modelo y herramienta gobernada | R2 | Responsable de runtime | TC-AGT-001, TC-APP-01-SUCCESS, TC-APP-01-FAULT |
| AGT-002 | Propuesta no confiable sin autoridad implícita | R2 | Responsable de políticas | TC-AGT-002, TC-APP-01-DENIED |
| AGT-003 | Concurrencia acotada y cancelación | R2 | Responsable de runtime | TC-AGT-003, TC-APP-01-FAULT, TC-APP-02-SUCCESS, TC-APP-02-FAULT |
| AGT-004 | Herramienta MCP real | R2 | Responsable de interoperabilidad | TC-AGT-004 |
| AGT-005 | Dos agentes autenticados en procesos distintos | R2 | Responsable de interoperabilidad | TC-AGT-005, TC-APP-02-SUCCESS, TC-APP-02-DENIED, TC-APP-02-FAULT |
| SEC-001 | Autorización final por operación | R2 | Responsable de seguridad | TC-SEC-001, TC-APP-01-SUCCESS, TC-APP-01-DENIED, TC-APP-02-DENIED |
| SEC-002 | Identidad autenticada y ciclo de claves | R2 | Responsable de identidad | TC-SEC-002, TC-APP-02-SUCCESS, TC-APP-02-DENIED |
| SEC-003 | Aislamiento real de efectos | R2 | Responsable de plataforma | TC-SEC-003, TC-APP-01-DENIED |
| SEC-004 | Evidencia verificable de fuentes y decisiones | R2 | Responsable de evidencia | TC-SEC-004, TC-APP-03-DENIED |
| SEC-005 | Protección y redacción de secretos | R2 | Responsable de secretos | TC-SEC-005, TC-APP-01-SUCCESS |
| SEC-006 | Artefactos con procedencia y firma | R3 | Responsable de supply chain | TC-SEC-006 |
| OPS-001 | Recuperación de estado y efectos inciertos | R2 | Responsable de persistencia | TC-OPS-001, TC-APP-02-FAULT, TC-APP-03-SUCCESS, TC-APP-03-DENIED, TC-APP-03-FAULT |
| OPS-002 | Upgrade, migración y rollback | R2 | Responsable de mantenimiento de estado | TC-OPS-002, TC-APP-03-SUCCESS, TC-APP-03-DENIED, TC-APP-03-FAULT |
| OPS-003 | Observabilidad y auditoría acotadas | R2 | Responsable de operaciones | TC-OPS-003, TC-APP-01-FAULT, TC-APP-03-SUCCESS, TC-APP-03-FAULT |
| OPS-004 | Presupuestos de capacidad | R3 | Responsable de rendimiento | TC-OPS-004 |
| OPS-005 | Estabilidad y recuperación prolongadas | R3 | Responsable de resiliencia | TC-OPS-005 |
| GOV-001 | Validación de seguridad independiente | R3 | Responsable de seguridad | TC-GOV-001 |
| GOV-002 | Beta externa reproducible | R3 | Responsable de comunidad | TC-GOV-002 |
| GOV-003 | Soporte, versiones y respuesta a incidentes | R3 | Responsable de release | TC-GOV-003 |

Las tareas que deben implementar cada requisito figuran en `implementation_tasks` del JSON. El validador comprueba que esas tareas existan en el plan actual, así como la cobertura de tests y de los tres resultados para cada aplicación.

## Reglas de seguridad y observación

- El sensor fuera de la VM debe demostrar un control positivo para cada tipo de efecto cuya ausencia se alegue. Distinguir propuesta, rechazo, despacho y efecto.
- Una falla de proveedor, red o almacenamiento no se cuenta como éxito. Las transiciones externas inciertas quedan identificadas hasta reconciliación.
- Autenticidad depende de anclas configuradas y política de claves; hash o firma por sí solos no demuestran cumplimiento legal ni seguridad universal.
- Retener falsos rechazos y resultados no ejecutados en denominadores. Un error del modelo no se convierte en «ataque contenido».
- Los artefactos citados por `evidence_expected` son destinos de una futura prueba sobre un commit candidato. El plan no crea esos resultados.

## Pisos de aceptación de producción

- Dos plataformas obligatorias; dos procesos y dos agentes en el caso remoto.
- Campaña mínima propuesta de 72 horas en cada perfil con carga representativa y fallos inyectados. MAT-025 fijará carga y oráculo antes de iniciar.
- Cero efectos prohibidos observados en casos obligatorios con sensores y controles positivos. La afirmación queda acotada a los casos medidos.
- Para el escenario mínimo APP-03 en hardware de referencia: cero eventos de estado **confirmados** perdidos y recuperación de servicio en cinco minutos. Un efecto externo incierto no se marca como confirmado y exige conciliación.
- Dos desarrolladores externos en dos proyectos distintos reproducen beta. Sin participantes reales, GOV-002 permanece pendiente.
- Límites configurables de tiempo, pasos, memoria, cola, entrada y gasto; los umbrales finales de rendimiento y capacidad se registran en MAT-024 antes del candidato.

## Capacidades posteriores y límites declarados

macOS y otras arquitecturas, WASM, registro público de paquetes, DID/VC verificados, blockchain y compatibilidad total MCP/A2A no son requisitos de R3. Pueden añadirse con su propio contrato y pruebas. La entrega no afirmará seguridad universal, cumplimiento legal automático, identidad verificada a partir de metadatos declarados ni ausencia de efectos por una traza interna únicamente.

## Control de cambios y ejecución

1. Crear una propuesta de cambio con motivo, requisitos afectados, nueva prueba, impacto de compatibilidad y usuario afectado. Cambiar JSON y este documento juntos.
2. Mantener ID estable al refinar un requisito; retirar/renombrar sólo con mapa de migración. No modificar umbrales después de ver resultados del candidato sin invalidar y repetir la evaluación afectada.
3. Un cierre requiere `status` de requisito y test sustentado en artefactos del mismo candidato. MAT-001 sólo define criterios; su cierre **no** marca ninguno de los 29 requisitos funcionales como implementado.
4. Verificar consistencia del contrato con `./spec/validate-product-contract.ps1`. Revisar manualmente que los oráculos no dependan sólo de la implementación probada.
5. Asignar persona propietaria y aprobar matriz de versiones, soportes y riesgos antes de R3.

MAT-002 cerró el [contrato de fronteras de confianza](trust-boundaries.md), sin demostrar aún las garantías del producto. [ESP-002](../tasks/espada/ESP-002.md) cerró el baseline histórico, MAT-003 fijó compatibilidad, ESP-003 arquitectura, MAT-004 memoria/efectos, MAT-005 concurrencia/entrega, MAT-006 el [modelo de capacidades A1](capabilities.md) y MAT-007 los contratos de [identidad](identity-and-keys.md), [secretos](secrets.md) y [procedencia](provenance.md). ESP-004 y MAT-029 están disponibles; estos modelos aún no implementan las garantías en el runtime Core.
