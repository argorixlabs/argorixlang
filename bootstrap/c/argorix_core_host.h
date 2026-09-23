/* Compiler-host shim for Argorix Core C1 (ESP-009, `stdlib.compiler_host`).
 *
 * Included only by a program whose functions can hold a `PackageRead` or
 * `BuildWrite` capability; every other binary is built without it and has no
 * file-system code. The generated C defines _XOPEN_SOURCE 700 before the
 * first include, for realpath and O_NOFOLLOW.
 *
 * The driver names a package root, a build root and their byte budgets on
 * the command line. Nothing else is read: no environment, no current
 * directory, no process, no network. Each operation re-checks, in C, what
 * `stdlib.path` checked in Argorix: the path is valid UTF-8, relative, in
 * normal form and free of NUL, `\` and `:`. It then resolves the path with
 * realpath and refuses anything that lands outside its root, so a symlink
 * cannot lead out of it either.
 *
 * Status codes are stable and are the contract with `stdlib.compiler_host`:
 * 0 ok, 1 invalid path, 2 not found, 3 outside the root, 4 not a regular
 * file, 5 over budget, 6 I/O error, 7 unsupported host. Every function is
 * static inline so that one a program never calls is not an error under
 * -Werror.
 */
#ifndef ARGORIX_CORE_HOST_H
#define ARGORIX_CORE_HOST_H

#include "argorix_core_runtime.h"

#include <stdbool.h>
#include <stdint.h>
#include <string.h>

typedef struct argorix_capability {
    uint32_t kind; /* 1 package read, 2 build write */
} argorix_capability;

enum {
    ARGORIX_HOST_OK = 0,
    ARGORIX_HOST_INVALID_PATH = 1,
    ARGORIX_HOST_NOT_FOUND = 2,
    ARGORIX_HOST_OUTSIDE_ROOT = 3,
    ARGORIX_HOST_NOT_A_FILE = 4,
    ARGORIX_HOST_OVER_BUDGET = 5,
    ARGORIX_HOST_IO_ERROR = 6,
    ARGORIX_HOST_UNSUPPORTED = 7
};

/* The path rules of stdlib.path, again on the host side. */
static inline bool argorix_host_path_valid(const uint8_t *path, uint64_t length) {
    if (length == 0U || path[0] == '/') {
        return false;
    }
    uint64_t index = 0U;
    uint64_t segment = 0U;
    while (index <= length) {
        if (index == length || path[index] == '/') {
            uint64_t size = index - segment;
            if (size == 0U) {
                return false;
            }
            if (size == 1U && path[segment] == '.') {
                return false;
            }
            if (size == 2U && path[segment] == '.' && path[segment + 1U] == '.') {
                return false;
            }
            segment = index + 1U;
            index += 1U;
            continue;
        }
        uint8_t byte = path[index];
        if (byte == 0U || byte == '\\' || byte == ':') {
            return false;
        }
        /* UTF-8 by Table 3-7 of the Unicode standard. */
        uint64_t extra = 0U;
        uint8_t low = 0x80U;
        uint8_t high = 0xBFU;
        if (byte < 0x80U) {
            extra = 0U;
        } else if (byte >= 0xC2U && byte <= 0xDFU) {
            extra = 1U;
        } else if (byte == 0xE0U) {
            extra = 2U;
            low = 0xA0U;
        } else if ((byte >= 0xE1U && byte <= 0xECU) || byte == 0xEEU || byte == 0xEFU) {
            extra = 2U;
        } else if (byte == 0xEDU) {
            extra = 2U;
            high = 0x9FU;
        } else if (byte == 0xF0U) {
            extra = 3U;
            low = 0x90U;
        } else if (byte >= 0xF1U && byte <= 0xF3U) {
            extra = 3U;
        } else if (byte == 0xF4U) {
            extra = 3U;
            high = 0x8FU;
        } else {
            return false;
        }
        if (length - index - 1U < extra) {
            return false;
        }
        for (uint64_t next = 1U; next <= extra; next++) {
            uint8_t continuation = path[index + next];
            uint8_t min = next == 1U ? low : 0x80U;
            uint8_t max = next == 1U ? high : 0xBFU;
            if (continuation < min || continuation > max) {
                return false;
            }
        }
        index += extra + 1U;
    }
    return true;
}

#ifndef _WIN32

#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdlib.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

/* Index 1 is the package root, index 2 the build root. */
static char argorix_host_root[3][PATH_MAX];
static bool argorix_host_granted[3];
static uint64_t argorix_host_budget[3];

static inline bool argorix_host_parse_budget(const char *text, uint64_t *out) {
    uint64_t value = 0U;
    if (text[0] == '\0') {
        return false;
    }
    for (const char *cursor = text; *cursor != '\0'; cursor++) {
        if (*cursor < '0' || *cursor > '9') {
            return false;
        }
        uint64_t digit = (uint64_t)(*cursor - '0');
        if (value > (UINT64_MAX - digit) / 10U) {
            return false;
        }
        value = value * 10U + digit;
    }
    *out = value;
    return true;
}

/* Grants exactly the capabilities the entry takes. An unknown flag, a flag
   for a capability the entry does not take, a missing root or budget, or a
   root that is not a directory is PERMISSION_DENIED before any Core code
   runs. */
static inline void argorix_host_start(int argc, char **argv, bool package, bool build) {
    bool seen_root[3] = {false, false, false};
    bool seen_budget[3] = {false, false, false};
    for (int index = 1; index < argc; index += 2) {
        if (index + 1 >= argc) {
            argorix_trap("PERMISSION_DENIED");
        }
        const char *flag = argv[index];
        const char *value = argv[index + 1];
        int kind = 0;
        bool is_root = false;
        if (strcmp(flag, "--package-root") == 0) {
            kind = 1;
            is_root = true;
        } else if (strcmp(flag, "--read-budget") == 0) {
            kind = 1;
        } else if (strcmp(flag, "--build-root") == 0) {
            kind = 2;
            is_root = true;
        } else if (strcmp(flag, "--write-budget") == 0) {
            kind = 2;
        } else {
            argorix_trap("PERMISSION_DENIED");
        }
        if ((kind == 1 && !package) || (kind == 2 && !build)) {
            argorix_trap("PERMISSION_DENIED");
        }
        if (is_root) {
            struct stat info;
            if (seen_root[kind] || realpath(value, argorix_host_root[kind]) == NULL
                || stat(argorix_host_root[kind], &info) != 0 || !S_ISDIR(info.st_mode)) {
                argorix_trap("PERMISSION_DENIED");
            }
            seen_root[kind] = true;
        } else {
            if (seen_budget[kind] || !argorix_host_parse_budget(value, &argorix_host_budget[kind])) {
                argorix_trap("PERMISSION_DENIED");
            }
            seen_budget[kind] = true;
        }
    }
    if ((package && (!seen_root[1] || !seen_budget[1]))
        || (build && (!seen_root[2] || !seen_budget[2]))) {
        argorix_trap("PERMISSION_DENIED");
    }
    argorix_host_granted[1] = package;
    argorix_host_granted[2] = build;
}

static inline argorix_capability argorix_host_capability(uint32_t kind) {
    if (kind < 1U || kind > 2U || !argorix_host_granted[kind]) {
        argorix_trap("PERMISSION_DENIED");
    }
    argorix_capability capability = {kind};
    return capability;
}

static inline void argorix_host_require(argorix_capability capability, uint32_t kind) {
    if (capability.kind != kind || !argorix_host_granted[kind]) {
        argorix_trap("PERMISSION_DENIED");
    }
}

/* root + "/" + path, or false if it does not fit. */
static inline bool argorix_host_join(
    uint32_t kind, const uint8_t *path, uint64_t length, char *out
) {
    size_t root = strlen(argorix_host_root[kind]);
    if (length >= (uint64_t)PATH_MAX || root + 1U + (size_t)length + 1U > PATH_MAX) {
        return false;
    }
    memcpy(out, argorix_host_root[kind], root);
    out[root] = '/';
    memcpy(out + root + 1U, path, (size_t)length);
    out[root + 1U + (size_t)length] = '\0';
    return true;
}

/* Whether a resolved path is the root or lies under it. */
static inline bool argorix_host_inside(uint32_t kind, const char *resolved, bool allow_root) {
    size_t root = strlen(argorix_host_root[kind]);
    if (strncmp(resolved, argorix_host_root[kind], root) != 0) {
        return false;
    }
    if (resolved[root] == '\0') {
        return allow_root;
    }
    /* A root of "/" already ends in the separator. */
    return resolved[root] == '/' || (root == 1U && argorix_host_root[kind][0] == '/');
}

/* Resolves a package path to a regular file inside the package root. */
static inline uint32_t argorix_host_locate(
    const uint8_t *path, uint64_t length, char *resolved, uint64_t *size
) {
    if (!argorix_host_path_valid(path, length)) {
        return ARGORIX_HOST_INVALID_PATH;
    }
    char joined[PATH_MAX];
    if (!argorix_host_join(1U, path, length, joined)) {
        return ARGORIX_HOST_INVALID_PATH;
    }
    if (realpath(joined, resolved) == NULL) {
        return (errno == ENOENT || errno == ENOTDIR) ? ARGORIX_HOST_NOT_FOUND
                                                     : ARGORIX_HOST_IO_ERROR;
    }
    if (!argorix_host_inside(1U, resolved, false)) {
        return ARGORIX_HOST_OUTSIDE_ROOT;
    }
    struct stat info;
    if (stat(resolved, &info) != 0) {
        return ARGORIX_HOST_IO_ERROR;
    }
    if (!S_ISREG(info.st_mode)) {
        return ARGORIX_HOST_NOT_A_FILE;
    }
    *size = (uint64_t)info.st_size;
    if (*size > argorix_host_budget[1]) {
        return ARGORIX_HOST_OVER_BUDGET;
    }
    return ARGORIX_HOST_OK;
}

static inline uint32_t argorix_host_package_status(
    argorix_capability capability, const uint8_t *path, uint64_t length
) {
    argorix_host_require(capability, 1U);
    char resolved[PATH_MAX];
    uint64_t size = 0U;
    return argorix_host_locate(path, length, resolved, &size);
}

/* The file's bytes. Reading is only meant after `status` returned 0: a file
   that changed in between (gone, grown past the budget, unreadable) is the
   trap HOST_UNAVAILABLE, never a partial result. */
static inline argorix_buffer argorix_host_package_read(
    argorix_capability capability, const uint8_t *path, uint64_t length, uint64_t byte_limit
) {
    argorix_host_require(capability, 1U);
    char resolved[PATH_MAX];
    uint64_t size = 0U;
    if (argorix_host_locate(path, length, resolved, &size) != ARGORIX_HOST_OK) {
        argorix_trap("HOST_UNAVAILABLE");
    }
    int descriptor = open(resolved, O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
    if (descriptor < 0) {
        argorix_trap("HOST_UNAVAILABLE");
    }
    struct stat info;
    if (fstat(descriptor, &info) != 0 || !S_ISREG(info.st_mode) || (uint64_t)info.st_size != size) {
        (void)close(descriptor);
        argorix_trap("HOST_UNAVAILABLE");
    }
    argorix_buffer buffer = argorix_buffer_new(1U, byte_limit);
    uint8_t chunk[4096];
    uint64_t total = 0U;
    while (total < size) {
        ssize_t count = read(descriptor, chunk, sizeof chunk);
        if (count < 0 && errno == EINTR) {
            continue;
        }
        if (count <= 0 || total + (uint64_t)count > size) {
            (void)close(descriptor);
            argorix_buffer_drop(&buffer);
            argorix_trap("HOST_UNAVAILABLE");
        }
        for (ssize_t index = 0; index < count; index++) {
            argorix_buffer_push(&buffer, &chunk[index]);
        }
        total += (uint64_t)count;
    }
    (void)close(descriptor);
    argorix_host_budget[1] -= size;
    return buffer;
}

/* Writes a whole file under the build root. The directory must exist inside
   the root; the file itself is created or replaced, never followed if it is
   a symlink. */
static inline uint32_t argorix_host_build_write(
    argorix_capability capability,
    const uint8_t *path,
    uint64_t length,
    const uint8_t *data,
    uint64_t data_length
) {
    argorix_host_require(capability, 2U);
    if (!argorix_host_path_valid(path, length)) {
        return ARGORIX_HOST_INVALID_PATH;
    }
    if (data_length > argorix_host_budget[2]) {
        return ARGORIX_HOST_OVER_BUDGET;
    }
    char joined[PATH_MAX];
    if (!argorix_host_join(2U, path, length, joined)) {
        return ARGORIX_HOST_INVALID_PATH;
    }
    char *slash = strrchr(joined, '/');
    *slash = '\0';
    char parent[PATH_MAX];
    if (realpath(joined, parent) == NULL) {
        return (errno == ENOENT || errno == ENOTDIR) ? ARGORIX_HOST_NOT_FOUND
                                                     : ARGORIX_HOST_IO_ERROR;
    }
    if (!argorix_host_inside(2U, parent, true)) {
        return ARGORIX_HOST_OUTSIDE_ROOT;
    }
    char target[PATH_MAX];
    size_t parent_length = strlen(parent);
    size_t name_length = strlen(slash + 1);
    if (parent_length + 1U + name_length + 1U > PATH_MAX) {
        return ARGORIX_HOST_INVALID_PATH;
    }
    memcpy(target, parent, parent_length);
    target[parent_length] = '/';
    memcpy(target + parent_length + 1U, slash + 1, name_length + 1U);
    int descriptor = open(target, O_WRONLY | O_CREAT | O_TRUNC | O_NOFOLLOW | O_CLOEXEC, 0644);
    if (descriptor < 0) {
        if (errno == ELOOP) {
            return ARGORIX_HOST_OUTSIDE_ROOT;
        }
        return errno == EISDIR ? ARGORIX_HOST_NOT_A_FILE : ARGORIX_HOST_IO_ERROR;
    }
    uint64_t written = 0U;
    while (written < data_length) {
        size_t step = (size_t)(data_length - written);
        ssize_t count = write(descriptor, data + written, step);
        if (count < 0 && errno == EINTR) {
            continue;
        }
        if (count <= 0) {
            (void)close(descriptor);
            return ARGORIX_HOST_IO_ERROR;
        }
        written += (uint64_t)count;
    }
    if (close(descriptor) != 0) {
        return ARGORIX_HOST_IO_ERROR;
    }
    argorix_host_budget[2] -= data_length;
    return ARGORIX_HOST_OK;
}

#else /* _WIN32: no compiler-host boundary yet; every operation says so. */

static bool argorix_host_granted[3];

static inline void argorix_host_start(int argc, char **argv, bool package, bool build) {
    (void)argc;
    (void)argv;
    argorix_host_granted[1] = package;
    argorix_host_granted[2] = build;
}

static inline argorix_capability argorix_host_capability(uint32_t kind) {
    if (kind < 1U || kind > 2U || !argorix_host_granted[kind]) {
        argorix_trap("PERMISSION_DENIED");
    }
    argorix_capability capability = {kind};
    return capability;
}

static inline uint32_t argorix_host_package_status(
    argorix_capability capability, const uint8_t *path, uint64_t length
) {
    (void)capability;
    (void)path;
    (void)length;
    return ARGORIX_HOST_UNSUPPORTED;
}

static inline argorix_buffer argorix_host_package_read(
    argorix_capability capability, const uint8_t *path, uint64_t length, uint64_t byte_limit
) {
    (void)capability;
    (void)path;
    (void)length;
    (void)byte_limit;
    argorix_trap("HOST_UNAVAILABLE");
}

static inline uint32_t argorix_host_build_write(
    argorix_capability capability,
    const uint8_t *path,
    uint64_t length,
    const uint8_t *data,
    uint64_t data_length
) {
    (void)capability;
    (void)path;
    (void)length;
    (void)data;
    (void)data_length;
    return ARGORIX_HOST_UNSUPPORTED;
}

#endif /* _WIN32 */

#endif /* ARGORIX_CORE_HOST_H */
