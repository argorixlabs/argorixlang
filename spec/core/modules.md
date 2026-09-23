# Módulos y compatibilidad Core 0.1

Cada archivo declara exactamente un módulo. `import package.path;` sólo resuelve módulos del paquete/lock autorizado; no ejecuta scripts, busca red ni escapa la raíz canónica. Imports forman grafo determinista; ciclos de valores/inicialización están prohibidos. Ciclos sólo de firmas/tipos mediante handles se permiten después de resolución conjunta.

Símbolos son privados salvo `pub`. Alias no crean otra identidad nominal. Duplicados, import inexistente, símbolo privado, ciclo prohibido y dos rutas físicas para el mismo módulo son errores.

La cabecera `core 0.1;` selecciona explícitamente Core. Archivos Argorix 1.0 de agentes sin cabecera Core conservan su parser/semántica. Un archivo no puede mezclar items Core con declaraciones de agentes en 0.1. Adaptadores posteriores usan formatos versionados, no inclusión textual.

El compilador recibe la raíz y lock desde `compiler-host`; el texto fuente no concede acceso a filesystem. No existe `extern`, `ffi`, `rust`, `cargo`, shell o builtin que implemente lexer/parser por el programa. ESP-005 define ABI estrecha y ESP-006 implementa el resolvedor temporal con los mismos límites.


## Lectura vigente de los imports (ESP-009.F, ESP-012.C)

Stage0 (`core_link.rs`) y el enlazador Argorix (`compiler/link.argx`) aplican la misma lectura, y `crates/argorixc/tests/link_differential.rs` comprueba que coinciden:

- `import a.b;` enlaza el nombre `b`; `import a.b as x;` enlaza `x`. El enlace no puede coincidir con un item propio ni con otro enlace.
- `b.f(..)` llama a la función pública `f` de `a.b` y `b.C` lee su constante pública `C`, salvo que `b` sea un local, que la oculta.
- Un struct o enum público de un módulo importado directamente se nombra por su nombre sin prefijo, porque un tipo de Core 0.1 es un solo identificador. Ese nombre no puede coincidir con un item propio ni con otro tipo importado.
- Sólo los imports directos son visibles; nada se reexporta.
- Todo ciclo de imports se rechaza. Los ciclos de firmas mediante handles que este documento permite tras resolución conjunta siguen sin implementarse.
