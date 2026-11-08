/* #807 GNU linker probe, NEVER linked into timed/production binaries.
 * Counts successful linked allocator calls (not libc's internal calls).
 * Scoped memcpy/memmove measures actual runtime concat copy traffic, not
 * a size-derived estimate. PtStr layout is deliberately not inspected.
 */
#include <inttypes.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
static uint64_t objects, fused, heap, reallocs, concat_heap, concat_reallocs, copied;
static int fail_allocation;
__attribute__((constructor)) static void configure(void) {
    fail_allocation = getenv("PYTHRUST_807_FAIL_ALLOC") != NULL;
}
static _Thread_local int inside;
struct allocation { void *ptr; size_t size; uint64_t copied; };
static _Thread_local struct allocation active[8];
static _Thread_local size_t used;
static uint64_t probe_overflow;
static void remember(void *p,size_t n) {
    if(!inside || !p) return;
    if(used == 8) { probe_overflow++; return; }
    active[used++] = (struct allocation){p,n,0};
}
static void copying(void *d,size_t n) {
    if(!inside) return;
    uintptr_t addr=(uintptr_t)d;
    for(size_t i=0;i<used;i++) {
        uintptr_t base=(uintptr_t)active[i].ptr;
        if(addr>=base && addr-base<=active[i].size && n<=active[i].size-(addr-base)) {
            active[i].copied+=n;
            return;
        }
    }
}
static void finish(void *object) {
    /* Exclude stack traffic and the returned object's construction;
     * count actual copies into the separately allocated text buffers. */
    for(size_t i=0;i<used;i++)
        if(active[i].ptr != object) copied+=active[i].copied;
    used=0;
}
extern void *__real_malloc(size_t);
extern void *__real_calloc(size_t,size_t);
extern void *__real_realloc(void *,size_t);
extern void *__real_memcpy(void *,const void *,size_t);
extern void *__real_memmove(void *,const void *,size_t);
void *__wrap_malloc(size_t n) {
    if(inside && fail_allocation) return NULL;
    void *p=__real_malloc(n);
    if(p) { heap++; if(inside) concat_heap++; }
    remember(p,n);
    return p;
}
void *__wrap_calloc(size_t n,size_t s) {
    void *p=__real_calloc(n,s);
    if(p) { heap++; if(inside) concat_heap++; }
    remember(p,n*s);
    return p;
}
void *__wrap_realloc(void *p,size_t n) {
    void *r=__real_realloc(p,n);
    if(r) { reallocs++; if(inside) concat_reallocs++; }
    return r;
}
void *__wrap_memcpy(void *d,const void *s,size_t n) {
    copying(d,n);
    return __real_memcpy(d,s,n);
}
void *__wrap_memmove(void *d,const void *s,size_t n) {
    copying(d,n);
    return __real_memmove(d,s,n);
}
extern void *__real_pt_str_new(const unsigned char *,size_t);
void *__wrap_pt_str_new(const unsigned char *s,size_t n) {
    if(fail_allocation && n==5 && s[0]=='L' && s[1]=='A' && s[2]=='T' && s[3]=='E' && s[4]=='R') {
        static const char marker[]="LATER_OPERAND_EXECUTED\n";
        (void)write(2,marker,sizeof(marker)-1);
    }
    return __real_pt_str_new(s,n);
}
extern void *__real_pt_str_concat(const void *,const void *);
void *__wrap_pt_str_concat(const void *a,const void *b) {
    inside++; void *p=__real_pt_str_concat(a,b); inside--; finish(p); objects++; return p;
}
/* Weak so the same probe also links against the pre-#807 archive. */
extern void *__real_pt_str_concat3(const void *,const void *,const void *) __attribute__((weak));
void *__wrap_pt_str_concat3(const void *a,const void *b,const void *c) {
    inside++; void *p=__real_pt_str_concat3(a,b,c); inside--; finish(p); objects++; fused++; return p;
}
__attribute__((destructor)) static void report(void) {
    fprintf(stderr,"concat_objects=%"PRIu64"\nfused_calls=%"PRIu64"\nheap_allocations=%"PRIu64"\nheap_reallocations=%"PRIu64"\nconcat_allocations=%"PRIu64"\nconcat_reallocations=%"PRIu64"\nconcat_copied_bytes=%"PRIu64"\nprobe_overflow=%"PRIu64"\n",
    objects,fused,heap,reallocs,concat_heap,concat_reallocs,copied,probe_overflow);
}
