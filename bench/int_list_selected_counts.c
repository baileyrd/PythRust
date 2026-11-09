/* #815 linked dispatch counts, separate from ordinary timing. */
#include <inttypes.h>
#include <stdint.h>
#include <stdatomic.h>
#include <stdio.h>
static _Atomic uint64_t reads, writes, typed_reads, typed_writes, holds, held_items;
#define GET(NAME, COUNT) \
extern uint64_t __real_##NAME(const void *, int64_t, unsigned char *); \
uint64_t __wrap_##NAME(const void *p, int64_t i, unsigned char *s) { \
    atomic_fetch_add(&COUNT, 1); return __real_##NAME(p, i, s); }
#define SET(NAME, COUNT) \
extern unsigned char __real_##NAME(const void *, int64_t, uint64_t); \
unsigned char __wrap_##NAME(const void *p, int64_t i, uint64_t v) { \
    atomic_fetch_add(&COUNT, 1); return __real_##NAME(p, i, v); }
GET(pt_seq_get, reads)
SET(pt_list_set, writes)
#ifdef TYPED
GET(pt_int_list_get, typed_reads)
SET(pt_int_list_set, typed_writes)
#endif
GET(pt_seq_item_held, held_items)
extern const void *__real_pt_seq_hold(const void *);
const void *__wrap_pt_seq_hold(const void *p) {
    atomic_fetch_add(&holds, 1); return __real_pt_seq_hold(p);
}
__attribute__((destructor)) static void report(void) {
    fprintf(stderr, "reads=%"PRIu64"\nwrites=%"PRIu64"\ntyped_reads=%"PRIu64"\ntyped_writes=%"PRIu64"\nholds=%"PRIu64"\nheld_items=%"PRIu64"\n",
        atomic_load(&reads), atomic_load(&writes), atomic_load(&typed_reads),
        atomic_load(&typed_writes), atomic_load(&holds), atomic_load(&held_items));
}
