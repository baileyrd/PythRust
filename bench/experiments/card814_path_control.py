"""Diagnostic: identical source path isolates binary-layout/launch controls.

Build before timing; byte-identical pairs are explicitly a negative/self control,
never counted as an optimization gain or substituted for the primary workloads.
"""
import json
import os
from pathlib import Path
import shutil
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import three_way as b

base,candidate,work=map(Path,sys.argv[1:])
work.mkdir()
pair={k:json.loads((d/'prepared.json').read_text()) for k,d in [('baseline',base),('candidate',candidate)]}
record=dict(status='preparing',prepared=pair,harness=b.identity(Path(__file__)),host=b.host(),command=sys.argv,cases={},runs=42,seed=814000,warmups=2)
b.pin(pair['baseline']['cpu'])
for name in ('strings','tuples'):
    source=work/(name+'.pyr');source.write_text(pair['baseline']['sources'][name]['pythrust_source'])
    row=dict(source=b.identity(source),binaries={},build_commands={},order=b.order(['baseline','candidate'],42,814000+len(record['cases'])),samples=[],warmup_samples=[],summary={})
    for label,p in pair.items():
        command=[p['compiler']['path'],'build',str(source)]
        b.run(command,env=dict(os.environ,PYTHRUST_RT=p['runtime']['path']))
        target=work/(name+'-'+label);shutil.copy2(source.with_suffix(''),target)
        row['binaries'][label]=b.identity(target);row['build_commands'][label]=command
    row['byte_identical']=row['binaries']['baseline']['sha256']==row['binaries']['candidate']['sha256']
    record['cases'][name]=row
record['status']='running'
b.save(work/'results.json',record)
for name,row in record['cases'].items():
    for label,item in row['binaries'].items():
        for i in range(2):row['warmup_samples'].append(dict(kind=label,warmup=i,**b.sample([item['path']],b.EXPECTED[name])))
    for i,sequence in enumerate(row['order']):
        for label in sequence:
            row['samples'].append(dict(round=i,kind=label,**b.sample([row['binaries'][label]['path']],b.EXPECTED[name])))
        b.save(work/'results.json',record)
    row['summary']={label:b.stats([x['wall_seconds'] for x in row['samples'] if x['kind']==label]) for label in pair}
    row['post_binaries']={k:b.identity(Path(v['path'])) for k,v in row['binaries'].items()}
    if row['post_binaries']!=row['binaries']:raise ValueError('changed binaries')
    print(name,'byte-identical',row['byte_identical'],row['summary'],flush=True)
record['status']='complete';b.save(work/'results.json',record)
