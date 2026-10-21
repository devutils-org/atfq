#!/usr/bin/env python3
"""One line per target: does each atfq build start, which checks failed, and
what its probes reported. Exit 1 when the static build fails anywhere.

    python3 scripts/distroproof/summary.py dist/matrix.json
"""
import json
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def describe(build):
    if build is None:
        return 'missing'
    if not build['starts']:
        return 'does not start: ' + build['error'].split('\n')[0][:70]
    statuses = Counter(build['probes'].values())
    probes = ', '.join(f'{n} {s}' for s, n in sorted(statuses.items()))
    failed = ' FAILED ' + ' '.join(build['failed']) if build['failed'] else ''
    return f'runs; probes: {probes}{failed}'


def main(report_path):
    report = json.loads(Path(report_path).read_text())
    bad = []
    for target in report['targets']:
        name = target['name']
        cell_path = ROOT / '.distroproof' / 'collect' / name / 'cell.json'
        try:
            cell = json.loads(cell_path.read_text()) if cell_path.is_file() else {'builds': {}}
        except ValueError:
            cell = {'builds': {}}
            print(f'{name}: cell.json is not valid JSON (the sweep stopped while writing it)')
        builds = cell['builds']
        print(f"{name}  (VM: {target['outcome']})")
        for build in ('release', 'static'):
            print(f'  {build:8} {describe(builds.get(build))}')
        static = builds.get('static')
        if not static or not static['starts'] or static['failed']:
            bad.append(name)
    print()
    print(f"static build: {len(report['targets']) - len(bad)}/{len(report['targets'])} targets pass")
    if bad:
        print('failing:', ' '.join(bad))
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else 'dist/matrix.json'))
