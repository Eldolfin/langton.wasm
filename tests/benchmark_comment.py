#!/usr/bin/env python3
"""Post/update a PR comment comparing benchmark results (main vs PR) via Forgejo API."""

import json
import os
import sys
from pathlib import Path
from urllib import request
from urllib.error import HTTPError

FORGEJO_URL = os.environ.get("FORGEJO_URL", "https://codeberg.org")
TOKEN = os.environ["FORGEJO_TOKEN"]
REPO = os.environ["GITHUB_REPOSITORY"]
PR_NUMBER = os.environ["PR_NUMBER"]

API = f"{FORGEJO_URL}/api/v1"
HEADERS = {"Authorization": f"token {TOKEN}", "Content-Type": "application/json"}

MARKER = "<!-- perf-bench -->"

SCENARIO_LABELS = {
    "light": "Light (2 ants)",
    "medium": "Medium (50 ants)",
    "heavy": "Heavy (500 ants)",
}


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


def build_comment(main_results: dict, pr_results: dict, duration: float) -> str:
    lines = [
        MARKER,
        "## ⚡ Performance Benchmark",
        "",
        "| Scenario | main (steps/s) | PR (steps/s) | Δ |",
        "|----------|---------------|-------------|------|",
    ]

    for key in ("light", "medium", "heavy"):
        label = SCENARIO_LABELS.get(key, key)
        main_sps = main_results.get(key, {}).get("steps_per_sec", 0)
        pr_sps = pr_results.get(key, {}).get("steps_per_sec", 0)
        delta = delta_cell(main_sps, pr_sps)
        lines.append(f"| {label} | {fmt_num(main_sps)} | {fmt_num(pr_sps)} | {delta} |")

    lines.append("")
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

    # Infer duration from results (use first available scenario)
    duration = 30.0
    for key in ("light", "medium", "heavy"):
        if key in main_results and "duration_s" in main_results[key]:
            duration = main_results[key]["duration_s"]
            break

    body = build_comment(main_results, pr_results, duration)

    existing_id = find_existing_comment()
    if existing_id is not None:
        api("PATCH", f"/repos/{REPO}/issues/comments/{existing_id}", {"body": body})
        print(f"Updated existing comment {existing_id}.")
    else:
        api("POST", f"/repos/{REPO}/issues/{PR_NUMBER}/comments", {"body": body})
        print("Posted new comment.")


if __name__ == "__main__":
    main()
