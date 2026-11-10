/* #810 linked access counts only. Never use these runs for timing. */
#include <inttypes.h>
#include <stdint.h>
#include <stdatomic.h>
#include <stdio.h>
static _Atomic uint64_t reads, writes, holds, held_items;
extern uint64_t __real_pt_seq_get(const void *, int64_t, unsigned char *);
uint64_t __wrap_pt_seq_get(const void *p, int64_t i, unsigned char *s) {
    atomic_fetch_add(&reads, 1); return __real_pt_seq_get(p, i, s);
}
extern unsigned char __real_pt_list_set(const void *, int64_t, uint64_t);
unsigned char __wrap_pt_list_set(const void *p, int64_t i, uint64_t v) {
    atomic_fetch_add(&writes, 1); return __real_pt_list_set(p, i, v);
}
extern const void *__real_pt_seq_hold(const void *);
const void *__wrap_pt_seq_hold(const void *p) {
    atomic_fetch_add(&holds, 1); return __real_pt_seq_hold(p);
}
extern uint64_t __real_pt_seq_item_held(const void *, int64_t, unsigned char *);
uint64_t __wrap_pt_seq_item_held(const void *p, int64_t i, unsigned char *s) {
    atomic_fetch_add(&held_items, 1); return __real_pt_seq_item_held(p, i, s);
}
__attribute__((destructor)) static void report(void) {
    fprintf(stderr, "reads=%"PRIu64"\nwrites=%"PRIu64"\nholds=%"PRIu64"\nheld_items=%"PRIu64"\n",
        atomic_load(&reads), atomic_load(&writes), atomic_load(&holds), atomic_load(&held_items));
}
