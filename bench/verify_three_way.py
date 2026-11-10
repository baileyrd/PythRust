"""Verify complete #813 raw evidence, including sources and preparation records."""
import json
from pathlib import Path
import re
import sys
import three_way as b


def hash_record(item):
    if not isinstance(item['path'],str) or not re.fullmatch('[0-9a-f]{64}',item['sha256']):
        raise ValueError('malformed identity')


def verify_invocation(prepared, name, kind):
    item = prepared['conditions'][name][kind]
    hash_record(item['executable'])
    source = prepared['sources'][name]['pythrust']['path']
    expected = [item['executable']['path']]
    if kind == 'cpython':
        host = prepared['host']
        if item['executable'] != host['python_executable_identity']:
            raise ValueError('unmatched CPython identity')
        # sys.executable may be a symlink or virtualenv entry point, while
        # executable identities intentionally record its resolved target.
        expected = [host['python_executable'], '-B', source]
    elif kind == 'pythrust' and item['executable']['path'] != str(Path(source).with_suffix('')):
        raise ValueError('unmatched native source/output')
    if item['command'] != expected:
        raise ValueError('invocation does not match executable/source/arguments')


def verify_builds(prepared):
    if prepared['rust_flags'] != b.FLAGS:
        raise ValueError('unoptimized Rust metadata')
    compiler = prepared['compiler']['path']
    rustc = prepared['rustc_identity']['path']
    for name, sources in prepared['sources'].items():
        conditions = prepared['conditions'][name]
        expected = [compiler, 'build', sources['pythrust']['path']]
        if prepared['builds'][name+'/pythrust']['command'] != expected:
            raise ValueError('unmatched PythRust build')
        for kind, item in conditions.items():
            if not kind.startswith('rust'):
                continue
            suffix = kind.replace('_', '-')
            output = str(Path(sources['rust']['path']).with_name(name+'-'+suffix))
            if item['executable']['path'] != output:
                raise ValueError('unmatched Rust source/output')
            flags = b.FLAGS.copy()
            if kind == 'rust_checked':
                flags[flags.index('overflow-checks=off')] = 'overflow-checks=on'
            if kind == 'rust_materialized':
                flags += ['--cfg', 'materialize']
            expected = [rustc, *flags, sources['rust']['path'], '-o', item['executable']['path']]
            if kind == 'rust':
                actual = prepared['builds'][name+'/rust']['command']
            else:
                # Older schema-2 captures omitted control build commands. Do not
                # manufacture provenance for them; new preparations retain it.
                actual = item.get('build_command')
                if actual is None:
                    if prepared.get('control_build_protocol') == 1:
                        raise ValueError('missing control build command')
                    continue
            if actual != expected:
                raise ValueError('unmatched optimized Rust build')


def verify(record):
    count=b.verify(record)
    p=record['prepared']
    if set(p['sources'])!=set(b.EXPECTED) or set(p['conditions'])!=set(b.EXPECTED):
        raise ValueError('incomplete preparation')
    for key in ('compiler','runtime','harness'): hash_record(p[key])
    if not isinstance(p['cpu'],int) or p['cpu'] not in p['host']['permitted_affinity']:
        raise ValueError('invalid prepared affinity')
    if not re.fullmatch('[0-9a-f]{40}',p['source']['head']) or not re.fullmatch('[0-9a-f]{40}',p['source']['tree']):
        raise ValueError('bad source revision')
    if set(p['builds'])!={name+'/'+kind for name in b.EXPECTED for kind in ('pythrust','rust')}:
        raise ValueError('missing compilation samples')
    for row in p['builds'].values():
        if len(row['samples'])!=7 or not row['command']: raise ValueError('bad build samples')
        computed=b.stats(row['samples'])
        if any(row[key]!=value for key,value in computed.items()): raise ValueError('bad build stats')
    for name,sources in p['sources'].items():
        for kind in ('pythrust','rust'):
            hash_record(sources[kind])
            if b.digest(sources[kind+'_source'].encode())!=sources[kind]['sha256']:
                raise ValueError('source bytes do not match identity')
        original=b'pass\n' if name=='startup' else (b.ROOT/'bench'/(name+'.pyr')).read_bytes()
        if b.digest(original)!=sources['pythrust']['sha256']:
            raise ValueError('timed source differs from original')
        kinds=set(b.KINDS)
        if name!='startup': kinds.add('rust_checked')
        if name=='strings': kinds.add('rust_materialized')
        if set(p['conditions'][name])!=kinds: raise ValueError('wrong condition set')
        for kind in p['conditions'][name]:
            verify_invocation(p, name, kind)
    for field in ('rust_tests','interpreter_checks'):
        if set(p[field])!=set(b.EXPECTED)-{'startup'}: raise ValueError('missing correctness checks')
    if any('1 passed; 0 failed' not in s for s in p['rust_tests'].values()):
        raise ValueError('Rust boundary tests did not pass')
    if record['warmups']!=2: raise ValueError('wrong warmup count')
    if record['schema']==2: verify_current(record)
    return count


def verify_current(record):
    p=record['prepared']
    if record['warmups']!=2 or p['schema']!=2: raise ValueError('new protocol required')
    for key in ('compiler','runtime','harness'):
        if record['post_identities'][key]!=p[key]: raise ValueError('identity changed during timing')
    for key in ('status','index_diff','tracked_diff','build_inputs'):
        if key not in p['source']: raise ValueError('missing source state')
    if not p['source']['build_inputs']: raise ValueError('missing build inputs')
    for value in p['source']['build_inputs'].values():
        hash_record(dict(path='build input',sha256=value))
    for key in ('environment','tools','rustc_identity','domain'):
        if not p[key]: raise ValueError('missing environment/provenance')
    hash_record(p['rustc_identity'])
    verify_builds(p)
    for key in ('cargo','cc','ld'): hash_record(p['tools'][key])
    host=p['host']
    for key in ('platform','libc','python','python_executable_identity','load','files','caveats'):
        if not host[key]: raise ValueError('missing host resources')
    if type(p['cpu']) is not int or any(type(x) is not int or x<0 for x in host['permitted_affinity']):
        raise ValueError('invalid CPU IDs')
    for name,sources in p['sources'].items():
        original=(b.ROOT/'bench/rust'/(name+'.rs')).read_bytes()
        if b.digest(original)!=sources['rust']['sha256']: raise ValueError('Rust source differs from pinned workload')
        cond=p['conditions'][name]
        if cond['pythrust']['executable']['path']==cond['rust']['executable']['path']:
            raise ValueError('self comparison')
        if cond['cpython']['executable']!=host['python_executable_identity']:
            raise ValueError('unmatched CPython')
    for key,row in record['cases'].items():
        name,label=key.split('/')
        kinds=list(b.KINDS) if label=='primary' else ['rust','rust_'+label]
        wanted=[(kind,i) for kind in kinds for i in range(2)]
        if [(x['kind'],x['warmup']) for x in row['warmup_samples']]!=wanted: raise ValueError('missing warmups')
        for x in row['warmup_samples']:
            if x['returncode']!=0 or x['stderr'] or x['stdout']!=b.EXPECTED[name]: raise ValueError('warmup failed')
            b.stats([x['wall_seconds']])


if __name__=='__main__':
    print('Verified complete cases:',verify(json.loads(Path(sys.argv[1]).read_text())))
