/* Linker-only #806 probe. Never linked into ordinary production builds.
 * Counts runtime two-int tuple constructions, not allocator invocations:
 * each boxed tuple owns both a PtSeq allocation and its slot vector.
 * GNU ld --wrap redirects generated calls; no runtime source change.
 */
#include <inttypes.h>
#include <stdatomic.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

static _Atomic uint64_t pairs, allocations, reallocations;
extern void *__real_malloc(size_t);
extern void *__real_calloc(size_t, size_t);
extern void *__real_realloc(void *, size_t);
void *__wrap_malloc(size_t size) {
    void *result = __real_malloc(size);
    if (result) atomic_fetch_add_explicit(&allocations, 1, memory_order_relaxed);
    return result;
}
void *__wrap_calloc(size_t count, size_t size) {
    void *result = __real_calloc(count, size);
    if (result) atomic_fetch_add_explicit(&allocations, 1, memory_order_relaxed);
    return result;
}
void *__wrap_realloc(void *ptr, size_t size) {
    void *result = __real_realloc(ptr, size);
    if (result) atomic_fetch_add_explicit(&reallocations, 1, memory_order_relaxed);
    return result;
}
extern void *__real_pt_seq_new(const unsigned char *, size_t, int64_t);
void *__wrap_pt_seq_new(const unsigned char *desc, size_t len, int64_t cap) {
    void *result = __real_pt_seq_new(desc, len, cap);
    if (result && len == 4 && desc[0] == 't' && desc[1] == 2 &&
        desc[2] == 'i' && desc[3] == 'i') {
        atomic_fetch_add_explicit(&pairs, 1, memory_order_relaxed);
    }
    return result;
}
__attribute__((destructor)) static void report(void) {
    fprintf(stderr, "heap_allocations=%" PRIu64 "\nheap_reallocations=%" PRIu64 "\n",
            atomic_load_explicit(&allocations, memory_order_relaxed),
            atomic_load_explicit(&reallocations, memory_order_relaxed));
    fprintf(stderr, "pair_tuple_allocations=%" PRIu64 "\n",
            atomic_load_explicit(&pairs, memory_order_relaxed));
}
