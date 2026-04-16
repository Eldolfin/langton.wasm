#!/usr/bin/env python3
"""Post/update a PR comment comparing benchmark results (main vs PR) via Forgejo API."""

import json
import os
import sys
from math import sqrt
from pathlib import Path
from statistics import mean, median
from urllib import request
from urllib.error import HTTPError

from benchmark_scenarios import SCENARIOS

FORGEJO_URL = os.environ.get("FORGEJO_URL", "https://codeberg.org")
TOKEN = os.environ["FORGEJO_TOKEN"]
REPO = os.environ["GITHUB_REPOSITORY"]
PR_NUMBER = os.environ["PR_NUMBER"]

API = f"{FORGEJO_URL}/api/v1"
HEADERS = {"Authorization": f"token {TOKEN}", "Content-Type": "application/json"}

MARKER = "<!-- perf-bench -->"


def api(method: str, path: str, body: dict | None = None) -> dict:
    data = json.dumps(body).encode() if body else None
    req = request.Request(f"{API}{path}", data=data, headers=HEADERS, method=method)
    try:
        with request.urlopen(req) as r:
            raw = r.read()
            return json.loads(raw) if raw else {}
    except HTTPError as e:
        print(f"API error {e.code}: {e.read().decode()}", file=sys.stderr)
        raise


def fmt_num(n: float) -> str:
    """Format number with comma separators, no decimals."""
    return f"{n:,.0f}"


def delta_cell(main_val: float, pr_val: float) -> str:
    """Build the delta cell with emoji indicator."""
    if main_val == 0:
        return "N/A"
    pct = ((pr_val - main_val) / main_val) * 100
    sign = "+" if pct >= 0 else ""
    text = f"{sign}{pct:.1f}%"
    if pct > 5:
        return f"\U0001f7e2 {text}"  # 🟢
    elif pct < -5:
        return f"\U0001f534 {text}"  # 🔴
    return text


def cv_pct(vals: list[float]) -> float | None:
    """Coefficient of variation (std / mean * 100). None when insufficient data."""
    if len(vals) < 2:
        return None
    avg = mean(vals)
    if avg == 0:
        return None
    variance = sum((v - avg) ** 2 for v in vals) / (len(vals) - 1)
    return sqrt(variance) / avg * 100


def cv_cell(vals: list[float]) -> str:
    cv = cv_pct(vals)
    if cv is None:
        return "—"
    return f"{cv:.1f}%"


def build_comment(
    main_results: dict,
    pr_results: dict,
    duration: float,
    metadata: dict | None = None,
) -> str:
    lines = [
        MARKER,
        "## ⚡ Performance Benchmark",
        "",
        "| Scenario | main median | main mean | CV | PR median | PR mean | CV | Δ (median) |",
        "|----------|------------|----------|-----|-----------|---------|-----|------------|",
    ]

    for key, scenario in SCENARIOS.items():
        label = scenario["label"]
        main_data = main_results.get(key, {})
        pr_data = pr_results.get(key, {})
        main_runs = main_data.get("individual_runs", [])
        pr_runs = pr_data.get("individual_runs", [])
        main_med = fmt_num(median(main_runs)) if main_runs else "0"
        main_avg = fmt_num(mean(main_runs)) if main_runs else "0"
        pr_med = fmt_num(median(pr_runs)) if pr_runs else "0"
        pr_avg = fmt_num(mean(pr_runs)) if pr_runs else "0"
        main_cv = cv_cell(main_runs)
        pr_cv = cv_cell(pr_runs)
        delta = delta_cell(
            median(main_runs) if main_runs else 0,
            median(pr_runs) if pr_runs else 0,
        )
        lines.append(
            f"| {label} | {main_med} | {main_avg} | {main_cv} | {pr_med} | {pr_avg} | {pr_cv} | {delta} |"
        )

    lines.append("")
    if metadata:
        iters = metadata["iterations"]
        per = metadata["duration_per_iteration_s"]
        lines.append(
            f"> {iters} \u00d7 {per:.0f}s runs (interleaved) on codeberg-medium runner. All values in steps/s. CV = coefficient of variation."
        )
    else:
        lines.append(f"> Measured over {duration:.0f}s on codeberg-medium runner.")

    return "\n".join(lines)


def find_existing_comment() -> int | None:
    """Find existing benchmark comment by marker, return comment ID or None."""
    comments = api("GET", f"/repos/{REPO}/issues/{PR_NUMBER}/comments")
    for c in comments:
        if MARKER in c.get("body", ""):
            return c["id"]
    return None


def main() -> None:
    if len(sys.argv) != 3:
        print(
            f"Usage: {sys.argv[0]} <main-results.json> <pr-results.json>",
            file=sys.stderr,
        )
        sys.exit(1)

    main_path = Path(sys.argv[1])
    pr_path = Path(sys.argv[2])

    main_results = json.loads(main_path.read_text())
    pr_results = json.loads(pr_path.read_text())

    # Extract metadata if present (new interleaved format)
    metadata = main_results.get("metadata") or pr_results.get("metadata")

    # Infer duration from metadata or per-scenario field
    duration = 30.0
    if metadata and "duration_per_iteration_s" in metadata:
        duration = metadata["duration_per_iteration_s"]
    else:
        for key in SCENARIOS:
            if key in main_results and "duration_s" in main_results[key]:
                duration = main_results[key]["duration_s"]
                break

    body = build_comment(main_results, pr_results, duration, metadata)

    existing_id = find_existing_comment()
    if existing_id is not None:
        api("PATCH", f"/repos/{REPO}/issues/comments/{existing_id}", {"body": body})
        print(f"Updated existing comment {existing_id}.")
    else:
        api("POST", f"/repos/{REPO}/issues/{PR_NUMBER}/comments", {"body": body})
        print("Posted new comment.")


if __name__ == "__main__":
    main()
