#!/usr/bin/env python3

import json
import os
from pathlib import Path


def load_json(path_env: str):
    path = os.getenv(path_env)
    if not path:
        return None
    path_obj = Path(path)
    if not path_obj.exists():
        return None
    with path_obj.open() as handle:
        return json.load(handle)


def ms_to_seconds(value_ms: int) -> float:
    return value_ms / 1000.0


current = load_json("CURRENT_BENCHMARK_JSON")
baseline = load_json("BASELINE_BENCHMARK_JSON")

if current is None:
    raise SystemExit("CURRENT_BENCHMARK_JSON is required")

current_seconds = ms_to_seconds(current["reindex_duration_ms"])
lines = [
    "## Index Benchmark",
    f"- Current branch full reindex: `{current_seconds:.2f}s`",
    f"- Indexed files: `{current['indexed_files']}`",
    f"- Benchmark repo: `{current['repo_url']} @ {current['checked_out_commit']}`",
]

if baseline is not None:
    baseline_seconds = ms_to_seconds(baseline["reindex_duration_ms"])
    delta_seconds = current_seconds - baseline_seconds
    delta_pct = (delta_seconds / baseline_seconds * 100.0) if baseline_seconds else 0.0
    direction = "slower" if delta_seconds > 0 else "faster"
    lines.append(f"- Base branch full reindex: `{baseline_seconds:.2f}s`")
    lines.append(
        f"- Delta vs base: `{delta_seconds:+.2f}s` (`{delta_pct:+.2f}%`, {direction})"
    )
else:
    lines.append("- Base branch full reindex: `unavailable`")
    lines.append("- Delta vs base: `unavailable`")

run_url = os.getenv("GITHUB_SERVER_URL", "https://github.com")
repository = os.getenv("GITHUB_REPOSITORY")
run_id = os.getenv("GITHUB_RUN_ID")
if repository and run_id:
    lines.append(f"- Workflow run: {run_url}/{repository}/actions/runs/{run_id}")

report = "\n".join(lines)

output_path = os.getenv("GITHUB_OUTPUT")
if output_path:
    with open(output_path, "a", encoding="utf-8") as handle:
        handle.write("report<<EOF\n")
        handle.write(report)
        handle.write("\nEOF\n")
else:
    print(report)
