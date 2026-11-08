"""#809 longer independent-order repeat for millisecond controls only."""
import argparse
import json
import os
from pathlib import Path
import sys
import tempfile

import profile_workloads as p
import sha256_pilot as s
import verify_sha256_pilot as v


def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('primary',type=Path)
    ap.add_argument('--output',type=Path,required=True)
    args=ap.parse_args()
    primary=json.loads(args.primary.read_text())
    v.verify(primary,identities=True)
    result=dict(primary_sha256=p.sha(args.primary.read_bytes()),runs=51,seed=8091,warmups=1,
                harness_sha256=p.sha(Path(__file__).read_bytes()),cases={})
    with tempfile.TemporaryDirectory(prefix='pythrust-809-controls-') as tmp:
        tmp=Path(tmp)
        datasets={}
        for name,dup in [('small_dup',True),('small_unique',False)]:
            directory=tmp/name
            manifest=p.generate_tree(directory,count=256,size=4096,duplicates=dup,nested=True)
            if manifest!=primary['inputs'][name]['manifest']:
                raise ValueError('fixture mismatch')
            datasets[name]=directory
        for name in ('startup','phase_discover','phase_read','fclone_tasks'):
            source=primary['cases'][name]['source']
            src=tmp/(name+'.pyr'); src.write_text(source)
            binaries={}
            for kind,item in primary['identities'].items():
                compiler=Path(item['checkout'])/'target/release/pythrust'
                work=tmp/(name+'-'+kind); work.mkdir()
                copy=work/'case.pyr'; copy.write_text(source)
                s.checked([compiler,'build',copy],b'',env=dict(os.environ,PYTHRUST_RT=str(compiler.parent/'libpythrust_rt.a')))
                binaries[kind]=copy.with_suffix('')
            cases=datasets.items() if name=='fclone_tasks' else [('default',None)] if name=='startup' else [('small_dup',datasets['small_dup'])]
            row=dict(source_sha256=p.sha(source.encode()),binary_sha256={k:p.sha(b.read_bytes()) for k,b in binaries.items()},executions={})
            for dataset,directory in cases:
                argv=[] if directory is None else [directory]
                want=s.checked([sys.executable,src,*argv])
                if name=='fclone_tasks' and want!=p.expected_groups(directory,primary['inputs'][dataset]['manifest']):
                    raise ValueError('group oracle mismatch')
                commands={k:[b,*argv] for k,b in binaries.items()}
                commands['cpython']=[sys.executable,src,*argv]
                for cpus in primary['affinities']:
                    key=dataset+'@'+cpus
                    row['executions'][key]=s.interleaved(commands,want,cpus,51,8091)
                    v.verify_timing(row['executions'][key],51,{'baseline','candidate','cpython'})
                    print(name,key,{k:round(x['median'],6) for k,x in row['executions'][key]['wall'].items()},flush=True)
            result['cases'][name]=row
            args.output.write_text(json.dumps(result,indent=2)+'\n')


if __name__=='__main__':
    main()
