/* #811 GNU linked-call diagnostic, NEVER timed. Misses inlined/internal copies.
 * Thread-local scopes + atomic counters permit task probes without data races. */
#include <inttypes.h>
#include <stddef.h>
#include <stdint.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
static _Atomic uint64_t heap, frees, reallocs, copies, conversions, decimal_heap, decimal_reallocs, decimal_copies;
static _Thread_local int inside;
static _Thread_local unsigned allocation;
static unsigned fail_index;
#define ADD(c,n) atomic_fetch_add_explicit(&(c),(n),memory_order_relaxed)
__attribute__((constructor)) static void configure(void) {
 const char *v=getenv("PYTHRUST_811_FAIL_ALLOC"); if(v) fail_index=(unsigned)atoi(v);
}
extern void *__real_malloc(size_t);
extern void *__real_calloc(size_t,size_t);
extern void __real_free(void *);
extern void *__real_realloc(void *,size_t);
extern void *__real_memcpy(void *,const void *,size_t);
extern void *__real_memmove(void *,const void *,size_t);
void *__wrap_malloc(size_t n) {
 if(inside && ++allocation==fail_index) return NULL;
 void *v=__real_malloc(n); if(v) { ADD(heap,1); if(inside) ADD(decimal_heap,1); } return v;
}
void *__wrap_calloc(size_t n,size_t s) { void *v=__real_calloc(n,s); if(v) { ADD(heap,1); if(inside) ADD(decimal_heap,1); } return v; }
void __wrap_free(void *v) { if(v) ADD(frees,1); __real_free(v); }
void *__wrap_realloc(void *v,size_t n) { void *r=__real_realloc(v,n); if(r) { ADD(reallocs,1); if(inside) ADD(decimal_reallocs,1); } return r; }
void *__wrap_memcpy(void *d,const void *s,size_t n) { ADD(copies,n); if(inside) ADD(decimal_copies,n); return __real_memcpy(d,s,n); }
void *__wrap_memmove(void *d,const void *s,size_t n) { ADD(copies,n); if(inside) ADD(decimal_copies,n); return __real_memmove(d,s,n); }
extern void *__real_pt_str_from_int(int64_t);
void *__wrap_pt_str_from_int(int64_t n) {
 ADD(conversions,1); inside=1; allocation=0; void *v=__real_pt_str_from_int(n); inside=0; return v;
}
#define OUT(k,c) fprintf(stderr,k "=%"PRIu64"\n",atomic_load_explicit(&(c),memory_order_relaxed))
__attribute__((destructor)) static void report(void) {
 OUT("heap_allocations",heap); OUT("heap_frees",frees); OUT("heap_reallocations",reallocs); OUT("copy_bytes",copies);
 OUT("conversions",conversions); OUT("decimal_allocations",decimal_heap); OUT("decimal_reallocations",decimal_reallocs); OUT("decimal_copy_bytes",decimal_copies);
}
