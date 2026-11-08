"""#807 identical-source timing and actual allocation/copy probe (Linux GNU ld).
Build both compiler/runtime pairs first; --verify-only runs bounded regressions.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import statistics
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent
WRAPS = ['pt_str_new', 'pt_str_concat', 'pt_str_concat3', 'malloc', 'calloc', 'realloc', 'memcpy', 'memmove']


def run(cmd, env=None):
    done = subprocess.run(cmd, env=env, capture_output=True)
    if done.returncode:
        raise RuntimeError(f'{cmd}: {done.stderr.decode()} {done.stdout.decode()}')
    return done


def sample(cmd, count, env=None):
    times, outputs = [], set()
    for _ in range(count):
        start = time.perf_counter()
        done = run(cmd, env)
        times.append(time.perf_counter() - start)
        outputs.add(done.stdout)
    assert len(outputs) == 1, (cmd, outputs)
    return dict(median=statistics.median(times), minimum=min(times), maximum=max(times), samples=times), outputs.pop()


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--before', type=Path)
    ap.add_argument('--after', type=Path, required=True)
    ap.add_argument('--runs', type=int, default=9)
    ap.add_argument('--verify-only', action='store_true')
    ap.add_argument('--output', type=Path)
    args = ap.parse_args()
    assert sys.platform == 'linux' and args.runs > 0
    cc = shutil.which('cc')
    assert cc
    compilers = {'after': args.after.resolve()}
    if args.before:
        compilers = {'before': args.before.resolve(), **compilers}
    result = dict(generated_at=datetime.now(timezone.utc).isoformat(),
        command=sys.argv, compilers={name: dict(path=str(path),
            compiler_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
            runtime_sha256=hashlib.sha256((path.parent / 'libpythrust_rt.a').read_bytes()).hexdigest())
            for name, path in compilers.items()},
        environment=dict(platform=platform.platform(), python=sys.version,
            rustc=run(['rustc', '-Vv']).stdout.decode(),
            cpu=Path('/proc/cpuinfo').read_text().split('model name',1)[-1].splitlines()[0],
            affinity=sorted(os.sched_getaffinity(0)), runs=args.runs,
            cc=run([cc,'--version']).stdout.decode().splitlines()[0]), cases={})
    head = 'def chain(a: str, b: str, c: str) -> str:\n'
    loop = 'total = 0\nfor i in range(100):\n    total += len(chain("é", "中", "🙂"))\nprint(total)\n'
    # Expected successful concat objects / fused calls / copied bytes, per version.
    cases = {
        'eligible': (head+'    return a + b + c\n'+loop, (200,0,1400), (100,100,900)),
        'fallback_call': ('def identity(s: str) -> str:\n    print("", end="")\n    return s\n'+head+'    return a + b + identity(c)\n'+loop, (200,0,1400), (200,0,1400)),
        'fallback_right': (head+'    return a + (b + c)\n'+loop, (200,0,1600), (200,0,1600)),
        'empty': (head+'    return a + b + c\n'+loop.replace('"é", "中", "🙂"','"", "", ""'), (200,0,0), (100,100,0)),
        'mixed_empty': (head+'    return a + b + c\n'+loop.replace('"é", "中", "🙂"','"", "中", ""'), (200,0,600), (100,100,300)),
        'startup': ('pass\n', (0,0,0), (0,0,0)),
    }
    if not args.verify_only:
        cases['chain'] = ((ROOT/'bench/string_chain.pyr').read_text(), (4000000,0,120000000), (2000000,2000000,70000000))
        cases['strings'] = ((ROOT/'bench/strings.pyr').read_text(), None, None)
    with tempfile.TemporaryDirectory(prefix='pythrust-807-') as tmp:
        tmp = Path(tmp)
        obj = tmp/'counts.o'
        run([cc,'-O2','-fno-builtin','-c',str(ROOT/'bench/string_allocations.c'),'-o',str(obj)])
        wrap = tmp/'linker'
        wrap.mkdir()
        wrapper = wrap/'cc'
        wrapper.write_text('#!/usr/bin/env python3\nimport os, sys\nos.execv('+repr(cc)+', ['+repr(cc)+', *sys.argv[1:], '+repr(str(obj))+', '+repr('-Wl,'+','.join('--wrap='+n for n in WRAPS))+'])\n')
        wrapper.chmod(0o700)
        for name,(source,expected_before,expected_after) in cases.items():
            expected = run([sys.executable,'-c',source]).stdout
            row = dict(source=source, sha256=hashlib.sha256(source.encode()).hexdigest(), stdout=expected.decode(), versions={})
            for version,compiler in compilers.items():
                work=tmp/(name+'-'+version)
                work.mkdir()
                src=work/'case.pyr'
                src.write_text(source)
                env=dict(os.environ,PYTHRUST_RT=str(compiler.parent/'libpythrust_rt.a'))
                build_times,_=sample([str(compiler),'build',str(src)],args.runs,env)
                binary=src.with_suffix('')
                binary_hash=hashlib.sha256(binary.read_bytes()).hexdigest()
                assert run([str(binary)]).stdout == expected
                execution,stdout=sample([str(binary)],args.runs)
                assert stdout == expected
                probe_env=dict(env,PATH=str(wrap)+os.pathsep+os.environ['PATH'])
                run([str(compiler),'build',str(src)],probe_env)
                probe=run([str(binary)])
                assert probe.stdout == expected
                counts=dict((k,int(v)) for k,v in (line.split('=') for line in probe.stderr.decode().splitlines()))
                assert counts['probe_overflow'] == 0, counts
                wanted=expected_before if version == 'before' else expected_after
                if wanted:
                    actual=tuple(counts[k] for k in ['concat_objects','fused_calls','concat_copied_bytes'])
                    assert actual == wanted,(name,version,actual,wanted)
                    # Every concat allocates one PtStr; nonempty buffer adds one.
                    assert counts['concat_allocations'] == wanted[0]*(1 if name == 'empty' else 2),(name,counts)
                    assert counts['concat_reallocations'] == 0,(name,counts)
                row['versions'][version]=dict(compile_link=build_times,process_execution=execution,binary_sha256=binary_hash,**counts)
            result['cases'][name]=row
        # Deterministically refuse the first concat buffer allocation.
        # A later operand has an observable construction marker in the probe.
        # The ordinary binary prefix must fail before that marker, even under
        # except Exception. Also check the eligible helper remains fatal.
        result['allocation_failure'] = {}
        for version,compiler in compilers.items():
            checks = {}
            for kind,source in {
                'fallback_prefix': 'def later() -> str:\n    return "LATER"\ndef f(a: str, b: str) -> str:\n    return a + b + later()\ntry:\n    print(f("a", "b"))\nexcept Exception:\n    print("caught")\n',
                'fused': head+'    return a + b + c\ntry:\n    print(chain("a", "b", "c"))\nexcept Exception:\n    print("caught")\n',
            }.items():
                src=tmp/(version+'-'+kind+'.pyr')
                src.write_text(source)
                env=dict(os.environ,PYTHRUST_RT=str(compiler.parent/'libpythrust_rt.a'),PATH=str(wrap)+os.pathsep+os.environ['PATH'])
                run([str(compiler),'build',str(src)],env)
                done=subprocess.run([str(src.with_suffix(''))],env=dict(os.environ,PYTHRUST_807_FAIL_ALLOC='1'),capture_output=True)
                assert done.returncode != 0,(kind,done)
                assert b'memory allocation' in done.stderr,(kind,done.stderr)
                assert b'LATER_OPERAND_EXECUTED' not in done.stderr,(kind,done.stderr)
                assert b'caught' not in done.stdout,(kind,done.stdout)
                checks[kind]=dict(returncode=done.returncode,stdout=done.stdout.decode(),stderr=done.stderr.decode())
            result['allocation_failure'][version]=checks
        if 'before' in compilers:
            for name,row in result['cases'].items():
                before,after=row['versions']['before'],row['versions']['after']
                removed=before['concat_allocations']-after['concat_allocations']
                assert before['heap_allocations']-after['heap_allocations'] == removed,(name,before,after)
            if 'strings' in result['cases']:
                vs=result['cases']['strings']['versions']
                for k in ['concat_objects','fused_calls','concat_allocations','concat_copied_bytes']:
                    assert vs['before'][k] == vs['after'][k],(k,vs)
    rendered=json.dumps(result,indent=2)+'\n'
    if args.output:
        args.output.write_text(rendered)
    print(rendered)


if __name__ == '__main__':
    main()
