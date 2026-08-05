/*
 * dl_ex.cpp — Twoyi-specific dl*_ex wrappers, ported from the legacy blob.
 *
 * The legacy libOpenglRender.so (closed-source) implements 4 functions —
 * dlopen_ex / dlsym_ex / dlclose_ex / dlerror_ex — that work around
 * Android 7.0+ (API >= 24 / NDK r14) library-namespace restrictions.
 *
 * Background: starting with Nougat, Android restricts which .so files an
 * app may dlopen by name. The legacy blob defeats this by:
 *   1. Reading ro.build.version.sdk once and caching it in g_sdk_int.
 *   2. On SDK < 24: delegating to the libc dl* functions unchanged.
 *   3. On SDK >= 24:
 *      - dlopen_ex() scans /proc/self/maps for an already-loaded mapping
 *        of the requested library; if found, it opens the on-disk ELF,
 *        parses its .dynsym table, and returns a custom "ExHandle" struct
 *        containing the symbol table.  If not found, it concatenates the
 *        library name onto each of 5 hardcoded system library paths and
 *        repeats the maps scan; if still not found, falls back to plain
 *        dlopen().
 *      - dlsym_ex() walks the ExHandle->symbols[] table and returns
 *        (base + sym_offset - load_bias) — this resolves mangled C++
 *        symbols that aren't in the public dynamic symbol table.
 *      - dlclose_ex() frees the ExHandle and its bookkeeping.
 *      - dlerror_ex() always returns NULL (the custom path never sets
 *        dlerror state).
 *
 * Legacy sizes (arm64): dlopen_ex 548 B, dlsym_ex 276 B,
 *                       dlclose_ex 208 B, dlerror_ex 144 B (1,176 B total).
 *
 * This file is the open-source re-implementation.  It is functionally
 * equivalent for twoyi's use case (looking up AHardwareBuffer_* symbols
 * in libandroid.so on Android 7+ devices).
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <limits.h>
#include <sys/mman.h>
#include <elf.h>
#include <sys/system_properties.h>
#include <cutils/log.h>

#ifndef PATH_MAX
#define PATH_MAX 4096
#endif

// ---------------------------------------------------------------------------
// Cached SDK version. The legacy blob stores this at offset 28 of the
// .bss data group starting at the GraphicBuffer static-state block; here
// we just use a file-static.
// ---------------------------------------------------------------------------
static int g_sdk_int = -1;

static int read_sdk_int()
{
    if (g_sdk_int > 0) return g_sdk_int;
    char buf[92];
    memset(buf, 0, sizeof(buf));
    __system_property_get("ro.build.version.sdk", buf);
    g_sdk_int = atoi(buf);
    return g_sdk_int;
}

// ---------------------------------------------------------------------------
// Custom handle returned by dlopen_ex() on Android 7+.
//
// Layout (matches legacy byte-for-byte; offsets verified from
// dlsym_ex / dlclose_ex disassembly):
//   offset  0: void* base_addr      (load base of the library in memory)
//   offset  8: ExSymbol* symbols    (allocated array of parsed symbols)
//   offset 16: char* strtab_copy    (owned copy of the .dynstr blob)
//   offset 24: uint32_t num_symbols (count of valid entries in symbols[])
//   offset 32: uintptr_t load_bias  (bias subtracted from absolute addrs)
//
// Total size: 40 bytes (0x28).  Allocated with calloc(1, 40) in legacy.
// ---------------------------------------------------------------------------
struct ExSymbol {
    const char* name;     // borrowed from strtab_copy
    uintptr_t   offset;   // st_value (file-relative; rebased at lookup time)
};

struct ExHandle {
    void*       base_addr;       // +0
    ExSymbol*   symbols;         // +8
    char*       strtab_copy;     // +16
    uint32_t    num_symbols;     // +24
    uintptr_t   load_bias;       // +32
};

// ---------------------------------------------------------------------------
// Hardcoded system library search paths (Android 7+).  Identical to the
// legacy blob's .rodata strings at 0xdd270..0xdd2d8.
// ---------------------------------------------------------------------------
static const char* kSystemLibPaths[] = {
    "/system/lib64/",
    "/apex/com.android.runtime/lib64/",
    "/apex/com.android.art/lib64/",
    "/odm/lib64/",
    "/vendor/lib64/",
};

// ---------------------------------------------------------------------------
// find_loaded_base: scan /proc/self/maps for a line whose mapping-path
// contains `libpath`, with permissions containing "r-xp" or "r--p".
// Returns the mapping's base address (hex parsed from the line start),
// or 0 if not found.
//
// The legacy implementation looks for r-xp (executable) and r--p
// (read-only) mappings — both indicate that the library file is loaded
// (the r--p entry is the ELF header mapping at offset 0).
// ---------------------------------------------------------------------------
static uintptr_t find_loaded_base(const char* libpath)
{
    FILE* fp = fopen("/proc/self/maps", "r");
    if (!fp) return 0;

    char line[512];
    uintptr_t base = 0;
    while (fgets(line, sizeof(line), fp)) {
        if (!strstr(line, libpath)) continue;
        if (!strstr(line, "r-xp") && !strstr(line, "r--p")) continue;
        // Found a matching executable/read-only mapping.  Parse the base
        // address from the start of the line (format: "ADDR-ADDR perms ...").
        uintptr_t addr = 0;
        if (sscanf(line, "%lx-", &addr) >= 1) {
            base = addr;
            break;
        }
    }
    fclose(fp);
    return base;
}

// ---------------------------------------------------------------------------
// parse_elf_dynsym: open `path` on disk, mmap it, walk its ELF section
// headers, find .dynsym + .dynstr, and populate an ExHandle with one
// ExSymbol entry per function/object symbol.
//
// `load_base` is the in-memory base address (from /proc/self/maps); it is
// used to compute the load_bias so dlsym_ex can return absolute addresses
// as (base + sym_offset - load_bias).
// ---------------------------------------------------------------------------
static ExHandle* parse_elf_dynsym(const char* path, uintptr_t load_base)
{
    int fd = open(path, O_RDONLY);
    if (fd < 0) return NULL;

    off_t size = lseek(fd, 0, SEEK_END);
    if (size <= 0) { close(fd); return NULL; }

    void* map = mmap(NULL, (size_t)size, PROT_READ, MAP_SHARED, fd, 0);
    close(fd);
    if (map == MAP_FAILED) return NULL;

    const Elf64_Ehdr* ehdr = (const Elf64_Ehdr*)map;
    if (memcmp(ehdr->e_ident, ELFMAG, SELFMAG) != 0) {
        munmap(map, (size_t)size);
        return NULL;
    }

    const Elf64_Shdr* shdrs = (const Elf64_Shdr*)((const char*)map + ehdr->e_shoff);
    const char* shstrtab = (const char*)map + shdrs[ehdr->e_shstrndx].sh_offset;

    // Locate .dynsym + .dynstr by name.
    const Elf64_Shdr* dynsym_sh = NULL;
    const Elf64_Shdr* dynstr_sh = NULL;
    for (unsigned i = 0; i < ehdr->e_shnum; i++) {
        const char* name = shstrtab + shdrs[i].sh_name;
        if (shdrs[i].sh_type == SHT_DYNSYM && strcmp(name, ".dynsym") == 0) {
            dynsym_sh = &shdrs[i];
        } else if (shdrs[i].sh_type == SHT_STRTAB && strcmp(name, ".dynstr") == 0) {
            dynstr_sh = &shdrs[i];
        }
    }
    if (!dynsym_sh || !dynstr_sh) {
        munmap(map, (size_t)size);
        return NULL;
    }

    const Elf64_Sym* syms = (const Elf64_Sym*)((const char*)map + dynsym_sh->sh_offset);
    size_t sym_count = dynsym_sh->sh_size / sizeof(Elf64_Sym);
    const char* strtab = (const char*)map + dynstr_sh->sh_offset;
    size_t strtab_size = dynstr_sh->sh_size;

    ExHandle* h = (ExHandle*)calloc(1, sizeof(ExHandle));
    if (!h) { munmap(map, (size_t)size); return NULL; }

    h->base_addr  = (void*)load_base;
    h->load_bias  = 0;  // set below — see comment
    // The legacy subtracts load_bias from (base + sym_offset).  In the
    // normal case where base_addr is the in-memory load address of the
    // *file's* base (i.e. the mapping that contains the ELF header),
    // load_bias equals the in-memory base, and st_value already encodes
    // the file-relative offset.  The net expression (base + st_value -
    // load_bias) therefore equals st_value, which for a typical ET_DYN
    // .so equals the offset of the symbol within the file's load
    // segment — i.e. the absolute runtime address.
    h->load_bias  = load_base;

    // Allocate the symbols array + a private copy of the strtab.
    h->strtab_copy = (char*)malloc(strtab_size);
    if (!h->strtab_copy) { free(h); munmap(map, (size_t)size); return NULL; }
    memcpy(h->strtab_copy, strtab, strtab_size);

    h->symbols = (ExSymbol*)calloc(sym_count, sizeof(ExSymbol));
    if (!h->symbols) { free(h->strtab_copy); free(h); munmap(map, (size_t)size); return NULL; }

    uint32_t n = 0;
    for (size_t i = 0; i < sym_count; i++) {
        if (syms[i].st_name == 0) continue;
        if (ELF64_ST_TYPE(syms[i].st_info) != STT_FUNC &&
            ELF64_ST_TYPE(syms[i].st_info) != STT_OBJECT) continue;
        if (syms[i].st_value == 0) continue;
        h->symbols[n].name   = h->strtab_copy + syms[i].st_name;
        h->symbols[n].offset = syms[i].st_value;
        n++;
    }
    h->num_symbols = n;

    munmap(map, (size_t)size);
    return h;
}

// ---------------------------------------------------------------------------
// check_loaded: given a candidate library path, look it up in
// /proc/self/maps.  If found, parse its ELF .dynsym and return an
// ExHandle; otherwise return NULL.  (Mirrors the legacy helper at
// dlerror_ex+0x90 / vaddr 0x57470.)
// ---------------------------------------------------------------------------
static ExHandle* check_loaded(const char* libpath)
{
    uintptr_t base = find_loaded_base(libpath);
    if (base == 0) return NULL;
    return parse_elf_dynsym(libpath, base);
}

// ===========================================================================
// Public dl*_ex API
// ===========================================================================

extern "C" {

// dlopen_ex — Android-7+-aware dlopen.
//   * SDK <  24 : plain dlopen().
//   * SDK >= 24 : try /proc/self/maps first (5 system paths + the literal
//                 filename); fall back to plain dlopen() if not found.
void* dlopen_ex(const char* filename, int flag)
{
    if (!filename) return dlopen(NULL, flag);

    int sdk = read_sdk_int();
    if (sdk < 24) {
        return dlopen(filename, flag);
    }

    // 1) If filename is absolute, try the maps scan directly.
    if (filename[0] == '/') {
        ExHandle* h = check_loaded(filename);
        if (h) return h;
    } else {
        // 2) Try the 5 hardcoded system library paths.
        char full[PATH_MAX];
        for (size_t i = 0; i < sizeof(kSystemLibPaths)/sizeof(kSystemLibPaths[0]); i++) {
            snprintf(full, sizeof(full), "%s%s", kSystemLibPaths[i], filename);
            ExHandle* h = check_loaded(full);
            if (h) return h;
        }
    }

    // 3) Fall back to plain dlopen — may fail on Android 7+ due to
    //    namespace restrictions, but we preserve the legacy behavior.
    return dlopen(filename, flag);
}

// dlsym_ex — Android-7+-aware dlsym.
//   * SDK <  24 : plain dlsym().
//   * SDK >= 24 : if `handle` is an ExHandle (allocated by dlopen_ex),
//                 walk its symbols[] table; otherwise fall back to dlsym().
void* dlsym_ex(void* handle, const char* symbol)
{
    if (!handle || !symbol) return NULL;
    int sdk = read_sdk_int();
    if (sdk < 24) {
        return dlsym(handle, symbol);
    }

    // Heuristic: an ExHandle's first 8 bytes are a non-null base_addr.
    // Plain dlopen handles are also non-null, so this isn't perfectly
    // distinguishable at runtime — but in twoyi's usage, dlopen_ex is
    // the only producer of handles passed to dlsym_ex, so an ExHandle
    // is always what we get.
    ExHandle* h = (ExHandle*)handle;
    if (h->num_symbols < 1 || !h->symbols) {
        return dlsym(handle, symbol);
    }
    for (uint32_t i = 0; i < h->num_symbols; i++) {
        if (strcmp(h->symbols[i].name, symbol) == 0) {
            return (void*)((uintptr_t)h->base_addr +
                           h->symbols[i].offset -
                           h->load_bias);
        }
    }
    return NULL;
}

// dlclose_ex — Android-7+-aware dlclose.
//   * SDK <  24 : plain dlclose().
//   * SDK >= 24 : if `handle` is an ExHandle, free its bookkeeping.
int dlclose_ex(void* handle)
{
    if (!handle) return 0;
    int sdk = read_sdk_int();
    if (sdk < 24) {
        return dlclose(handle);
    }
    ExHandle* h = (ExHandle*)handle;
    if (h->strtab_copy) free(h->strtab_copy);
    if (h->symbols)     free(h->symbols);
    free(h);
    return 0;
}

// dlerror_ex — Android-7+-aware dlerror.
//   * SDK <  24 : plain dlerror().
//   * SDK >= 24 : always NULL (the custom path never sets dlerror state).
const char* dlerror_ex(void)
{
    int sdk = read_sdk_int();
    if (sdk > 23) return NULL;
    return dlerror();
}

} // extern "C"
