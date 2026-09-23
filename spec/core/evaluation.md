# Evaluación y control Core 0.1

## Funciones, scopes y llamadas

Funciones tienen firma completa, retorno único y parámetros por valor. La recursión directa/mutua es válida si las firmas se resuelven. Cada bloque crea scope; shadowing local es válido, duplicar nombre en el mismo scope no. Variables deben inicializarse antes de uso y todo camino de una función no-`unit` retorna valor.

Llamada, argumentos, operandos, índices, campos y elementos se evalúan estrictamente de izquierda a derecha, una vez. `&&` y `||` cortocircuitan. No hay inicializadores globales ejecutables.

## Control

`if` usado como expresión exige ramas del mismo tipo y `else`. `while` y `loop` son statements salvo que todos los `break value` de un `loop` concuerden. `break`/`continue` sólo alcanzan el loop léxicamente interior. `return` valida el tipo declarado.

`match` evalúa el scrutinee una vez, prueba brazos en orden y debe ser exhaustivo para bool/enum; guards no cuentan para exhaustividad. Brazos inalcanzables son error. `_` es catch-all explícito.

## Aritmética y traps

Aritmética normal usa el ancho exacto y atrapa en overflow/underflow. División por cero y `MIN / -1` atrapan. Shifts requieren cantidad menor que el ancho. `wrapping_add/sub/mul/shl/shr` son operaciones intrínsecas explícitas y modulares; no se aplican implícitamente.

Acceso fuera de límites, handle inválido, profundidad/steps/memoria excedidos y OOM son traps tipados del runtime; nunca UB. ESP-005 fija recursos y ABI de traps. Una función que espera recuperar un fallo lo representa mediante enum propio (`Ok`/`Err`) y `match`; Core no posee exceptions.

## Determinismo

Sin efectos host, mismo programa/input/perfil produce mismo resultado o trap. Orden de mapas no forma parte de Core 0.1; una tabla de símbolos del corpus usa `Buffer<Entry>` y búsqueda explícita. Reloj, entropía, red y procesos no existen. El filesystem sólo existe como las dos capacidades tipadas del perfil compiler-host (`PackageRead`, `BuildWrite`; ver `stdlib.md`), que el driver presta a `argorix_main`; su resultado depende también del contenido de las raíces prestadas, y un archivo que cambia entre `status` y `read` es el trap `HOST_UNAVAILABLE`, nunca un resultado parcial.

