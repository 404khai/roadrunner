#!/usr/bin/env python3
"""Verify matched Phase 33 semantics and summarize descriptive stage measurements."""
import argparse
import json
from pathlib import Path
from collect_phase33 import summarize


def compare(baseline, candidate):
    old = {r['manifest']: r for r in baseline['cases']}
    new = {r['manifest']: r for r in candidate['cases']}
    if len(old) != len(baseline['cases']) or len(new) != len(candidate['cases']):
        raise ValueError('duplicate workload identity')
    if baseline['configuration'] != candidate['configuration']:
        raise ValueError('measurement configuration differs')
    if set(old) != set(new):
        raise ValueError('workload coverage differs')
    if baseline['hardware'] != candidate['hardware']:
        raise ValueError('hardware differs')
    if baseline['compiler'] != candidate['compiler']:
        raise ValueError('compiler differs')
    rows = []
    for name, before in old.items():
        after = new[name]
        for field in ['manifest_sha256', 'graph_digest', 'semantic_hash']:
            if before[field] != after[field]:
                raise ValueError(f'{name}: {field} differs')
        for record in [before, after]:
            for sample in record['samples']:
                if sample['probe']['result']['semantic_hash'] != record['semantic_hash']:
                    raise ValueError(f'{name}: sample semantic mismatch')
        stages = {}
        for field in before['samples'][0]['probe']['result']:
            if field.endswith('_ns'):
                stages[field] = {label: summarize([s['probe']['result'][field] for s in record['samples']]) for label, record in [('before', before), ('after', after)]}
        rows.append(dict(manifest=name, semantic_hash=before['semantic_hash'], graph_nodes=before['graph_nodes'], graph_edges=before['graph_edges'], stages=stages, peak_process_rss_bytes=dict(before=before['peak_process_rss_bytes'], after=after['peak_process_rss_bytes']), result_bytes=dict(before=before['samples'][0]['probe']['result'].get('result_bytes'), after=after['samples'][0]['probe']['result'].get('result_bytes')), retained=dict(before=before['samples'][0]['probe']['result'].get('diagnostics'), after=after['samples'][0]['probe']['result'].get('diagnostics'))))
    return dict(schema_version=1, baseline_binary_sha256=baseline['binary_sha256'], candidate_binary_sha256=candidate['binary_sha256'], baseline_source=baseline['source_revision'], candidate_source=candidate['source_revision'], hardware=baseline['hardware'], cases=rows, limits='Descriptive medians/quantiles of small process/stage samples; not request tails, SLOs or supported-scale claims. Full result SHA-256 matches, not reduced output. Whole-process RSS includes setup and serialization. Original queue, dispatch, context and evidence semantics retained.')


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('baseline', type=Path)
    p.add_argument('candidate', type=Path)
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    a.output.write_text(json.dumps(compare(json.loads(a.baseline.read_text()), json.loads(a.candidate.read_text())), indent=2)+'\n')


if __name__ == '__main__':
    main()
