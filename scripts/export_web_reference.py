#!/usr/bin/env python3
"""Refresh or check public landing evidence and the server-owned OpenAPI contract."""
import argparse
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check', action='store_true')
args = parser.parse_args()
files = {
    'demos/catalog.json': 'web/tests/catalog.json',
    'demos/delivery.json': 'web/tests/delivery-distributed.json',
    'demos/dispatch.json': 'docs/evidence/phase-31/spatial.json',
    'demos/route.json': 'web/tests/forward.json',
    'demos/route-request.json': 'web/tests/forward-request.json',
}
outputs = {name: (root / source).read_bytes() for name, source in files.items()}
outputs['docs/openapi.json'] = subprocess.check_output(
    ['cargo', 'run', '-q', '-p', 'roadrunner-api', '--', '--openapi'], cwd=root
)
for name, content in outputs.items():
    target = root / 'web/public' / name
    if args.check:
        if not target.exists() or target.read_bytes() != content:
            raise SystemExit(f'Stale public reference: {target.relative_to(root)}')
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(content)
print(f'{"Checked" if args.check else "Exported"} {len(outputs)} public references')
