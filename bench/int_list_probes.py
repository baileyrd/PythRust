"""#815 separate linked counts/PC profiles. Retains local gmon and binaries."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import int_list_campaign as c
import three_way as b
import profile_workloads as p

NAMES=('sorting','int_access','float_access','generic_float')
COUNTERS={'reads','writes','typed_reads','typed_writes','holds','held_items'}


def parse(raw):
    result={}
    for line in raw.splitlines():
        key,value=line.split('=')
        if key in result or not value.isascii() or not value.isdecimal(): raise ValueError('bad counter')
        result[key]=int(value)
    if set(result)!=COUNTERS: raise ValueError('missing counters')
    return result


def collect(work,output):
    if output.exists(): raise FileExistsError(output)
    root=work/'probes';root.mkdir()
    prepared={k:json.loads((work/k/'prepared.json').read_text()) for k in c.LABELS}
    cpu=prepared['baseline']['cpu']; b.pin(cpu)
    data=dict(status='running',cc=shutil.which('cc'),host=b.host(),command=sys.argv,prepared=prepared,
              harness=b.identity(Path(__file__).resolve()),probe=b.identity(b.ROOT/'bench/int_list_selected_counts.c'),
              tools={k:dict(identity=b.identity(Path(shutil.which(k)).resolve()),version=b.run([k,'--version']).stdout.decode()) for k in ('cc','ld','objdump','gprof')},cases={})
    for label,manifest in prepared.items():
        data['cases'][label]={}
        for name in NAMES:
            folder=root/(label+'-'+name);folder.mkdir()
            src=folder/(name+'.pyr');src.write_text(c.sources()[name])
            command=[manifest['compiler']['path'],'build',str(src)]
            obj=folder/'counts.o'; cc=shutil.which('cc')
            cmd=[cc,'-std=c11','-O2','-fno-builtin','-Wall','-Wextra','-Werror']
            if label=='candidate': cmd+=['-DTYPED=1']
            cmd+=['-c',str(b.ROOT/'bench/int_list_selected_counts.c'),'-o',str(obj)]
            b.run(cmd)
            wraps=['pt_seq_get','pt_list_set','pt_seq_hold','pt_seq_item_held']
            if label=='candidate': wraps+=['pt_int_list_get','pt_int_list_set']
            env=p.write_linker(folder/'count-linker',cc,[str(obj)]+['-Wl,--wrap='+n for n in wraps])
            env['PYTHRUST_RT']=manifest['runtime']['path']
            b.run(command,env=env)
            binary=src.with_suffix('');counts_binary=folder/'counts';shutil.copy2(binary,counts_binary)
            done=b.run([counts_binary])
            if done.stdout.decode()!=manifest['cases'][name]['expected']:raise ValueError('count output')
            row=dict(source=b.identity(src),count_build=command,count_cc=cmd,count_object=b.identity(obj),
                     count_linker=b.identity(folder/'count-linker/cc'),count_linker_source=(folder/'count-linker/cc').read_text(),
                     count_binary=b.identity(counts_binary),count_command=[str(counts_binary)],
                     raw=done.stderr.decode(),counts=parse(done.stderr.decode()),stdout=done.stdout.decode(),returncode=done.returncode)
            env=p.write_linker(folder/'pg-linker',cc,['-pg','-no-pie']);env['PYTHRUST_RT']=manifest['runtime']['path']
            b.run(command,env=env)
            profile_binary=folder/'profile';shutil.copy2(binary,profile_binary)
            paths=[];samples=[]
            for i in range(7):
                env=dict(os.environ,GMON_OUT_PREFIX=str(folder/('gmon-'+str(i))))
                cmd=[str(profile_binary)]
                samples.append(b.sample(cmd,manifest['cases'][name]['expected'],env=env))
                found=list(folder.glob('gmon-'+str(i)+'.*'))
                if len(found)!=1:raise ValueError('missing gmon')
                paths+=found
            gprof_cmd=['gprof','-b','-p',str(profile_binary),*map(str,paths)]
            report=b.run(gprof_cmd).stdout.decode()
            if 'no time accumulated' in report:raise ValueError('empty PC profile')
            row.update(profile_binary=b.identity(profile_binary),profile_build=command,
                pg_linker=b.identity(folder/'pg-linker/cc'),pg_linker_source=(folder/'pg-linker/cc').read_text(),
                profile_command=[str(profile_binary)],profile_samples=samples,gmon=[b.identity(x) for x in paths],gprof_command=gprof_cmd,flat_profile=report)
            asm=b.run(['objdump','-d',profile_binary]).stdout
            (folder/'generated.s').write_bytes(asm)
            row['assembly']=b.identity(folder/'generated.s')
            data['cases'][label][name]=row;b.save(output,data)
    data['status']='complete';verify(data);b.save(output,data)


def linker_source(python, cc, extra):
    return ('#!'+python+'\nimport subprocess,sys,time\n'
            'start=time.perf_counter()\np=subprocess.run('+repr([cc])+
            '+sys.argv[1:]+'+repr(extra)+')\nsys.exit(p.returncode)\n')


def verify(data, canonical=None):
    c.verify_prepared(data['prepared'],c.canonical_prepared() if canonical is None else canonical)
    fixed=c.frozen()['probe']
    c.valid_tools(data['tools'],('cc','ld','objdump','gprof'))
    if data['tools']!=fixed['tools'] or data['cc']!=fixed['cc']:raise ValueError('unmatched probe tools')
    for key in ('probe','harness'):
        c.valid_identity(data[key])
        if data[key]!=fixed[key]:raise ValueError('unmatched probe source/harness')
    if data['status']!='complete' or set(data['cases'])!=set(c.LABELS): raise ValueError('incomplete probes')
    for label,rows in data['cases'].items():
        if set(rows)!=set(NAMES):raise ValueError('probe cases')
        for name,row in rows.items():
            manifest=data['prepared'][label];case=manifest['cases'][name]
            for key in ('source','count_object','count_linker','count_binary','profile_binary','pg_linker','assembly'):
                c.valid_identity(row[key])
            folder=Path(row['source']['path']).parent
            for key,relative in [('count_object','counts.o'),('count_linker','count-linker/cc'),
                                 ('count_binary','counts'),('profile_binary','profile'),
                                 ('pg_linker','pg-linker/cc'),('assembly','generated.s')]:
                if row[key]['path']!=str(folder/relative):raise ValueError('probe artifact path')
            if len(row['gmon'])!=7:raise ValueError('missing gmon records')
            for i,gmon in enumerate(row['gmon']):
                c.valid_identity(gmon)
                prefix=str(folder/('gmon-'+str(i)+'.'))
                if not gmon['path'].startswith(prefix) or not gmon['path'][len(prefix):].isdigit():
                    raise ValueError('unmatched gmon execution')
            wraps=['pt_seq_get','pt_list_set','pt_seq_hold','pt_seq_item_held']
            if label=='candidate':wraps+=['pt_int_list_get','pt_int_list_set']
            count_flags=[row['count_object']['path']]+['-Wl,--wrap='+name for name in wraps]
            python=manifest['host']['python_executable']
            for key,flags in [('pg_linker',['-pg','-no-pie']),('count_linker',count_flags)]:
                expected=linker_source(python,data['cc'],flags)
                if row[key+'_source']!=expected:raise ValueError('incorrect probe linker command/flags')

            if row['counts']!=parse(row['raw']) or row['stdout']!=case['expected'] or row['returncode']!=0:raise ValueError('count parity')
            want_reads=10852716 if name=='sorting' else 4000000
            want_writes=2714780 if name=='sorting' else 4000000
            typed=label=='candidate' and name in ('sorting','int_access')
            expected=dict(reads=0 if typed else want_reads,writes=0 if typed else want_writes,
                typed_reads=want_reads if typed else 0,typed_writes=want_writes if typed else 0,holds=0,held_items=0)
            if row['counts']!=expected:raise ValueError('unexpected access mechanism')
            if row['source']['sha256']!=case['source_identity']['sha256']:raise ValueError('probe input differs')
            for key in ('count_build','profile_build'):
                if row[key]!=[manifest['compiler']['path'],'build',row['source']['path']]:raise ValueError('probe build')
            if row['count_command']!=[row['count_binary']['path']]:raise ValueError('count execution')
            if row['profile_command']!=[row['profile_binary']['path']] or row['gprof_command']!=['gprof','-b','-p',row['profile_binary']['path'],*[x['path'] for x in row['gmon']]]:raise ValueError('profile execution')
            expected_cc=[data['cc'],'-std=c11','-O2','-fno-builtin','-Wall','-Wextra','-Werror']
            if label=='candidate':expected_cc+=['-DTYPED=1']
            expected_cc+=['-c',data['probe']['path'],'-o',row['count_object']['path']]
            if row['count_cc']!=expected_cc:raise ValueError('count build settings')
            if len(row['profile_samples'])!=7 or len(row['gmon'])!=7 or not row['flat_profile']:raise ValueError('missing profiles')
            for sample in row['profile_samples']:
                if sample['stdout']!=case['expected'] or sample['stderr'] or sample['returncode']!=0:raise ValueError('profile parity')
                b.stats([sample['wall_seconds']])
            for key in ('pg_linker','count_linker'):
                if row[key]['sha256']!=b.digest(row[key+'_source'].encode()):raise ValueError('wrapper mismatch')
    return 8


if __name__=='__main__':
    action=sys.argv[1];work=Path(sys.argv[2]);output=Path(sys.argv[3])
    if action=='collect':collect(work,output)
    elif action=='verify':print('Verified probe cases:',verify(json.loads(output.read_text())))
    else:raise ValueError('collect/verify required')
