# Modelo de memoria Argorix Core M1

Estado: contrato de diseño MAT-004 con [prototipo secuencial seguro ESP-005](core/memory.md); **no implementado aún en stage0 ni demostrado para backend C/nativo**. ESP-006/007 deben preservar sus errores al implementarlo.

## M-01 — Estado y representación abstracta

Una arena tiene `arena_id`, `epoch`, propietario, presupuesto de bytes y estado `active|released`. Una asignación tiene `slot`, `generation`, tipo de elemento, longitud, mutabilidad y estado `live|freed`. Un handle contiene:

```text
Handle<T, P> = (arena_id, arena_epoch, slot, allocation_generation,
                offset, length, permission P)
P = read | read_write
```

La representación física y ancho final pertenecen a ESP-005; la comparación lógica de todos esos campos es obligatoria. Ningún programa válido observa dirección cruda. Copiar un handle copia autoridad acotada, no ownership del arena. Los tipos recursivos usan handles; no layouts infinitos embebidos.

El propietario del arena es una tarea/runtime scope autenticado, no un string controlado por el programa. Sólo el propietario puede asignar, liberar, transferir o cerrar. Transferir ownership es atómico: después del éxito el propietario anterior ya no puede mutar el arena.

## M-02 — Operaciones totales

Todas las operaciones devuelven `Result<Ok, MemoryError>` y carecen de comportamiento indefinido observable.

| Operación | Precondición | Efecto en éxito | Error y atomicidad |
| --- | --- | --- | --- |
| `arena_create(owner, limit)` | owner válido; `0 <= limit <= host_limit` | arena activa, uso 0, epoch nuevo | `InvalidLimit|ResourceLimit`; sin arena parcial |
| `alloc(owner, arena, T, len, mutable)` | owner actual, arena activa; tamaño representable; cuota suficiente | slot live, generación nueva, bytes inicializados a valor cero de T | `NotOwner|ArenaReleased|Overflow|OutOfMemory`; uso/slots no cambian |
| `slice(handle, start, len, P)` | handle válido; rango dentro de vista; P no amplía permiso | nueva vista al mismo slot | `InvalidHandle|OutOfBounds|PermissionDenied`; sin estado nuevo |
| `borrow_read(handle, borrower)` | handle válido | token read; coexistencia con reads | `InvalidHandle|BorrowConflict`; no token parcial |
| `borrow_write(handle, borrower)` | handle `read_write` válido; cero préstamos | token exclusivo write | `InvalidHandle|PermissionDenied|BorrowConflict` |
| `read(handle_or_read_token, index)` | vista válida; índice dentro; no write ajeno | devuelve valor de tipo T | `InvalidHandle|UseAfterFree|ArenaReleased|OutOfBounds|BorrowConflict` |
| `write(handle_or_write_token, index, T)` | permiso write/token write; índice dentro | reemplaza exactamente una celda; tipo se conserva | errores de read + `PermissionDenied|TypeMismatch`; memoria no cambia |
| `end_borrow(token)` | token live | elimina token una vez | `InvalidBorrow`; préstamos no cambian |
| `free(owner, root_handle)` | owner; raíz cubre asignación; live; cero préstamos | estado freed, generación incrementada, cuota recuperada | `NotOwner|InvalidHandle|NotRoot|BorrowConflict|DoubleFree`; atómico |
| `transfer(old, new, arena)` | old es owner; arena activa; cero préstamos; new válido | cambia owner | `NotOwner|BorrowConflict|InvalidOwner`; atómico |
| `arena_release(owner, arena)` | owner; activa; cero préstamos | released, epoch incrementa, todas las asignaciones inválidas, cuota 0 | `NotOwner|BorrowConflict|ArenaReleased`; atómico |

`len=0` es válido y produce una vista sin índices válidos. `offset+length`, `len*size_of(T)` y contadores usan aritmética checked. El runtime no corrige rangos ni convierte OOM en array vacío.

## M-03 — Validez y aliasing

Un handle es válido sólo si coinciden arena/epoch, slot/generation, la asignación está live, su rango está contenido y su permiso no excede al original. Todo acceso repite la validación. Un handle previo a `free` o `arena_release` jamás revive aunque el ID/slot se reutilice.

Regla de préstamos por asignación (conservadora para M1): `N` préstamos read **o** un préstamo write, nunca ambos. Un handle desnudo puede leerse sólo si no existe write token; puede escribirse sólo si no hay préstamo. Slices solapados heredan el mismo estado de préstamo de la asignación. M1 puede rechazar dos writes demostrablemente disjuntos; aceptar ese caso es optimización futura, no requisito.

No se libera ni transfiere con tokens activos. Los tokens no se serializan, no cruzan procesos y se invalidan junto al arena. Esto evita use-after-free y data races en el modelo secuencial. MAT-005 fija happens-before lógico, orden de mensajes y quanta por actor en C1; la sincronización concreta de threads/async permanece para MAT-009 y debe refinar, no contradecir, M1/C1.

## M-04 — Límites, fallos y recursos

Cada arena fija `byte_limit`, cada tarea fija máximos de arenas, handles/tokens, profundidad y pasos. Los números finales son por perfil en MAT-024. `OutOfMemory`, cuota, overflow, stack y timeout son resultados distintos. Un error no autoriza efectos, no consume cuota salvo telemetría acotada y no deja mutación parcial.

Liberar recupera la cuota lógica de la asignación; no promete que el allocator host devuelva páginas al SO. `arena_release` es idempotente sólo mediante manejo explícito del segundo error `ArenaReleased`, no como éxito silencioso. Panic/abort del host nunca se expone como resultado válido Core.

## M-05 — Tipos y preservación

Tipos mínimos: `bool`, `u8`, `i64`, `bytes`, `string`, `Array<T>`, `Slice<T,P>`, records, variants, `Option<T>`, `Result<T,E>`, `Handle<T,P>` y `Capability<K,S>`. `string` siempre contiene UTF-8 válido; conversión desde bytes valida. Valores no inicializados no existen tras `alloc`: se usa inicialización total o constructor que falla antes de publicar el handle.

Invariantes del modelo reducido:

1. **Preservación:** si estado y expresión están bien tipados, un paso produce valor del tipo declarado o error tipado y mantiene estado bien formado.
2. **Progreso acotado:** una operación bien formada produce transición o error enumerado; no queda atascada.
3. **No revival:** epochs/generations crecen y un handle inválido no recupera validez.
4. **Bounds:** toda vista está dentro de su asignación y todo acceso dentro de la vista.
5. **Exclusión:** no coexisten write borrow y otro borrow.
6. **Contabilidad:** `used_bytes` es suma checked de asignaciones live y nunca excede límite.
7. **Atomicidad de fallo:** un error deja digest del estado igual.

La suite MAT-004 comprueba estos invariantes mediante exploración acotada y secuencias generadas. No constituye una prueba matemática general ni evidencia del compilador/runtime.

## M-06 — ABI y layouts

La ABI pública no expone structs internos del backend. Cruces host usan descriptors versionados con enteros de ancho fijo, buffers `(ptr_host, byte_len)` creados por shim, códigos de error y ownership explícito. El programa nunca fabrica `ptr_host`; el shim copia o valida antes de publicar un handle Core.

Los layouts de records/variants, alignment, endianness, calling convention y tamaño concreto de `Handle` se congelan en ESP-005/016/017 por plataforma. Serialización estable es una operación explícita, no `memcpy` del layout. Una incompatibilidad cambia versión y guía de migración.

## M-07 — Decisión de arena

Estado: `PROTOTYPE_ACCEPTED_FOR_TRANSITION`. ESP-005 ejecutó handles/arenas, árbol, buffer, límites, préstamos y ABI canónica sin `unsafe`; no acredita stage0, C, nativo ni producción. El diseño se rechaza si una implementación posterior cambia errores/invariantes o no reproduce los vectores. Rendimiento release, fragmentación y concurrencia siguen abiertos en ESP-008/016/017 y MAT-024. El gate abstracto permanece en `tests/models/memory/model-policy.json`.
