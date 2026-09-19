# Tipos y valores Core 0.1

## Escalares

`bool`, `unit`, enteros `u8/u16/u32/u64/i8/i16/i32/i64`, `bytes` inmutable y `string` UTF-8 válido. No hay `usize`, tamaño dependiente del host, float, null ni coerciones implícitas. Literales sin suffix se infieren desde el contexto; fuera de rango es error estático.

## Agregados e intrínsecos paramétricos

- `Array<T,N>`: longitud constante y layout por especificar en ESP-005.
- `Slice<T>`: vista inmutable o mutable según el origen; no posee memoria y valida límites.
- `Buffer<T>`: secuencia growable con operaciones fallibles; su ABI/memoria se concreta en ESP-005.
- `Arena<T>`: almacenamiento growable de objetos estables; `alloc` retorna `Handle<T>` y liberar/incrementar generación invalida handles anteriores.
- `Handle<T>`: referencia tipada con identidad y generación. Sólo permite recursión indirecta; no existe aritmética de punteros.
- `struct`: producto nominal con campos en orden declarado.
- `enum`: suma nominal etiquetada; variantes pueden llevar campos.

Sólo estos cinco constructores intrínsecos aceptan argumentos de tipo en 0.1. No existen genéricos definidos por usuario. Un tipo recursivo directo (`struct Node { next: Node }`) es inválido; `Handle<Node>` es válido y el acceso puede fallar con `InvalidHandle`.

## Mutabilidad, movimiento e igualdad

Bindings son inmutables salvo `let mut`. Asignar el binding requiere mutabilidad. Escalares, arrays de valores copiables y handles se copian; buffers/arenas se mueven salvo préstamo temporal. Usar un valor movido es error estático. Acceder por `Handle<T>` valida generación/límites y adquiere el préstamo más corto: lectura/copia obtiene read borrow; asignación o método mutador obtiene write borrow exclusivo y falla antes de mutar si existe conflicto. La mutabilidad del binding-handle sólo permite cambiar qué identidad guarda, no eludir préstamos. ESP-005 debe prototipar y congelar estos lifetimes antes de implementar stage0.

Igualdad existe para escalares, bytes/string, arrays y structs/enums cuyos campos sean comparables. Es estructural y determinista. Handles comparan identidad `(arena,generation,index)`, no contenido. Buffers no son comparables por defecto.

## Texto y bytes

`string` siempre es UTF-8 válido y se recorre por iterador de scalar Unicode, no por índice entero directo. `bytes`/`Slice<u8>` se indexan por byte. Decodificar retorna enum `Utf8Result`; jamás reemplaza silenciosamente datos inválidos. Longitudes y offsets públicos usan `u64`; una plataforma que no pueda representarlos rechaza la asignación.
