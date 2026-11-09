"""#817 frozen evidence mutation tests, independent of future production files."""
import copy
import unittest
from unittest.mock import patch
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import string_copy_campaign as c


class Evidence(unittest.TestCase):
    def test_no_future_checkout_dependency(self):
        with patch.object(c,'ROOT',Path('/nonexistent/future-checkout')), patch.object(c,'sources',side_effect=AssertionError('future input read')):
            self.assertEqual(c.verify_saved(),12)

    def test_saved_bundle(self):
        self.assertEqual(c.verify_saved(), 12)
        self.assertEqual(c.verify_validation(),9)

    def test_mutations(self):
        original = c.load_bundle()
        def rehashed_missing_wrap(x):
            r=x['probe'];r['count_linker_source']=r['count_linker_source'].replace('-Wl,--wrap=malloc','')
            r['count_linker']['sha256']=c.b.digest(r['count_linker_source'].encode())
        def rehashed_missing_pg(x):
            r=x['probe'];r['pg_linker_source']=r['pg_linker_source'].replace('-pg','')
            r['pg_linker']['sha256']=c.b.digest(r['pg_linker_source'].encode())
        mutations = {
            'rehashed missing wrap':rehashed_missing_wrap,
            'rehashed missing pg':rehashed_missing_pg,
            'symmetric omitted input': lambda x: [x['preparation']['inputs'].pop(next(iter(x['preparation']['inputs']))) for _ in [0]],
            'omitted tool': lambda x: x['preparation']['tools'].pop('ld'),
            'compiler substitution': lambda x: x['preparation']['compiler'].update(sha256='0'*64),
            'source digest substitution': lambda x: x['preparation']['inputs'][next(iter(x['preparation']['inputs']))].update(sha256='0'*64),
            'workload omission': lambda x: x['preparation']['cases'].pop('strings'),
            'tree substitution': lambda x: x['preparation'].update(tree='0'*40),
            'artifact substitution': lambda x: x['probe']['count_object'].update(sha256='0'*64),
            'missing profile hash': lambda x: x['probe']['gmon'][0].pop('sha256'),
            'malformed profile hash': lambda x: x['probe']['gmon'][0].update(sha256='abc'),
            'changed flags': lambda x: x['probe']['cc_command'].remove('-fno-builtin'),
            'rehashed wrapper': lambda x: x['probe'].update(count_linker_source='bad', count_linker={'path':x['probe']['count_linker']['path'],'sha256':c.b.digest(b'bad')}),
            'duplicate execution': lambda x: x['probe']['samples'].__setitem__(1,copy.deepcopy(x['probe']['samples'][0])),
            'cross bundle': lambda x: x['probe']['compiler'].update(path='/tmp/another/compiler'),
            'changed timing command': lambda x: x['timings'][0]['cases']['strings']['commands']['baseline'].append('--changed'),
            'missing warmup': lambda x: x['timings'][0]['cases']['strings']['warmups']['baseline'].pop(),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                record = copy.deepcopy(original)
                mutate(record)
                with self.assertRaises((ValueError,KeyError,TypeError)):
                    c.verify(record)

    def test_semantic_profile_and_command_guards(self):
        # Also exercise guards before the final immutable-record digest check.
        for mutate in (
            lambda x: x['preparation']['tools'].pop('ld'),
            lambda x: x['probe']['cc_command'].remove('-fno-builtin'),
            lambda x: x['probe']['gmon'][0].pop('sha256'),
            lambda x: x['probe']['samples'].__setitem__(1,copy.deepcopy(x['probe']['samples'][0])),
            lambda x: x['probe'].update(pg_linker_source='bad',pg_linker={'path':x['probe']['pg_linker']['path'],'sha256':c.b.digest(b'bad')}),
            lambda x: x['probe']['runtime'].update(path='/tmp/substitute'),
        ):
            data=copy.deepcopy(c.load_bundle());mutate(data)
            with self.assertRaises((ValueError,KeyError,TypeError)):c.validate(data)

    def test_parser_rejects_duplicates_missing_negative(self):
        for text in ('x=1\nx=1\n','x=-1\n','x=1\n'):
            with self.assertRaises(ValueError):
                c.parse_counts(text)


@unittest.skipUnless(sys.platform.startswith('linux') and shutil.which('cc'), 'GNU Linux link-wrap probe')
class ProbeCalibration(unittest.TestCase):
    def test_opaque_objects_buffers_empty_unicode_and_reuse(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root/'runtime.c').write_text(r"""
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
struct object {char *bytes; size_t length;};
void *pt_str_new(const unsigned char *s,size_t n) {
    struct object *p=malloc(sizeof(*p));p->bytes=n?malloc(n):NULL;p->length=n;
    if(n)memcpy(p->bytes,s,n);return p;
}
void drop(void *v) {struct object *p=v;free(p->bytes);free(p);}
void *pt_str_from_int(int64_t n) {(void)n;return pt_str_new((const unsigned char *)"12",2);}
void *pt_str_index(const void *v,int64_t i) {
    const struct object *p=v;(void)i;return pt_str_new((const unsigned char *)p->bytes,1);
}
void *pt_str_concat(const void *a,const void *b) {
    const struct object *x=a,*y=b;size_t n=x->length+y->length;
    struct object *p=malloc(sizeof(*p));p->length=n;p->bytes=n?malloc(n):NULL;
    if(x->length)memcpy(p->bytes,x->bytes,x->length);
    if(y->length)memcpy(p->bytes+x->length,y->bytes,y->length);return p;
}
""")
            (root/'main.c').write_text(r"""
#include <stddef.h>
#include <stdint.h>
void *pt_str_new(const unsigned char *,size_t);
void *pt_str_concat(const void *,const void *);
void *pt_str_from_int(int64_t);
void *pt_str_index(const void *,int64_t);
void drop(void *);
int main(void) {
    for(int i=0;i<100;i++) {
        void *a=pt_str_new((const unsigned char *)"\xc3\xa9",2);
        void *e=pt_str_new((const unsigned char *)"",0);
        void *b=pt_str_from_int(12);
        void *x=pt_str_concat(a,b);void *y=pt_str_concat(e,x);
        void *z=pt_str_index(b,0);
        drop(a);drop(e);drop(b);drop(x);drop(y);drop(z);
    }
    return 0;
}
""")
            obj = root/'probe.o'
            subprocess.run(['cc','-std=c11','-O2','-fno-builtin','-Wall','-Wextra','-Werror','-c',str(c.ROOT/'bench/string_copy_counts.c'),'-o',str(obj)],check=True,capture_output=True)
            binary=root/'calibration'
            subprocess.run(['cc','-std=c11','-O0','-fno-builtin',str(root/'runtime.c'),str(root/'main.c'),str(obj),'-no-pie',*['-Wl,--wrap='+n for n in c.WRAPS],'-o',str(binary)],check=True,capture_output=True)
            done=subprocess.run([binary],capture_output=True,check=True)
            counts=c.parse_counts(done.stderr.decode())
            self.assertEqual(counts['errors'],0)
            self.assertEqual(counts['scoped_reallocations'],0)
            self.assertEqual(counts['row0']['calls'],200)
            self.assertEqual(counts['row0']['buffer_bytes'],200)
            self.assertEqual(counts['row1']['buffer_bytes'],200)
            self.assertEqual(counts['row2']['buffer_bytes'],100)
            self.assertEqual(sum(counts['row'+str(i)]['buffer_copies'] for i in range(3,11)),800)
            self.assertEqual(sum(counts['row'+str(i)]['object_bytes'] for i in range(11)),600*16)


if __name__ == '__main__':
    unittest.main()
