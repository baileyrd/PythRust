/* #808 link-only diagnostic. Atomic counters are deliberately NOT timing probes.
 * Linked call edges only: inlined/same-object/internal libc operations are missed.
 * Counting adds shared atomic traffic; do not infer contention from probe wall time.
 */
#include <inttypes.h>
#include <stddef.h>
#include <stdint.h>
#include <stdatomic.h>
#include <stdio.h>
static _Atomic uint64_t heap, reallocs, requested, copies;
static _Atomic uint64_t str_inc, str_dec, seq_inc, seq_dec;
static _Atomic uint64_t seq_get, seq_get_held, seq_item_held, seq_hold, submit;
#define ADD(c,n) atomic_fetch_add_explicit(&(c),(n),memory_order_relaxed)
extern void *__real_malloc(size_t);
extern void *__real_calloc(size_t,size_t);
extern void *__real_realloc(void *,size_t);
extern void *__real_memcpy(void *,const void *,size_t);
extern void *__real_memmove(void *,const void *,size_t);
void *__wrap_malloc(size_t n) { void *p=__real_malloc(n); if(p) { ADD(heap,1); ADD(requested,n); } return p; }
void *__wrap_calloc(size_t n,size_t s) { void *p=__real_calloc(n,s); if(p) { ADD(heap,1); ADD(requested,n*s); } return p; }
void *__wrap_realloc(void *p,size_t n) { void *r=__real_realloc(p,n); if(r) { ADD(reallocs,1); ADD(requested,n); } return r; }
void *__wrap_memcpy(void *d,const void *s,size_t n) { ADD(copies,n); return __real_memcpy(d,s,n); }
void *__wrap_memmove(void *d,const void *s,size_t n) { ADD(copies,n); return __real_memmove(d,s,n); }
#define VOID_WRAP(name,counter) \
 extern void __real_##name(void *); \
 void __wrap_##name(void *p) { ADD(counter,1); __real_##name(p); }
VOID_WRAP(pt_str_inc,str_inc)
VOID_WRAP(pt_str_dec,str_dec)
VOID_WRAP(pt_seq_inc,seq_inc)
VOID_WRAP(pt_seq_dec,seq_dec)
extern uint64_t __real_pt_seq_get(const void *,int64_t,unsigned char *);
uint64_t __wrap_pt_seq_get(const void *p,int64_t i,unsigned char *s) { ADD(seq_get,1); return __real_pt_seq_get(p,i,s); }
extern uint64_t __real_pt_seq_get_held(const void *,int64_t,unsigned char *);
uint64_t __wrap_pt_seq_get_held(const void *p,int64_t i,unsigned char *s) { ADD(seq_get_held,1); return __real_pt_seq_get_held(p,i,s); }
extern uint64_t __real_pt_seq_item_held(const void *,int64_t,unsigned char *);
uint64_t __wrap_pt_seq_item_held(const void *p,int64_t i,unsigned char *s) { ADD(seq_item_held,1); return __real_pt_seq_item_held(p,i,s); }
extern const void *__real_pt_seq_hold(const void *);
const void *__wrap_pt_seq_hold(const void *p) { ADD(seq_hold,1); return __real_pt_seq_hold(p); }
extern void *__real_pt_submit(void *,void *,const uint64_t *,size_t,const unsigned char *,size_t);
void *__wrap_pt_submit(void *e,void *f,const uint64_t *a,size_t n,const unsigned char *d,size_t len) { ADD(submit,1); return __real_pt_submit(e,f,a,n,d,len); }
#define OUT(label,c) fprintf(stderr,label "=%"PRIu64"\n",atomic_load_explicit(&(c),memory_order_relaxed))
__attribute__((destructor)) static void report(void) {
 OUT("heap_allocations",heap); OUT("heap_reallocations",reallocs); OUT("requested_bytes",requested); OUT("copy_bytes",copies);
 OUT("str_inc",str_inc); OUT("str_dec",str_dec); OUT("seq_inc",seq_inc); OUT("seq_dec",seq_dec);
 OUT("seq_get",seq_get); OUT("seq_get_held",seq_get_held); OUT("seq_item_held",seq_item_held); OUT("seq_hold",seq_hold); OUT("submit",submit);
}
