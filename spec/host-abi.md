# ABI host Argorix H1

Estado: contrato ESP-005; no es todavía una implementación de sandbox o adapter real.

## Regla de frontera

Core nunca recibe o fabrica punteros host, file descriptors, sockets, process handles, environment blocks ni function pointers. Cruces usan descriptors ABI-1 serializados, handles M1 validados y capability A1. El shim copia/valida antes de publicar memoria Core y vuelve a comprobar autoridad inmediatamente antes del dispatch.

`CoreResultV1` tiene 56 bytes: `abi_version:u16`, `status:u16` (`0=ok,1=error,2=uncertain`), `error_code:u32` y `payload:CoreHandleV1`. Resultado sin payload usa handle cero reservado que el programa no puede dereferenciar. Unknown version/status/error falla cerrado.

`HostCallV1` tiene 120 bytes: version/profile/operation (8), request ID (8), capability descriptor handle (48), input handle (48) y deadline monotónico (8). Strings cruzan como bytes UTF-8 validados; bytes arbitrarios se etiquetan separadamente. Outputs viven en arena del caller con cuota previa.

## Perfiles incompatibles

| Perfil | Operaciones iniciales | Prohibido explícitamente |
| --- | --- | --- |
| `compiler-host` | `package.read`, `build.write`, `clock.metadata`, `linker.invoke` | red, secreto, shell, proceso arbitrario, rutas fuera de roots |
| `agent-runtime` | `fs.read`, `fs.write`, `clock.read`, `process.spawn` sólo si cada capability nominada existe | heredar package/build/linker, shell implícito, ambiente completo |
| `verification-host` | `artifact.read`, `anchor.read` | escritura, red, proceso, linker, ejecución del programa |

Capability types incluyen profile, operation, canonical scope, subject, expiry, budget y nonce. No hay conversión entre perfiles. `process.spawn` exige executable exacto/digest, argv estructurado, cwd/root y environment allowlist; jamás shell string. `linker.invoke` sólo acepta linker/toolchain fijados y objetos en build root.

## Orden de validación

1. Decodificar/versionar descriptor sin dereferenciar payload.
2. Validar todos los handles, tipos, tamaños, UTF-8 y límites.
3. Validar perfil, effect declarado y capability A1 fresca.
4. Reservar output/budget.
5. Despachar mediante operación nominada.
6. Registrar estado y copiar resultado acotado; fallo o incertidumbre no se convierten en éxito.

Un error de memoria ocurre antes de eventos `dispatched`. Ausencia de servicio, operación desconocida o capability incorrecta produce código estable y cero dispatch.

## Compatibilidad entre backends

Encoding canónico es little-endian en ambos targets iniciales. Cada backend debe ejecutar los mismos vectores hex, códigos y casos negativos. El layout en registros/stack se mantiene interno; sólo los bytes ABI-1 son contrato entre componentes. Cambiar tamaño, offset o semántica incrementa `abi_version` y requiere migración.

Este contrato no prueba que filesystem/proceso estén aislados: MAT-011/013 y ESP-021 deben aplicar límites OS y sensores externos.

