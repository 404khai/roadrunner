#!/usr/bin/env python3
"""Create a fixed controlled null design with different graph geometry/topology.

This is engineering evidence within one simulator mechanism, not a random sample
of independent physical worlds. No noise, target-derived feature or seed change.
"""
import argparse
import copy
import json
from pathlib import Path

DESIGNS = [
    ('wide-line', 'Train', 10, [(0, 0), (50000, 0)], [(0, 1)]),
    ('short-chain', 'Train', 80, [(0, 0), (10000, 0), (20000, 2000)], [(0, 1), (1, 2)]),
    ('long-branch', 'Train', 25, [(0, 0), (60000, 0), (15000, 40000), (30000, -20000)], [(0, 1), (0, 2), (0, 3)]),
    ('middle-cycle', 'Validation', 30, [(0, 0), (12000, 0), (12000, 18000), (0, 18000)], [(0, 1), (1, 2), (2, 3), (3, 0)]),
    ('middle-diamond', 'Test', 35, [(0, 0), (20000, 0), (10000, 10000), (10000, -10000)], [(0, 1), (0, 2), (2, 1), (0, 3), (3, 1)]),
    ('middle-star', 'Test', 45, [(0, 0), (16000, 0), (0, 18000), (-22000, 0), (0, -9000)], [(0, 1), (0, 2), (0, 3), (0, 4)]),
]


def generate(destination):
    root = Path(__file__).resolve().parents[1]
    template = json.loads((root / 'data/fixtures/phase-19/observed-road-delay.json').read_text())
    destination.mkdir(parents=True, exist_ok=False)
    sources, assignments = [], {}
    for index, (name, partition, speed, points, roads) in enumerate(DESIGNS):
        document = copy.deepcopy(template)
        document['graph']['nodes'] = [
            {'id': i, 'coordinate': {'latitude_e7': index * 100000 + y, 'longitude_e7': x}}
            for i, (x, y) in enumerate(points)]
        document['graph']['roads'] = [
            {'id': i, 'from': a, 'to': b, 'speed_kph': speed, 'bidirectional': True}
            for i, (a, b) in enumerate(roads)]
        scenario = document['scenario']
        scenario.update(end_seconds=1000, scenario_id='controlled-null/' + name + '/v1', dynamic_events=[])
        scenario['dispatch']['forecast_validity_seconds'] = 2000
        scenario['riders'] = scenario['riders'][:1]
        def retain(run, doc):
            filename = run + '.json'
            (destination / filename).write_text(json.dumps(doc, indent=2) + '\n')
            sources.append({'scenario': filename, 'run': run, 'lineage': {
                'generator': 'controlled-distinct-topologies/no-delay/v1',
                'family': 'constructed-topology/' + name, 'base_world': 'constructed-geometry/' + name,
                'realization': 'deterministic-no-delay/' + name, 'policy': 'dynamic-multistart-local/v1',
                'dependencies': ['controlled-world/' + name]}})
            assignments[run] = partition
        retain(name, document)
        if index == 0:
            short = copy.deepcopy(document)
            short['scenario']['end_seconds'] = 1
            retain(name + '-censored', short)
            empty = copy.deepcopy(document)
            empty['scenario']['riders'][0]['available'] = False
            retain(name + '-no-starts', empty)
    request = {'dataset_version': 'controlled-route-null/v1', 'sources': sources,
               'splits': {'claim': 'unseen-synthetic-components/v1', 'assignments': assignments, 'excluded_reasons': {}}}
    (destination / 'request.json').write_text(json.dumps(request, indent=2) + '\n')
    (destination / 'design.json').write_text(json.dumps({
        'schema': 'controlled-route-null-design/v1',
        'claim': 'engineering component holdout within shared deterministic simulator; no sampling independence, physical or causal claim',
        'construction': 'six explicit different geometries and topologies; Train ranges enclose Validation/Test; horizon and no-start descendants remain with Train ancestor',
        'expected': '12 complete starts plus one censored start; no-delay source labels route-derived; no evidence for learnable physical residual',
        'assignments': assignments}, indent=2) + '\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('destination', type=Path)
    generate(parser.parse_args().destination)
