"""#817 evidence-only residual string study. No production delta or speed claim.

One immutable preparation; separate link probes/PC sampling and ordinary release
self-controls. Frozen historical validation never consults future source files.
"""
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

import three_way as b
import int_list_campaign as old

BASE = '02c7816698f2d424ec0f2a487dce9f41c75bebe0'
TREE = '40b65a9e91eb33dddd6946680d8780033277de1c'
ROOT = Path(__file__).resolve().parents[1]
WORK = Path(os.environ.get('PYTHRUST_817_WORK','/workspace/card817')).resolve()
EVIDENCE = ROOT/'docs/benchmarks'
TOOLS = ('cargo','rustc','cc','ld','objdump','gprof','python3.13')
LABELS = ('baseline','replay')
WRAPS = ('malloc','calloc','realloc','free','memcpy','memmove','pt_str_new',
         'pt_str_from_int','pt_str_index','pt_str_concat')
BUILD = ['cargo','build','--locked','--release','-p','pythrust-compiler','-p','pythrust-rt']
FIELDS = ('calls','allocations','requested','object_bytes','buffer_bytes','buffer_copies',
          'other_copies','left_bytes','right_bytes','result_bytes','return_pc')
SCALARS = ('errors','outside_allocations','outside_requested','outside_copies','reallocations','scoped_reallocations')
# Filled only after collection. New campaigns require separate exports.
VALIDATION_SHA256 = '70dfa7050d33434ff517a2437d7a52da3d403ca88316d8e40a501991a4b5d5d7'
FROZEN_SHA256 = '4031fcbd2910f3458f8aea0a3bb6cd16be7a12c10a5cb1974a3105c1f8d66223'


def sources():
    result = old.sources()
    for name in ('tasks_alloc','tasks_alloc_serial'):
        result[name] = (ROOT/'bench'/f'{name}.pyr').read_text()
    return result


def load(path):
    return json.loads(Path(path).read_text())


def run(command, **kwargs):
    return b.run(command, **kwargs)


def ident(path):
    return b.identity(Path(path).resolve())


def tools():
    return {name:dict(identity=ident(shutil.which(name)),version=run([name,'-Vv' if name=='rustc' else '--version']).stdout.decode()) for name in TOOLS}


def preparation():
    if sys.version_info[:2] != (3,13): raise ValueError('CPython 3.13 required')
    for name in ('RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','PYTHRUST_RT_SRC','LD_PRELOAD','LD_LIBRARY_PATH'):
        if os.environ.get(name): raise ValueError('uncontrolled '+name)
    if run(['git','rev-parse','HEAD']).stdout.decode().strip()!=BASE: raise ValueError('wrong baseline')
    if run(['git','diff',BASE,'--','crates','Cargo.toml','Cargo.lock']).stdout: raise ValueError('production delta')
    inputs={}
    names=run(['git','ls-tree','-r',BASE,'--','crates','Cargo.toml','Cargo.lock']).stdout.decode().splitlines()
    for line in names:
        mode,kind,rest=line.split(' ',2);blob,path=rest.split('\t',1)
        raw=run(['git','cat-file','blob',blob]).stdout
        if raw!=(ROOT/path).read_bytes(): raise ValueError('changed input '+path)
        inputs[path]=dict(blob=blob,sha256=b.digest(raw),bytes=len(raw),mode=mode)
    out=WORK/'release';out.mkdir()
    for name in ('pythrust','libpythrust_rt.a'): shutil.copy2(ROOT/'target/release'/name,out/name)
    p=dict(base=BASE,tree=TREE,production_delta=[],inputs=inputs,tools=tools(),
           compiler=ident(out/'pythrust'),runtime=ident(out/'libpythrust_rt.a'),
           build_command=BUILD,build_log=ident(WORK/'release-build.log'),
           harness=ident(__file__),probe_source=ident(ROOT/'bench/string_copy_counts.c'),
           cpu=min(os.sched_getaffinity(0)),host=b.host(),cases={},
           environment={k:os.environ.get(k) for k in (*old.ENV_KEYS,'PYTHON','CARGO_HOME','RUSTUP_HOME')},
           utc=datetime.now(timezone.utc).isoformat())
    b.save(WORK/'preparation.json',p)
    env=dict(os.environ,PYTHRUST_RT=p['runtime']['path'])
    folder=WORK/'inputs';folder.mkdir()
    for name,source in sources().items():
        src=folder/(name+'.pyr');src.write_text(source)
        command=[p['compiler']['path'],'build',str(src)]
        run(command,env=env)
        expected=run([sys.executable,'-B',src]).stdout.decode()
        bins={}
        for label in LABELS:
            dest=WORK/label;dest.mkdir(exist_ok=True)
            shutil.copy2(src.with_suffix(''),dest/name);bins[label]=ident(dest/name)
            b.sample([str(dest/name)],expected)
        small=folder/(name+'-small.pyr')
        small.write_text(source.replace('4000000','400').replace('1000000','1000').replace('400000','400').replace('300000','100'))
        ref=run([sys.executable,'-B',small]);interp=run([p['compiler']['path'],'run',str(small)],env=env)
        if ref.stderr or interp.stderr or ref.stdout!=interp.stdout: raise ValueError('interpreter '+name)
        p['cases'][name]=dict(source=source,source_identity=ident(src),build_command=command,
            binaries=bins,expected=expected,interpreter=dict(source=small.read_text(),stdout=ref.stdout.decode(),result=interp.stdout.decode()))
        b.save(WORK/'preparation.json',p)
    p['refused']={}
    for name,source in dict(mixed='xs = [1, 2.0]\nprint(xs)\n',dyn='xs: list[dyn] = [1, 2.0]\nprint(xs)\n').items():
        src=folder/(name+'.pyr');src.write_text(source);cmd=[p['compiler']['path'],'check',str(src)]
        done=subprocess.run(cmd,capture_output=True)
        p['refused'][name]=dict(command=cmd,source=source,returncode=done.returncode,stdout=done.stdout.decode(),stderr=done.stderr.decode())
        if done.returncode!=1 or done.stdout or not done.stderr: raise ValueError('unsupported behavior changed')
    p['status']='complete';b.save(WORK/'preparation.json',p)


def wrapper_source(cc,flags):
    return '#!'+sys.executable+'\nimport os,sys\nos.execv('+repr(cc)+','+repr([cc])+'+sys.argv[1:]+'+repr(flags)+')\n'


def linker(folder,cc,flags):
    folder.mkdir();path=folder/'cc';path.write_text(wrapper_source(cc,flags));path.chmod(0o700)
    return dict(os.environ,PATH=str(folder)+os.pathsep+os.environ['PATH'])


def parse_counts(raw):
    values={}
    for line in raw.splitlines():
        key,value=line.split('=')
        if key in values: raise ValueError('duplicate counter')
        parts=value.split(',')
        if any(not re.fullmatch('[0-9]+',x) for x in parts): raise ValueError('invalid counter')
        if key.startswith('row'):
            if len(parts)!=len(FIELDS): raise ValueError('row width')
            values[key]=dict(zip(FIELDS,map(int,parts)))
        else:
            if len(parts)!=1: raise ValueError('scalar width')
            values[key]=int(value)
    if set(values)!={*(f'row{i}' for i in range(11)),*SCALARS}: raise ValueError('counter inventory')
    return values


def probe():
    p=load(WORK/'preparation.json');folder=WORK/'probe-scoped';folder.mkdir()
    src=folder/'strings.pyr';src.write_text(p['cases']['strings']['source'])
    cc=p['tools']['cc']['identity']['path'];obj=folder/'counts.o'
    cmd=[cc,'-std=c11','-O2','-fno-builtin','-Wall','-Wextra','-Werror','-c',p['probe_source']['path'],'-o',str(obj)]
    run(cmd)
    flags=[str(obj)]+['-Wl,--wrap='+n for n in WRAPS]+['-no-pie']
    env=linker(folder/'count-linker',cc,flags);env['PYTHRUST_RT']=p['runtime']['path']
    build=[p['compiler']['path'],'build',str(src)];run(build,env=env)
    binary=src.with_suffix('');count=folder/'counts';shutil.copy2(binary,count)
    done=run([count]);raw=done.stderr.decode();counts=parse_counts(raw)
    if done.stdout.decode()!=p['cases']['strings']['expected'] or counts['errors'] or counts['scoped_reallocations']: raise ValueError('count run invalid: '+raw)
    r=dict(collection_harness=ident(__file__),collection_probe_source=ident(ROOT/'bench/string_copy_counts.c'),compiler=p['compiler'],runtime=p['runtime'],source=ident(src),count_object=ident(obj),cc_command=cmd,
        count_build=build,count_command=[str(count)],count_binary=ident(count),raw=raw,counts=counts,
        stdout=done.stdout.decode(),returncode=done.returncode,count_linker=ident(folder/'count-linker/cc'),
        count_linker_source=(folder/'count-linker/cc').read_text())
    b.save(WORK/'probe.json',r)
    env=linker(folder/'pg-linker',cc,['-pg','-no-pie']);env['PYTHRUST_RT']=p['runtime']['path'];run(build,env=env)
    profile=folder/'profile';shutil.copy2(binary,profile);samples=[];gmons=[]
    b.pin(p['cpu'])
    for i in range(14):
        prefix=folder/f'gmon-{i}'
        env=dict(os.environ,GMON_OUT_PREFIX=str(prefix))
        sample=b.sample([str(profile)],p['cases']['strings']['expected'],env=env)
        paths=list(folder.glob(prefix.name+'.*'))
        if len(paths)!=1: raise ValueError('missing gmon')
        samples.append(dict(execution=i,command=[str(profile)],environment={'GMON_OUT_PREFIX':str(prefix)},**sample));gmons.append(ident(paths[0]))
    gprof=['gprof','-b','-p',str(profile),*[x['path'] for x in gmons]]
    r.update(profile_build=build,profile_binary=ident(profile),pg_linker=ident(folder/'pg-linker/cc'),
             pg_linker_source=(folder/'pg-linker/cc').read_text(),samples=samples,gmon=gmons,gprof_command=gprof,flat_profile=run(gprof).stdout.decode())
    for name,binpath in [('count',count),('profile',profile),('release',Path(p['cases']['strings']['binaries']['baseline']['path']))]:
        target=folder/(name+'-full.s');cmd=['objdump','-d',str(binpath)];target.write_bytes(run(cmd).stdout)
        selected=folder/(name+'-selected.s');commands=[['objdump','-d','--disassemble='+s,str(binpath)] for s in ('pythrust_main','pt_str_concat','pt_str_from_int','pt_str_new','pt_str_index')]
        selected.write_bytes(b''.join(run(c).stdout for c in commands))
        r.setdefault('code',{})[name]=dict(binary=ident(binpath),full=ident(target),full_command=cmd,selected=ident(selected),commands=commands)
    b.save(WORK/'probe.json',r)


def measure(rounds,seed):
    p=load(WORK/'preparation.json');b.pin(p['cpu'])
    record=dict(status='running',runs=rounds,seed=seed,host=b.host(),before=old.quiet(),cases={})
    for name,case in p['cases'].items():
        commands={label:[case['binaries'][label]['path']] for label in LABELS}
        warmups={label:[b.sample(cmd,case['expected']) for _ in range(2)] for label,cmd in commands.items()}
        order=b.order(LABELS,rounds,seed);samples={label:[] for label in LABELS};events=[]
        for i,labels in enumerate(order):
            for label in labels:
                result=b.sample(commands[label],case['expected']);samples[label].append(result)
                events.append(dict(round=i,label=label,command=commands[label],sample=result))
        record['cases'][name]=dict(commands=commands,warmups=warmups,order=order,events=events,samples=samples,
            stats={label:b.stats([x['wall_seconds'] for x in rows]) for label,rows in samples.items()})
        b.save(WORK/f'timing{rounds}.json',record)
    record.update(status='complete',after=old.quiet());b.save(WORK/f'timing{rounds}.json',record)


def valid_identity(item):
    old.valid_identity(item)


def validate(data):
    p=data['preparation'];probe=data['probe']
    if p['base']!=BASE or p['tree']!=TREE or p['production_delta'] or p['status']!='complete': raise ValueError('baseline admission')
    old.valid_tools(p['tools'],TOOLS)
    for item in p['inputs'].values():
        if not re.fullmatch('[a-f0-9]{40}',item['blob']) or not re.fullmatch('[a-f0-9]{64}',item['sha256']): raise ValueError('invalid source identities')
    for key in ('compiler','runtime','harness','probe_source','build_log'):valid_identity(p[key])
    if p['build_command']!=BUILD: raise ValueError('build settings')
    for name,c in p['cases'].items():
        valid_identity(c['source_identity'])
        if c['source_identity']['sha256']!=b.digest(c['source'].encode()):raise ValueError('workload digest')
        if c['build_command']!=[p['compiler']['path'],'build',c['source_identity']['path']]:raise ValueError('build command')
        if set(c['binaries'])!=set(LABELS):raise ValueError('conditions')
        for item in c['binaries'].values():valid_identity(item)
        if c['binaries']['baseline']['sha256']!=c['binaries']['replay']['sha256']:raise ValueError('self control changed')
        if c['interpreter']['stdout']!=c['interpreter']['result']:raise ValueError('interpreter parity')
    if probe['compiler']!=p['compiler'] or probe['runtime']!=p['runtime']:raise ValueError('cross bundle artifacts')
    for key in ('collection_harness','collection_probe_source','source','count_object','count_binary','profile_binary','count_linker','pg_linker'): valid_identity(probe[key])
    if probe['source']['sha256']!=p['cases']['strings']['source_identity']['sha256']:raise ValueError('probe input')
    folder=Path(probe['source']['path']).parent;cc=p['tools']['cc']['identity']['path']
    for name,flags in [('count_linker',[probe['count_object']['path']]+['-Wl,--wrap='+n for n in WRAPS]+['-no-pie']),('pg_linker',['-pg','-no-pie'])]:
        # Historical interpreter path is frozen in the preparation, not current sys.
        text='#!'+p['host']['python_executable']+'\nimport os,sys\nos.execv('+repr(cc)+','+repr([cc])+'+sys.argv[1:]+'+repr(flags)+')\n'
        if probe[name+'_source']!=text or probe[name]['sha256']!=b.digest(text.encode()):raise ValueError('wrapper settings')
    expected_cc=[cc,'-std=c11','-O2','-fno-builtin','-Wall','-Wextra','-Werror','-c',p['probe_source']['path'],'-o',str(folder/'counts.o')]
    if probe['cc_command']!=expected_cc:raise ValueError('count settings')
    for key in ('count_build','profile_build'):
        if probe[key]!=[p['compiler']['path'],'build',probe['source']['path']]:raise ValueError('probe build')
    if probe['count_command']!=[probe['count_binary']['path']]:raise ValueError('count command')
    if probe['counts']!=parse_counts(probe['raw']) or probe['counts']['errors'] or probe['counts']['scoped_reallocations']:raise ValueError('counter integrity')
    if len(probe['samples'])!=14 or len(probe['gmon'])!=14:raise ValueError('profile inventory')
    for i,(sample,gmon) in enumerate(zip(probe['samples'],probe['gmon'])):
        valid_identity(gmon)
        if sample['execution']!=i or sample['command']!=[probe['profile_binary']['path']] or sample['environment']!={'GMON_OUT_PREFIX':str(folder/f'gmon-{i}')}:raise ValueError('profile execution')
        if not re.fullmatch(re.escape(str(folder/f'gmon-{i}.'))+r'\d+',gmon['path']):raise ValueError('gmon linkage')
        check_sample(sample,p['cases']['strings']['expected'])
    if len({x['path'] for x in probe['gmon']})!=14:raise ValueError('duplicate profile')
    if probe['gprof_command']!=['gprof','-b','-p',probe['profile_binary']['path'],*[x['path'] for x in probe['gmon']]]:raise ValueError('profile report command')
    if not probe['flat_profile'] or 'no time accumulated' in probe['flat_profile']:raise ValueError('empty PC report')
    for name,code in probe['code'].items():
        for key in ('binary','full','selected'):valid_identity(code[key])
        expected=probe['count_binary'] if name=='count' else probe['profile_binary'] if name=='profile' else p['cases']['strings']['binaries']['baseline']
        if code['binary']!=expected or code['full_command']!=['objdump','-d',expected['path']]:raise ValueError('generated-code binary')
        if code['commands']!=[['objdump','-d','--disassemble='+s,expected['path']] for s in ('pythrust_main','pt_str_concat','pt_str_from_int','pt_str_new','pt_str_index')]:raise ValueError('generated-code commands')
    if [x['runs'] for x in data['timings']]!=[24,42]:raise ValueError('timing sets')
    for t in data['timings']:
        if t['status']!='complete' or set(t['cases'])!=set(p['cases']):raise ValueError('timing inventory')
        for name,c in t['cases'].items():
            case=p['cases'][name];commands={label:[case['binaries'][label]['path']] for label in LABELS}
            if c['commands']!=commands or c['order']!=b.order(LABELS,t['runs'],t['seed']):raise ValueError('timing commands/order')
            expected_events=[]
            for label in LABELS:
                if len(c['warmups'][label])!=2 or len(c['samples'][label])!=t['runs']:raise ValueError('missing samples')
                for s in c['warmups'][label]+c['samples'][label]:check_sample(s,case['expected'])
                if c['stats'][label]!=b.stats([s['wall_seconds'] for s in c['samples'][label]]):raise ValueError('statistics')
            for i,labels in enumerate(c['order']):
                for label in labels:expected_events.append(dict(round=i,label=label,command=commands[label],sample=c['samples'][label][i]))
            if c['events']!=expected_events:raise ValueError('execution order')
    return len(p['cases'])


def check_sample(s,expected):
    if s['stdout']!=expected or s['stderr'] or s['returncode']!=0:raise ValueError('sample parity')
    b.stats([s['wall_seconds']])
    if s['user_seconds']<0 or s['system_seconds']<0 or len(s['switches'])!=2 or any(x<0 for x in s['switches']):raise ValueError('resource record')


def load_bundle():
    return load(EVIDENCE/'card817-evidence.json')


def verify(data):
    export_raw=(EVIDENCE/'card817-manifest.json').read_bytes()
    if b.digest(export_raw)!=FROZEN_SHA256:raise ValueError('changed immutable #817 manifest')
    export=json.loads(export_raw)
    if export['preparation']!=data['preparation']:raise ValueError('unmatched canonical preparation/input/tool inventory')
    # Full collected-record digests bind counts, raw reports, timings and generated
    # code records to this campaign. They are integrity checks, not attestation.
    for key in ('probe','timings'):
        if export['records'][key]!=b.digest(json.dumps(data[key],sort_keys=True).encode()):raise ValueError('cross bundle '+key)
    for name,key in [('preparation_harness','harness'),('preparation_probe','probe_source')]:
        if b.digest(export['measured_sources'][name].encode())!=data['preparation'][key]['sha256']:raise ValueError('preparation tool source export')
    for name,key in [('collection_harness','collection_harness'),('collection_probe','collection_probe_source')]:
        if b.digest(export['measured_sources'][name].encode())!=data['probe'][key]['sha256']:raise ValueError('collection tool source export')
    return validate(data)


def verify_saved():
    data=load_bundle();result=verify(data)
    for name,row in data['probe']['code'].items():
        if b.digest((EVIDENCE/f'card817-code/{name}.s').read_bytes())!=row['selected']['sha256']:raise ValueError('disassembly bytes')
    return result




def verify_validation():
    """Published final-build/log linkage, distinct from immutable measurement."""
    raw=(EVIDENCE/'card817-validation.json').read_bytes()
    if b.digest(raw)!=VALIDATION_SHA256:raise ValueError('changed validation record')
    state=json.loads(raw);p=load_bundle()['preparation']
    if state['base']!=BASE or state['tree']!=TREE or state['production_delta']:raise ValueError('validation source')
    if len(state['stages'])!=9 or any(row['returncode']!=0 for row in state['stages']):raise ValueError('validation stage status')
    if set(state['final_build'])!={'compiler','runtime'}:raise ValueError('final artifact inventory')
    for key,item in state['final_build'].items():
        valid_identity(item)
        if item['sha256']!=p[key]['sha256']:raise ValueError('measured/final artifact differs')
    if state['final_inputs']!={name:row['sha256'] for name,row in p['inputs'].items()}:raise ValueError('final production inputs differ')
    if state['logs']['initial-release-build.log']['sha256']!=p['build_log']['sha256']:raise ValueError('initial build log linkage')
    if set(state['logs'])!={x.name for x in (EVIDENCE/'card817-logs').glob('*.log')}:raise ValueError('published log inventory')
    for name,item in state['logs'].items():
        raw=(EVIDENCE/item['path']).read_bytes()
        if item['path']!='card817-logs/'+name or b.digest(raw)!=item['sha256'] or len(raw)!=item['bytes']:raise ValueError('log bytes')
    for row in state['stages']:
        if row['log']['sha256']!=state['logs'][Path(row['published_log']).name]['sha256']:raise ValueError('stage/log linkage')
    return 9


if __name__=='__main__':
    action=sys.argv[1]
    if action=='prepare':preparation()
    elif action=='probe':probe()
    elif action=='measure':measure(int(sys.argv[2]),int(sys.argv[3]))
    elif action=='verify':print('Verified workloads/stages:',verify_saved(),verify_validation())
    else:raise ValueError('prepare/probe/measure/verify')
