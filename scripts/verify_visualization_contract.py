#!/usr/bin/env python3
"""Independent schema, identity, E7 shape and metric oracles. Requires jsonschema 4.25.1."""
import argparse
import copy
import hashlib
import importlib.metadata
import json
import math
import struct
import subprocess
import tempfile
from pathlib import Path
from jsonschema import Draft202012Validator


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output', default='docs/evidence/phase-26-independent-oracle.json')
    a = p.parse_args()
    schema = json.loads(Path('docs/schemas/route-evidence-v1.schema.json').read_text())
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema)
    with tempfile.TemporaryDirectory(prefix='phase26-oracle-') as tmp:
        sample = Path(tmp) / 'sample.json'
        subprocess.run(['target/release/examples/visualization_evidence', str(Path(tmp) / 'graph.json'), str(sample)], check=True, capture_output=True)
        route = json.loads(sample.read_bytes())
    validator.validate(route)
    geom = route['geometry']
    # Independent canonicalization oracle: explicit domain bytes, length prefixes, LE IDs.
    h = hashlib.sha256(b'roadrunner/spatial-path/v1\0' + geom['graph_digest'].encode())
    h.update(struct.pack('<Q', 101))
    h.update(b''.join(struct.pack('<I', n) for n in range(101)))
    h.update(struct.pack('<Q', 100))
    h.update(b''.join(struct.pack('<I', e) for e in range(100)))
    assert geom['path_id'] == 'path-v1:' + h.hexdigest()
    assert geom['nodes'] == list(range(101))
    evaluation_tuple = [1, geom['path_id'], route['provenance'], route['search']['algorithm'], route['objective'], route['predicted_elapsed_seconds']]
    evaluation_bytes = json.dumps(evaluation_tuple, separators=(',', ':'), ensure_ascii=False).encode()
    assert route['evaluation_id'] == 'evaluation-v1:' + hashlib.sha256(evaluation_bytes).hexdigest()
    distance = 0
    radius = 6_371_008.8
    def haversine(a, b):
        lat1, lon1 = map(math.radians, (a['latitude_e7'] / 1e7, a['longitude_e7'] / 1e7))
        lat2, lon2 = map(math.radians, (b['latitude_e7'] / 1e7, b['longitude_e7'] / 1e7))
        v = math.sin((lat2-lat1)/2)**2 + math.cos(lat1)*math.cos(lat2)*math.sin((lon2-lon1)/2)**2
        return 2 * radius * math.asin(math.sqrt(v))
    for i, occurrence in enumerate(geom['occurrences']):
        expected = [dict(latitude_e7=0, longitude_e7=i*4), dict(latitude_e7=1, longitude_e7=i*4+1), dict(latitude_e7=1, longitude_e7=i*4+2), dict(latitude_e7=0, longitude_e7=(i+1)*4)]
        assert geom['points_e7'][i*4:(i+1)*4] == expected
        assert occurrence['ordinal'] == i and occurrence['edge'] == i
        assert (occurrence['point_start'], occurrence['point_end']) == (i*4, (i+1)*4)
        segment = sum(haversine(x, y) for x, y in zip(expected, expected[1:]))
        assert math.isclose(segment, occurrence['distance_meters'], rel_tol=1e-10, abs_tol=1e-12)
        distance += segment
    assert math.isclose(distance, route['distance_meters'], rel_tol=1e-10)
    assert math.isclose(distance/10, route['predicted_elapsed_seconds'], rel_tol=1e-10)
    assert route['objective']['kind'] == 'travel_time'
    assert route['objective']['value'] == route['predicted_elapsed_seconds']
    assert route['search']['expanded_states'] == 101
    rejected = []
    for field, value in [('semantics.geometry', 'unknown'), ('semantics.authority', 'unknown'), ('semantics.schema_version', 2), ('objective.kind', 'unknown'), ('objective_unit', 'meters')]:
        bad = copy.deepcopy(route)
        obj = bad
        names = field.split('.')
        for name in names[:-1]:
            obj = obj[name]
        obj[names[-1]] = value
        assert not validator.is_valid(bad), field
        rejected.append(field)
    optional = copy.deepcopy(route)
    optional.pop('optional_capabilities')
    optional['unknown_optional_extension'] = dict(available=False)
    validator.validate(optional)
    result = dict(schema_version=1, status='VERIFIED', validator='jsonschema '+importlib.metadata.version('jsonschema'),
                  fixture='phase26-chain/v1;100 segments;4 vertices;36kph', exact_vertices=400, exact_occurrences=100,
                  path_id_independent_oracle='domain+digest+LE64 lengths+LE32 IDs SHA256',
                  evaluation_id_oracle='independent Python compact JSON tuple on verified fixture values;v1 field order',
                  metric_oracle='independent Python spherical Haversine;1e-10 relative tolerance;speed conversion 36kph=10m/s',
                  required_mutations_rejected=rejected, optional_omission_and_unknown_extension='accepted',
                  scope='portable export; no scene, physical source, operational replay or universal evaluation identity claim')
    Path(a.output).write_text(json.dumps(result, indent=2)+'\n')
    print(a.output)


if __name__ == '__main__':
    main()
