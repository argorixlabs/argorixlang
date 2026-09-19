# Módulos y compatibilidad Core 0.1

Cada archivo declara exactamente un módulo. `import package.path;` sólo resuelve módulos del paquete/lock autorizado; no ejecuta scripts, busca red ni escapa la raíz canónica. Imports forman grafo determinista; ciclos de valores/inicialización están prohibidos. Ciclos sólo de firmas/tipos mediante handles se permiten después de resolución conjunta.

Símbolos son privados salvo `pub`. Alias no crean otra identidad nominal. Duplicados, import inexistente, símbolo privado, ciclo prohibido y dos rutas físicas para el mismo módulo son errores.

La cabecera `core 0.1;` selecciona explícitamente Core. Archivos Argorix 1.0 de agentes sin cabecera Core conservan su parser/semántica. Un archivo no puede mezclar items Core con declaraciones de agentes en 0.1. Adaptadores posteriores usan formatos versionados, no inclusión textual.

El compilador recibe la raíz y lock desde `compiler-host`; el texto fuente no concede acceso a filesystem. No existe `extern`, `ffi`, `rust`, `cargo`, shell o builtin que implemente lexer/parser por el programa. ESP-005 define ABI estrecha y ESP-006 implementa el resolvedor temporal con los mismos límites.

