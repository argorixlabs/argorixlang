# Argorix Core 0.1

Estado: especificación normativa de bootstrap ESP-004, implementada para análisis en el frontend Rust stage0 por ESP-006. `argorixc core-check` reconoce esta sintaxis y la valida; ESP-007 añadió lowering y verificación del IR estructurado, y ESP-008 añadió ejecución mediante el backend C transitorio y el runtime mínimo C1, con lo que un programa Core se compila y se ejecuta hoy (ver [c-backend.md](c-backend.md) y `conformance/core_c/`). ESP-009 está en curso sobre la biblioteca estándar mínima. ESP-010–013 reemplazarán el frontend y el lowering transitorios por fuentes Argorix.

Core 0.1 es un subconjunto de sistemas dentro de ArgorixLang, no un lenguaje separado. Todo archivo empieza con `core 0.1;`; sin esa cabecera, el parser histórico conserva la gramática de agentes 1.0. Una versión Core desconocida se rechaza y nunca se interpreta como otra versión.

Documentos normativos:

- [grammar.ebnf](grammar.ebnf): léxico y gramática concreta;
- [types.md](types.md): tipos, valores, memoria abstracta e igualdad;
- [evaluation.md](evaluation.md): scopes, evaluación, control, errores y overflow;
- [modules.md](modules.md): módulos, imports y relación con Argorix 1.0;
- [core-spec.json](core-spec.json): inventario legible por máquina.
- [ir.md](ir.md): contrato del IR Core verificado y frontera del backend;
- [core-ir.schema.json](core-ir.schema.json): envoltura y discriminadores legibles por máquina.

ESP-005 añade [memoria ejecutable M1/ABI-1](memory.md), el [contrato ABI JSON](memory-abi.json) y la [frontera host H1](../host-abi.md). Son oráculos para stage0/backend, no capacidades ya disponibles en programas Core.

El corpus en `tests/selfhost/spec` fija ejemplos positivos y negativos para cada construcción. ESP-006 ejecuta los 4 casos positivos y 12 negativos con categoría estable. ESP-007 baja y verifica los cuatro positivos y conserva su huella semántica después de serializarlos. Los ejemplos `lexer.argx`, `parser.argx` y `symbols.argx` demuestran expresabilidad; aún no forman el compilador self-hosted ni pueden ejecutarse.

## Superficie mínima

Core define funciones con parámetros/retorno, llamadas y recursión; `let`/`let mut`; bloques léxicos; `if`, `while`, `loop`, `break`, `continue`, `return`; enteros fijos, booleanos, bytes, texto UTF-8, arrays, slices, buffers y arenas; structs y enums; `match` exhaustivo; handles tipados para recursión; módulos/imports; y errores recuperables como enums.

No incluye conversión implícita numérica, null, exceptions, reflection, dynamic dispatch, async, macros, clases, herencia, genéricos de usuario, GC especificado, punteros crudos, acceso al host ni FFI. Las operaciones host se añadirán como capacidades tipadas en ESP-005.

## Determinismo y fallos

Evaluación, argumentos, campos y elementos ocurren de izquierda a derecha. Overflow, división por cero, índice fuera de rango, UTF-8 inválido, handle inválido, OOM y límite excedido atrapan con diagnóstico estable; no son comportamiento indefinido. `wrapping_*` hace overflow modular explícito.
