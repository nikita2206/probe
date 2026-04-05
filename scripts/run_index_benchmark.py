#!/usr/bin/env python3

import argparse
import json
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path


DEFAULT_REPO_URL = "https://github.com/spring-projects/spring-framework.git"
DEFAULT_REPO_REF = "82b179f2383d7df6a1d1e12cc3ba6b179bc90f84"


def run_command(args, cwd=None, capture_output=False):
    result = subprocess.run(
        args,
        cwd=cwd,
        text=True,
        capture_output=capture_output,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"command failed: {' '.join(args)}\n"
            f"stdout:\n{result.stdout or ''}\n"
            f"stderr:\n{result.stderr or ''}"
        )
    return result


def checkout_repo(repo_url: str, repo_ref: str, repo_dir: Path) -> str:
    run_command(["git", "init", str(repo_dir)])
    run_command(["git", "remote", "add", "origin", repo_url], cwd=repo_dir)
    run_command(["git", "fetch", "--depth", "1", "origin", repo_ref], cwd=repo_dir)
    run_command(["git", "checkout", "--detach", "FETCH_HEAD"], cwd=repo_dir)
    return (
        run_command(["git", "rev-parse", "HEAD"], cwd=repo_dir, capture_output=True)
        .stdout.strip()
    )


def parse_indexed_file_count(stats_output: str) -> int:
    match = re.search(r"Files in index:\s+(\d+)", stats_output)
    if not match:
        raise RuntimeError(f"failed to parse indexed file count from:\n{stats_output}")
    return int(match.group(1))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--probe-binary", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--repo-url", default=DEFAULT_REPO_URL)
    parser.add_argument("--repo-ref", default=DEFAULT_REPO_REF)
    args = parser.parse_args()

    probe_binary = Path(args.probe_binary)
    if not probe_binary.exists():
        raise FileNotFoundError(f"probe binary not found: {probe_binary}")

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory(prefix="probe-benchmark-") as temp_dir:
        repo_dir = Path(temp_dir) / "benchmark-repo"

        checkout_started = time.perf_counter()
        checked_out_commit = checkout_repo(args.repo_url, args.repo_ref, repo_dir)
        checkout_duration_ms = round((time.perf_counter() - checkout_started) * 1000)

        reindex_started = time.perf_counter()
        run_command([str(probe_binary), "-d", str(repo_dir), "rebuild"])
        reindex_duration_ms = round((time.perf_counter() - reindex_started) * 1000)

        stats_result = run_command(
            [str(probe_binary), "-d", str(repo_dir), "stats"],
            capture_output=True,
        )
        indexed_files = parse_indexed_file_count(stats_result.stdout)

        result = {
            "repo_url": args.repo_url,
            "repo_ref": args.repo_ref,
            "checked_out_commit": checked_out_commit,
            "checkout_duration_ms": checkout_duration_ms,
            "reindex_duration_ms": reindex_duration_ms,
            "indexed_files": indexed_files,
        }

        print(f"Benchmark repository: {args.repo_url}")
        print(f"Benchmark ref: {args.repo_ref}")
        print(f"Checked out commit: {checked_out_commit}")
        print(f"Checkout duration (ms): {checkout_duration_ms}")
        print(f"Full reindex duration (ms): {reindex_duration_ms}")
        print(f"Indexed files: {indexed_files}")

        output_path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")

    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(str(exc), file=sys.stderr)
        raise
