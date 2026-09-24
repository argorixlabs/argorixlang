/* The compiler-host operations for native code (ESP-016): the C1 host shim
 * (bootstrap/c/argorix_core_host.h) with a capability passed as its kind and
 * a buffer returned through a pointer. Linked only into a program that can
 * hold a capability, as the C backend includes the host shim only then.
 */
#define _XOPEN_SOURCE 700
#ifdef __APPLE__
#define _DARWIN_C_SOURCE 1
#endif
#include "argorix_core_runtime.h"
#include "argorix_core_host.h"

void argorix_rt_host_start(int argc, char **argv, uint32_t package, uint32_t build) {
    argorix_host_start(argc, argv, package != 0U, build != 0U);
}

uint32_t argorix_rt_host_capability(uint32_t kind) {
    return argorix_host_capability(kind).kind;
}

uint64_t argorix_rt_package_status(uint32_t kind, const uint8_t *path, uint64_t length) {
    argorix_capability capability = {kind};
    return argorix_host_package_status(capability, path, length);
}

void argorix_rt_package_read(
    argorix_buffer *out, uint32_t kind, const uint8_t *path, uint64_t length, uint64_t byte_limit
) {
    argorix_capability capability = {kind};
    *out = argorix_host_package_read(capability, path, length, byte_limit);
}

uint64_t argorix_rt_build_write(
    uint32_t kind, const uint8_t *path, uint64_t length, const uint8_t *data, uint64_t data_length
) {
    argorix_capability capability = {kind};
    return argorix_host_build_write(capability, path, length, data, data_length);
}
