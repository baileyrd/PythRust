"""Cross-bind immutable #815 primary and supporting evidence; no execution."""
import json
from pathlib import Path
import re

import int_list_campaign as c
import int_list_path_control as path_control
import int_list_probes as probes

CODE_CASES=('sorting','int_access','float_access','strings')


def verify_code(code, canonical, folder):
    if set(code)!=set(c.LABELS):raise ValueError('generated-code conditions')
    for label,rows in code.items():
        if set(rows)!=set(CODE_CASES):raise ValueError('generated-code inventory')
        for name,row in rows.items():
            for key in ('binary','disassembly'):c.valid_identity(row[key])
            if row['binary']!=canonical[label]['cases'][name]['binary']:
                raise ValueError('generated code differs from measured binary')
            filename=name+'-'+label+'.s'
            original_root=Path(canonical[label]['harness']['path']).parents[1]
            if row['disassembly']['path']!=str(original_root/'docs/benchmarks/card815-code'/filename):
                raise ValueError('generated-code artifact name')
            raw=(folder/'card815-code'/filename).read_bytes()
            if c.b.digest(raw)!=row['disassembly']['sha256']:
                raise ValueError('generated-code bytes differ')
            symbols=re.findall(r'^[0-9a-f]+ <([^>]+)>:',raw.decode(),re.MULTILINE)
            if row['symbols']!=symbols:raise ValueError('generated-code symbol inventory')
            names=['pythrust_main','pt_seq_get','pt_list_set','pt_int_list_get','pt_int_list_set']
            names += [n for n in symbols if re.fullmatch(r'py\d+_\w+',n)]
            expected=[['objdump','-d','--disassemble='+n,row['binary']['path']] for n in names]
            if row['commands']!=expected:raise ValueError('generated-code command linkage')


def verify_validation(state, canonical, folder):
    if state['baseline']!=c.BASE or state['baseline_tree']!=c.frozen()['versions']['baseline']['tree']:
        raise ValueError('validation baseline')
    if state['measured_artifacts_match_final_build'] is not True or set(state['final_build_artifacts'])!={'compiler','runtime'}:
        raise ValueError('incomplete final-build evidence')
    for kind,item in state['final_build_artifacts'].items():
        c.valid_identity(item)
        original_root=Path(canonical['candidate']['harness']['path']).parents[1]
        expected_path=original_root/'target/release'/Path(canonical['candidate'][kind]['path']).name
        if item['path']!=str(expected_path) or item['sha256']!=canonical['candidate'][kind]['sha256']:
            raise ValueError('final build differs from measured candidate')
    if set(state['log_artifacts'])!={p.name for p in (folder/'card815-logs').glob('*.log')}:
        raise ValueError('validation log inventory')
    for name,item in state['log_artifacts'].items():
        if item['path']!='card815-logs/'+name:raise ValueError('validation log path')
        raw=(folder/item['path']).read_bytes()
        if c.b.digest(raw)!=item['sha256'] or len(raw)!=item['bytes']:
            raise ValueError('validation log bytes')
    for row in state['validation']:
        name=Path(row['published_log']).name
        if row['sha256']!=state['log_artifacts'][name]['sha256']:
            raise ValueError('unlinked validation result')


def verify(primary, repeat, probe, path, code, state, folder=None):
    folder=c.evidence_root() if folder is None else Path(folder)
    c.verify(primary);c.verify(repeat)
    canonical=primary['prepared']
    if repeat['prepared']!=canonical:raise ValueError('repeat preparation differs')
    probes.verify(probe,canonical)
    path_control.verify(path,canonical)
    verify_code(code,canonical,folder)
    verify_validation(state,canonical,folder)
    return 10


def verify_bundle(folder=None):
    folder=c.evidence_root() if folder is None else Path(folder)
    names=('pair24','pair42','probes','path-control','code','validation')
    records=[json.loads((folder/('card815-'+n+'.json')).read_text()) for n in names]
    return verify(*records,folder=folder)


if __name__=='__main__':
    print('Verified cross-bound #815 workloads:',verify_bundle())
