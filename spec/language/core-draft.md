# Argorix Core — propuesta histórica para autocompilación

Estado: **SUSTITUIDA POR ESP-004 / NO IMPLEMENTADA**. La sintaxis normativa está en [Argorix Core 0.1](../core/README.md), su [EBNF](../core/grammar.ebnf) y corpus `tests/selfhost/spec`. Este borrador se conserva como historial de MAT-003; donde difiera, prevalece Core 0.1. En particular, la versión final usa cabecera `core 0.1;`, `let mut` en lugar de `var`, anchos enteros explícitos, enums concretos para errores recuperables y sólo contenedores intrínsecos parametrizados. El parser stage0 todavía no acepta estos programas; ESP-006 debe implementarlos. La frontera con agentes está en [current-v1.md](current-v1.md).

## Objetivo de expresividad

Core debe poder expresar lexer UTF-8, parser recursivo, AST, tablas de símbolos, resolución de módulos, IR, emisor, VM/verificador, conformidad, firma y gestor local de paquetes sin invocar rutinas Rust ocultas. La biblioteca estándar y el host pueden aportar primitivas explícitas de bytes, archivos, procesos, relojes y criptografía según capacidad; no la lógica del compilador. Un transpiler C transitorio no satisface el backend nativo final.

## C-01 — Programa y nombres

- Mantener `module` e `import` existentes y sus nombres ASCII segmentados. Introducir `fn`, `let`, `var`, `return`, `if`, `else`, `while`, `match` sólo detrás de una versión de fuente/Core explícita, nunca reinterpretando un `.argx` 1.0.
- Espacios léxicos separados: tipos, funciones, valores, módulos y declaraciones de agente. Una referencia ambigua o repetida en el mismo espacio se rechaza; no hay fallback a símbolo de host. Imports se resuelven localmente por manifiesto/lockfile con identidad del paquete; nada se descarga por aparecer un nombre.
- Binding léxico: un nombre interno sombrea un nombre externo sólo donde se permite expresamente. Uso antes de inicialización, rama sin retorno obligatorio y ciclo de import inválido son errores estáticos.

## C-02 — Tipos y valores

- Mínimo propuesto: `bool`, `u8`, `i64`, `bytes`, `string`, `Array<T>`, `Slice<T>`, registros con campos y variantes etiquetadas con payload, `Option<T>` y `Result<T,E>`. `string` contiene UTF-8 válido e inmutable. `bytes` puede contener cualquier octeto. Conversiones bytes/string son explícitas y fallan ante UTF-8 inválido; no hay reemplazo silencioso.
- `i64` usa rango `[-2^63, 2^63-1]`. Literales fuera de rango se rechazan; `u8` fuera de `0..255` se rechaza. Los `int`/`float` declarativos de mensajes 1.0 conservan su significado de contrato heredado y no se convierten automáticamente a `i64`/operaciones Core.
- Igualdad de `bool`, números, bytes y string es por valor (string por bytes UTF-8, sin normalización Unicode implícita); arrays/records/variantes se comparan estructuralmente cuando todos sus componentes permiten igualdad. Orden en colecciones es significativo. No existe igualdad general de handles, capacidades, funciones ni efectos.
- Los tipos recursivos deben pasar por un handle/índice explícito. MAT-004 define validez, ownership, préstamos y lifetime en el modelo M1; el layout físico queda para ESP-005. Antes de implementar/verificar ese contrato, un compilador NO DEBE aceptar programas recursivos de datos como seguros.

## C-03 — Expresiones, evaluación y errores

- Orden de evaluación propuesto: izquierda a derecha para operandos y argumentos; `&&` y `||` cortocircuitan. Una expresión `if` evalúa sólo su rama elegida. `match` requiere cobertura exhaustiva de variantes; brazo inalcanzable es diagnóstico. Las funciones preservan el orden visible de efectos autorizados.
- `+`, `-`, `*` para `i64` y `u8` son **checked**: overflow/underflow retorna error de ejecución tipado o se propaga como `Result`, nunca wrap implícito. División por cero y `i64::MIN / -1` fallan explícitamente. División entera trunca hacia cero; el resto conserva el signo del dividendo. No se permite comportamiento indefinido ni dependencia del overflow del host. Operadores de bit deben especificarse aparte antes de incorporarse.
- Un fallo recuperable es un valor `Result`; `return` entrega valor o error conforme al tipo de función. Trampa irrecuperable aborta la tarea actual y deja evidencia, sin convertirla en éxito. Recursión, bucles, longitud de array/string y stack se limitan por presupuesto declarado, cuyo valor exacto fija MAT-024 por perfil.
- Un literal/cadena se procesa como secuencia UTF-8; `len_bytes` devuelve número de octetos, no grafemas. Indexar string directamente por entero queda prohibido hasta definir una operación segura por bytes o escalares. No se promete normalización NFC/NFD automática.

## C-04 — Control de flujo, memoria y host

- `let` inicializa una vez; `var` permite asignación local tipada. Lectura de variable no inicializada, tipo incompatible o retorno faltante son errores estáticos. `while` y recursión no tienen límite implícito; el runtime aplica cuota de pasos/tiempo y un fallo de cuota explícito. Los efectos del host nunca se ejecutan para compensar un chequeo de tipos fallido.
- Funciones puras no acceden a red/FS/proceso/reloj/aleatoriedad. Una función que invoca host requiere parámetro de capacidad tipada y efecto visible en su firma. No se deriva capacidad de un string, nombre de módulo, DID o metadata de proveedor. El host verifica alcance, caducidad y presupuesto antes de cada operación real (MAT-006/011/013).
- M1 decide arenas con owner, handles generacionales, slices acotados y préstamos read/write dinámicos conservadores. Liberar o cerrar incrementa generación/epoch; no se permite acceso crudo ni liberación con préstamos. Esta decisión está `PROVISIONALLY_ACCEPTED_FOR_PROTOTYPE`, no implementada: Core sigue `DRAFT` y todo fixture de esta sección queda `PLANNED` hasta ESP-004/005/006.

## C-05 — Módulos, artefactos y compatibilidad

- Módulos Core y paquetes usan rutas dentro de raíz autorizada, sin salida por symlink/junction. El lockfile une nombre, versión, origen y digest de cada dependencia. Una compilación reproducible toma mismos bytes/fuentes/flags y produce bytecode semánticamente equivalente; comparación byte a byte sólo si se define serializador canónico y toolchain congelado.
- Source → AST → IR → bytecode → objeto nativo conserva tipo y efecto; cada lowering tiene oráculo de round-trip/semántica. `bytecode_version` y ABI cambian explícitamente si se altera representación. Bytecode viejo no se interpreta como nuevo mediante defaults que agreguen autoridad o efectos.
- La gramática Core final, manejo de errores, ownership y ABI deben publicarse juntos antes de ESP-006. [compatibility.md](../compatibility.md) exige migración documentada para cambios incompatibles. Ningún ejemplo de este archivo cuenta como test aprobado mientras stage0 no soporte la construcción.

## Ejemplos de comportamiento esperado, no ejecutables aún

```argorix
// CORE-P01: resultado 7; orden de llamadas izquierda a derecha.
module core.arithmetic
fn add(a: i64, b: i64) -> Result<i64, Overflow> { return checked_add(a, b) }
fn main() -> Result<i64, Overflow> { return add(3, 4) }
```

```argorix
// CORE-N01: rechazo o error Overflow; nunca -9223372036854775808 por wrap.
module core.overflow
fn main() -> Result<i64, Overflow> { return checked_add(9223372036854775807, 1) }
```

Estos ejemplos ilustran semántica, no sintaxis final: `Result`, `Overflow` y `checked_add` aún no están declarados en una biblioteca estándar Argorix. Su desambiguación es requisito de ESP-004/009, no permiso para introducir un intrinsic Rust opaco.
