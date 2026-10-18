#!/usr/bin/env python3
"""Retain complete benchmark evidence, losslessly compressing identical canonical files once."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import shutil
from summarize_phase34 import summarize, quantile


def retain(source, objects):
    checksum = hashlib.sha256()
    with source.open('rb') as reader:
        for data in iter(lambda: reader.read(1024*1024), b''):
            checksum.update(data)
    digest = checksum.hexdigest()
    target = objects/(digest+'.json.gz')
    if not target.exists():
        with source.open('rb') as reader, target.open('wb') as output:
            with gzip.GzipFile(filename='', fileobj=output, mode='wb', mtime=0) as compressed:
                shutil.copyfileobj(reader, compressed)
    return dict(path='objects/'+target.name, sha256=digest, uncompressed_bytes=source.stat().st_size)


def os_resources(path):
    if not path.exists():
        return None
    text = path.read_text()
    rss = re.search(r'(\d+)\s+maximum resident set size', text)
    cpu = re.search(r'([0-9.]+) real\s+([0-9.]+) user\s+([0-9.]+) sys', text)
    linux_rss = re.search(r'Maximum resident set size \(kbytes\):\s*(\d+)', text)
    return dict(peak_process_rss_bytes=int(rss[1]) if rss else int(linux_rss[1])*1024 if linux_rss else None,
                coarse_os_wall_seconds=float(cpu[1]) if cpu else None,
                coarse_user_seconds=float(cpu[2]) if cpu else None,
                coarse_system_seconds=float(cpu[3]) if cpu else None)


def archive(root, destination):
    destination.mkdir()
    objects = destination/'objects'
    objects.mkdir()
    records = []
    index = []
    for path in sorted(root.glob('*/run.json')):
        record = json.loads(path.read_text())
        target = destination/path.parent.name
        target.mkdir()
        for name in ['run.json', 'resources.txt', 'stderr.log', 'stdout.log']:
            if (path.parent/name).exists():
                shutil.copyfile(path.parent/name, target/name)
        for name in ['progress.jsonl', 'report.json']:
            if (path.parent/'engine'/name).exists():
                shutil.copyfile(path.parent/'engine'/name, target/name)
        item = dict(run=path.parent.name, outcome=record['outcome'], os_resources=os_resources(path.parent/'resources.txt'))
        for name in ['result.json', 'routes.json']:
            source = path.parent/'engine'/name
            if source.exists():
                item[name] = retain(source, objects)
                item[name]['complete'] = record['outcome'] == 'completed'
                if item[name]['complete'] and item[name]['sha256'] != record['final']['result_sha256']:
                    raise ValueError(f'{path}: canonical result checksum mismatch')
        index.append(item)
        records.append(record)
    # All manifest-qualified bundles, including tiers not executed, remain reproducible.
    for bundle in sorted(root.glob('*-bundle')):
        target = destination/bundle.name
        target.mkdir()
        for name in ['manifest.json', 'bundle.json']:
            shutil.copyfile(bundle/name, target/name)
        saved = retain(bundle/'scenario.json', objects)
        meta = json.loads((bundle/'bundle.json').read_text())
        if saved['sha256'] != meta['scenario_sha256']:
            raise ValueError('scenario checksum mismatch')
        artifact = bundle/'graph.rrgraph'
        digest = hashlib.sha256(artifact.read_bytes()).hexdigest()
        if digest != meta['artifact_sha256']:
            raise ValueError('graph artifact checksum mismatch')
        graph_path = objects/(digest+'.rrgraph')
        if not graph_path.exists():
            shutil.copyfile(artifact, graph_path)
        (target/'retained.json').write_text(json.dumps(dict(scenario=saved, graph='objects/'+graph_path.name), indent=2)+'\n')
    shutil.copyfile(root/'qualification.json', destination/'qualification.json')
    # Explicit names, not successful outcomes, identify excluded warmups.
    measured = []
    routing = []
    for path, record in zip(sorted(root.glob('*/run.json')), records):
        if record.get('plane') == 'routing':
            if record['outcome'] != 'completed':
                routing.append(dict(run=path.parent.name, outcome=record['outcome']))
                continue
            final = record['final']
            values = final['query_ns']
            routing.append(dict(run=path.parent.name, outcome=record['outcome'], algorithm=final['algorithm'],
                                concurrency=final['concurrency'], queries=final['queries'], successful=final['successful'], errors=final['errors'],
                                route_p50_ns=quantile(values,.5), route_p95_ns=quantile(values,.95), route_p99_ns=quantile(values,.99),
                                route_requests_per_execution_second=final['queries']/(final['execution_ns']/1e9), result_sha256=final['result_sha256']))
        elif not path.parent.name.endswith('-run-0') or record['outcome'] != 'completed':
            measured.append(record)
    output = dict(schema_version=1, archive_tool_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), summary_tool_sha256=hashlib.sha256(Path(__file__).with_name('summarize_phase34.py').read_bytes()).hexdigest(), runs=index, simulation_summary=summarize(measured), routing=routing,
                  limits='Complete JSON files are pooled by exact SHA-256 and losslessly gzip compressed. This is benchmark packaging, not a new historical archive or reduced evidence format. Successful headless warmups remain in raw records but not measured distributions. Failed warmups remain visible. Collector wall includes monitoring/bookkeeping; OS real/CPU time is coarse. Null OS/sampled measurements are unavailable, not zero allocations. No capacity extrapolation.')
    (destination/'index.json').write_text(json.dumps(output, indent=2)+'\n')


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('source', type=Path)
    p.add_argument('destination', type=Path)
    a = p.parse_args()
    archive(a.source, a.destination)


if __name__ == '__main__':
    main()
