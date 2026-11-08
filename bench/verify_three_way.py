"""Verify complete #813 raw evidence, including sources and preparation records."""
import json
from pathlib import Path
import re
import sys
import three_way as b


def hash_record(item):
    if not isinstance(item['path'],str) or not re.fullmatch('[0-9a-f]{64}',item['sha256']):
        raise ValueError('malformed identity')


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
        for item in p['conditions'][name].values():
            hash_record(item['executable'])
            if not item['command']: raise ValueError('missing command')
    for field in ('rust_tests','interpreter_checks'):
        if set(p[field])!=set(b.EXPECTED)-{'startup'}: raise ValueError('missing correctness checks')
    if any('1 passed; 0 failed' not in s for s in p['rust_tests'].values()):
        raise ValueError('Rust boundary tests did not pass')
    if record['warmups']!=2: raise ValueError('wrong warmup count')
    return count


if __name__=='__main__':
    print('Verified complete cases:',verify(json.loads(Path(sys.argv[1]).read_text())))
