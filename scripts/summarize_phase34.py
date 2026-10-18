#!/usr/bin/env python3
"""Summarize retained Phase 34 run records without converting stops into capacity claims."""
import argparse
import json
from pathlib import Path
import statistics


def quantile(values, p):
    values = sorted(values)
    n = (len(values)-1)*p
    low = int(n)
    return values[low] + (values[min(low+1, len(values)-1)]-values[low])*(n-low)


def histogram_quantiles(buckets):
    count = sum(buckets)
    if not count:
        return None
    # Bucket i is [2^(i-1), 2^i) ns, except bucket 0 == zero.
    def upper(p):
        at = max(1, int(count*p + .999999999))
        total = 0
        for i, amount in enumerate(buckets):
            total += amount
            if total >= at:
                return 0 if i == 0 else (1 << i)-1
        raise ValueError('histogram count')
    return dict(samples=count, p50_upper_ns=upper(.5), p95_upper_ns=upper(.95), p99_upper_ns=upper(.99))


def summarize(records):
    groups = {}
    for record in records:
        identity = record['bundle']['scenario_sha256']
        key = (identity, record['configuration']['diagnostics'])
        groups.setdefault(key, []).append(record)
    cases = []
    for (identity, observed), runs in groups.items():
        # Hardware/binary/configuration mismatch cannot become a matched latency distribution.
        for field in ['hardware', 'binary_sha256', 'source_revision', 'configuration', 'compiler']:
            if any(r[field] != runs[0][field] for r in runs):
                raise ValueError(f'mismatched {field}')
        completed = [r for r in runs if r['outcome'] == 'completed']
        hashes = {r['final']['result_sha256'] for r in completed}
        if len(hashes) > 1:
            raise ValueError('nondeterministic full results')
        times = [r['final']['execution_ns'] for r in completed]
        histogram = {}
        for r in completed:
            for operation, buckets in r['final']['queue_action_log2_ns'].items():
                if len(buckets) != 65:
                    raise ValueError('invalid histogram shape')
                histogram[operation] = [a+b for a,b in zip(histogram.get(operation, [0]*65), buckets)]
        cases.append(dict(scenario_sha256=identity, diagnostics=observed, runs=len(runs), completed=len(completed),
                          outcomes=[r['outcome'] for r in runs],
                          execution_ns=None if not times else dict(raw=times, median=statistics.median(times), p95=quantile(times,.95), p99=quantile(times,.99)),
                          full_result_sha256=next(iter(hashes),None),
                          queue_action_latency_buckets={k:histogram_quantiles(v) for k,v in histogram.items()},
                          final_counts=[r['final']['summary'] for r in completed],
                          delivered_per_collector_wall_second=[r['final']['summary']['delivered_orders']/r['wall_seconds'] for r in completed],
                          delivered_per_execution_second=[r['final']['summary']['delivered_orders']/(r['final']['execution_ns']/1e9) for r in completed],
                          peak_sampled_process_group_rss_bytes=[max((s['process_group_rss_bytes'] for s in r['resource_samples']), default=0) or None for r in runs],
                          last_observed_boundary=[r['last_observed_boundary'] for r in runs]))
    return dict(schema_version=1,cases=cases,limits='No cold-cache, city capacity, operational service, durable throughput or rendering claims. Histograms give bucket upper bounds for queue-action processing, not HTTP requests. Run quantiles describe small repeated-run samples, not per-order tails. Collector wall includes polling and bookkeeping overhead; it is not exact process runtime. A null sampled RSS peak means no positive sample, not zero memory. Incomplete runs excluded from timing quantiles but retained in outcomes.')


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('records', nargs='+', type=Path)
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    a.output.write_text(json.dumps(summarize([json.loads(path.read_text()) for path in a.records]), indent=2)+'\n')


if __name__ == '__main__':
    main()
