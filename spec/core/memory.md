# Memoria ejecutable Core M1/ABI-1

Estado: contrato ESP-005 sobre [M1](../memory-model.md). El prototipo Rust valida la lógica secuencial. El runtime C1 de ESP-008 implementa ya la parte que su perfil declara —handle canónico de 48 bytes, validación de arena, época, slot, generación, tipo, rango y permiso, y almacenamiento de `Buffer`/`Arena` desde ESP-009— con evidencia ejecutable en `tests/selfhost/runtime/` y `conformance/core_c/`. El backend nativo no existe todavía.

## Arena y asignación

Una arena activa tiene `arena_id:u64`, `epoch:u64`, owner no falsificable, `byte_limit:u64`, `used_bytes:u64`, `peak_used_bytes:u64`, límite de slots/tokens y tabla de asignaciones. Cada asignación conserva `slot:u32`, `generation:u32`, `type_id:u32`, element size, length, mutability, estado y bytes inicializados en cero.

Todas las sumas/productos usan checked arithmetic. `alloc` falla antes de mutar por overflow, límite de slots u OOM. Un slot liberado puede reutilizarse sólo con generation incrementada; un handle previo nunca revive. `release` incrementa epoch e invalida todo. Ningún error se representa como valor vacío exitoso.

## Handle ABI lógico

`CoreHandleV1` posee exactamente 48 bytes en encoding canónico little-endian:

| Offset | Campo | Tipo |
| ---: | --- | --- |
| 0 | `arena_id` | u64 |
| 8 | `arena_epoch` | u64 |
| 16 | `slot` | u32 |
| 20 | `allocation_generation` | u32 |
| 24 | `offset` | u64 |
| 32 | `length` | u64 |
| 40 | `type_id` | u32 |
| 44 | `permission` | u8 (`1=read`, `2=read_write`) |
| 45 | `reserved` | 3 bytes cero |

El encoding no es layout Rust/C ni contiene dirección host. Reserved no-cero, permiso desconocido, type mismatch o longitud/rango inválidos se rechazan. Comparar handles compara los ocho campos lógicos; secretos/capabilities siguen sin igualdad pública.

## Préstamos y acceso

Cada asignación admite N read tokens o un write token. Leer por handle desnudo se permite sin write token; escribir por handle desnudo sólo sin tokens. Slice conserva arena/epoch/slot/generation/type y sólo estrecha rango/permiso. `free`, transfer y release exigen cero tokens. Token terminado no revive y no se serializa.

Acceso Core mediante `Handle<T>` adquiere un préstamo mínimo por expresión; APIs que retienen vistas usan token explícito. Todo acceso revalida arena, epoch, slot, generation, tipo, rango, permiso y borrow state. Error deja digest lógico idéntico.

## Tree y buffer

El prototipo de árbol usa nodos de 120 bytes: `i64`, presencia+padding y dos handles canónicos. Calcula cantidad/profundidad/bytes antes de construir; profundidad superior al perfil produce `DepthLimit` sin asignación parcial. El recorrido valida cada handle.

El buffer growable duplica capacidad con aritmética checked, asigna bloque nuevo, copia y sólo después libera el anterior. OOM conserva handle/len/capacity/datos anteriores. `peak_used_bytes` incluye brevemente bloque viejo+nuevo y constituye el techo medido del algoritmo, no promesa final del allocator.

## Errores estables

Los códigos ABI-1 están en `memory-abi.json`. Toda operación retorna éxito o uno de ellos: no panic, UB, sentinel ambiguo ni errno dependiente de plataforma. Errores nuevos requieren nueva versión o rango de extensión explícito; un backend no mapea dos errores normativos a éxito.

## Límites

El prototipo es secuencial, usa `Vec` Rust y no demuestra layout físico, zero-copy, thread safety, fragmentación, seguridad del allocator, costo de C/native ni aislamiento OS. ESP-006/007 implementan semántica; ESP-008 compara C; ESP-016/017 congelan calling convention/layout nativo. MAT-024 fija presupuestos de producción.

