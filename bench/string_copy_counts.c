/* #817 SERIAL link-only probe, never a release timing binary. Opaque pointers:
 * classify the returned allocation as object, all other scoped allocations as
 * buffer/scratch. Only linked memcpy/memmove edges are visible. No PtStr layout.
 * Live object lengths are derived from inputs and checked against buffer sizes.
 * Retire identities at actual free so address reuse cannot inherit a length.
 */
#include <inttypes.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

enum { NEW, INTEGER, INDEX, CONCAT0, ROWS=11 };
struct row { uint64_t calls, allocations, requested, object_bytes, buffer_bytes,
    buffer_copies, other_copies, left_bytes, right_bytes, result_bytes; uintptr_t site; };
static struct row rows[ROWS];
static uint64_t errors, outside_allocations, outside_requested, outside_copies, reallocations, scoped_reallocations;
static _Thread_local int scope=-1;
struct allocation { void *p; size_t n; uint64_t copies; };
static _Thread_local struct allocation allocations[16];
static _Thread_local size_t used;
struct object { void *p; size_t n; };
static _Thread_local struct object objects[128];
static _Thread_local size_t live;
extern void *__real_malloc(size_t);
extern void *__real_calloc(size_t,size_t);
extern void *__real_realloc(void *,size_t);
extern void __real_free(void *);
extern void *__real_memcpy(void *,const void *,size_t);
extern void *__real_memmove(void *,const void *,size_t);
static void allocated(void *p,size_t n) {
    if(!p) return;
    if(scope<0) {outside_allocations++;outside_requested+=n;return;}
    rows[scope].allocations++;rows[scope].requested+=n;
    if(used==16) {errors++;return;}
    allocations[used++]=(struct allocation){p,n,0};
}
void *__wrap_malloc(size_t n) {void *p=__real_malloc(n);allocated(p,n);return p;}
void *__wrap_calloc(size_t n,size_t s) {void *p=__real_calloc(n,s);allocated(p,n*s);return p;}
void *__wrap_realloc(void *p,size_t n) {void *r=__real_realloc(p,n);if(r) {reallocations++;if(scope<0)outside_requested+=n;else scoped_reallocations++;}return r;}
void __wrap_free(void *p) {
    for(size_t i=0;i<live;i++) if(objects[i].p==p) {objects[i]=objects[--live];break;}
    __real_free(p);
}
static void copied(void *p,size_t n) {
    if(scope<0) {outside_copies+=n;return;}
    uintptr_t d=(uintptr_t)p;
    for(size_t i=0;i<used;i++) {
        uintptr_t a=(uintptr_t)allocations[i].p;
        if(d>=a && d-a<=allocations[i].n && n<=allocations[i].n-(d-a)) {
            allocations[i].copies+=n;return;
        }
    }
    rows[scope].other_copies+=n;
}
void *__wrap_memcpy(void *p,const void *s,size_t n) {copied(p,n);return __real_memcpy(p,s,n);}
void *__wrap_memmove(void *p,const void *s,size_t n) {copied(p,n);return __real_memmove(p,s,n);}
static size_t length(const void *p) {
    for(size_t i=0;i<live;i++) if(objects[i].p==p) return objects[i].n;
    errors++;return 0;
}
static void begin(int id) {if(scope!=-1 || used) errors++;scope=id;rows[id].calls++;}
static void *finish(void *p,size_t n) {
    uint64_t buffers=0;
    for(size_t i=0;i<used;i++) {
        struct allocation a=allocations[i];
        if(a.p==p) {rows[scope].object_bytes+=a.n;rows[scope].other_copies+=a.copies;}
        else {buffers+=a.n;rows[scope].buffer_copies+=a.copies;}
    }
    rows[scope].buffer_bytes+=buffers;rows[scope].result_bytes+=n;
    if(!p || buffers!=n || live==128) errors++;
    else objects[live++]=(struct object){p,n};
    used=0;scope=-1;return p;
}
extern void *__real_pt_str_new(const unsigned char *,size_t);
void *__wrap_pt_str_new(const unsigned char *p,size_t n) {begin(NEW);return finish(__real_pt_str_new(p,n),n);}
extern void *__real_pt_str_from_int(int64_t);
void *__wrap_pt_str_from_int(int64_t n) {
    uint64_t m=n<0 ? 0-(uint64_t)n : (uint64_t)n;size_t len=n<0 ? 1 : 0;
    do {len++;m/=10;} while(m);
    begin(INTEGER);return finish(__real_pt_str_from_int(n),len);
}
extern void *__real_pt_str_index(const void *,int64_t);
void *__wrap_pt_str_index(const void *p,int64_t i) {
    begin(INDEX);void *r=__real_pt_str_index(p,i);
    size_t n=0;for(size_t k=0;k<used;k++) if(allocations[k].p!=r)n+=allocations[k].n;
    return finish(r,n);
}
extern void *__real_pt_str_concat(const void *,const void *);
void *__wrap_pt_str_concat(const void *a,const void *b) {
    uintptr_t site=(uintptr_t)__builtin_return_address(0);int id=CONCAT0;
    while(id<ROWS && rows[id].site && rows[id].site!=site)id++;
    if(id==ROWS) {errors++;id=CONCAT0;}
    rows[id].site=site;size_t left=length(a),right=length(b);
    begin(id);rows[id].left_bytes+=left;rows[id].right_bytes+=right;
    return finish(__real_pt_str_concat(a,b),left+right);
}
__attribute__((destructor)) static void report(void) {
    for(int i=0;i<ROWS;i++) {
        struct row r=rows[i];
        fprintf(stderr,"row%d=%"PRIu64",%"PRIu64",%"PRIu64",%"PRIu64",%"PRIu64",%"PRIu64",%"PRIu64",%"PRIu64",%"PRIu64",%"PRIu64",%"PRIuPTR"\n",i,r.calls,r.allocations,r.requested,r.object_bytes,r.buffer_bytes,r.buffer_copies,r.other_copies,r.left_bytes,r.right_bytes,r.result_bytes,r.site);
    }
    fprintf(stderr,"errors=%"PRIu64"\noutside_allocations=%"PRIu64"\noutside_requested=%"PRIu64"\noutside_copies=%"PRIu64"\nreallocations=%"PRIu64"\nscoped_reallocations=%"PRIu64"\n",errors,outside_allocations,outside_requested,outside_copies,reallocations,scoped_reallocations);
}
