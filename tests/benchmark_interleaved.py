#!/usr/bin/env python3
"""Interleaved Playwright benchmark for Langton's Ant WASM app.

Instead of running all scenarios for main then all for PR, this runner
interleaves at the *scenario* level within each iteration:

    iteration 1: main-light, PR-light, main-medium, PR-medium, …
    iteration 2: main-light, PR-light, main-medium, PR-medium, …

Both builds are served simultaneously under separate URL paths
(/ref and /pr) to avoid filesystem swaps that can cause browser caching
issues.
"""

import argparse
import json
import shutil
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from statistics import median

from playwright.sync_api import sync_playwright, Page

from benchmark_scenarios import SCENARIOS

REPO_ROOT = Path.cwd()
INDEX_HTML = REPO_ROOT / "crates" / "langton" / "index.html"
PORT = 8765
BASE_URL = f"http://localhost:{PORT}"


def _port_open(port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.settimeout(0.1)
        return s.connect_ex(("localhost", port)) == 0


def setup_serve_dir(main_pkg: Path, pr_pkg: Path) -> Path:
    """Create a temp serve directory with /ref and /pr subdirs.

    Structure:
        serve_root/
          ref/
            index.html
            pkg/ → (copy of main build)
          pr/
            index.html
            pkg/ → (copy of PR build)
    """
    serve_root = Path(tempfile.mkdtemp(prefix="bench-serve-"))

    for name, pkg_src in [("ref", main_pkg), ("pr", pr_pkg)]:
        subdir = serve_root / name
        subdir.mkdir()
        shutil.copy2(INDEX_HTML, subdir / "index.html")
        shutil.copytree(pkg_src, subdir / "pkg")

    return serve_root


def start_http_server(serve_dir: Path) -> subprocess.Popen | None:
    """Start HTTP server on PORT serving serve_dir."""
    if _port_open(PORT):
        print(f"Port {PORT} already in use, reusing existing server", file=sys.stderr)
        return None

    proc = subprocess.Popen(
        ["python3", "-m", "http.server", str(PORT), "--directory", str(serve_dir)],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    for _ in range(40):
        if _port_open(PORT):
            return proc
        time.sleep(0.1)
    proc.kill()
    raise RuntimeError("HTTP server did not start in time")


def parse_steps(text: str) -> int:
    """Parse step counter text like 'Steps: 12,345' → 12345."""
    return int(text.replace("Steps:", "").replace(",", "").strip())


def measure_scenario(
    page: Page, variant: str, params: dict, duration_s: float
) -> float:
    """Run one scenario, return steps_per_sec.

    *variant* is "ref" or "pr" — determines the URL path prefix.
    """
    extra = "&".join(f"{k}={v}" for k, v in params.items() if k != "label")
    url = f"{BASE_URL}/{variant}/?debug&speedup_frames=0&{extra}"

    page.goto(url)
    page.wait_for_selector("canvas", timeout=15_000)
    page.wait_for_selector(".DebugUI-step-counter", timeout=15_000)

    # Let simulation stabilize briefly
    page.wait_for_timeout(500)

    # Read initial step count
    el = page.query_selector(".DebugUI-step-counter")
    assert el is not None
    initial_steps = parse_steps(el.inner_text())
    t_start = time.monotonic()

    # Wait for benchmark duration
    page.wait_for_timeout(int(duration_s * 1000))

    # Read final step count
    el = page.query_selector(".DebugUI-step-counter")
    assert el is not None
    final_steps = parse_steps(el.inner_text())
    t_end = time.monotonic()

    elapsed = t_end - t_start
    steps = final_steps - initial_steps
    steps_per_sec = steps / elapsed if elapsed > 0 else 0.0

    return steps_per_sec


def build_output(
    results: dict[str, list[float]], iterations: int, duration_s: float
) -> dict:
    """Build JSON-serializable output dict."""
    out: dict = {
        "metadata": {"iterations": iterations, "duration_per_iteration_s": duration_s}
    }
    for scenario in SCENARIOS:
        vals = results[scenario]
        out[scenario] = {
            "steps_per_sec": round(median(vals), 1) if vals else 0.0,
            "iterations": len(vals),
            "individual_runs": [round(v, 1) for v in vals],
        }
    return out


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Interleaved benchmark: alternates main/PR each scenario"
    )
    parser.add_argument(
        "--main-pkg", type=Path, required=True, help="Pre-built main pkg/ dir"
    )
    parser.add_argument(
        "--pr-pkg", type=Path, required=True, help="Pre-built PR pkg/ dir"
    )
    parser.add_argument(
        "--duration",
        type=float,
        default=10,
        help="Duration per scenario in seconds (default: 10)",
    )
    parser.add_argument(
        "--iterations", type=int, default=10, help="Number of iterations (default: 10)"
    )
    parser.add_argument(
        "--main-output", type=str, required=True, help="Output JSON for main"
    )
    parser.add_argument(
        "--pr-output", type=str, required=True, help="Output JSON for PR"
    )
    args = parser.parse_args()

    main_pkg = args.main_pkg.resolve()
    pr_pkg = args.pr_pkg.resolve()

    if not main_pkg.is_dir():
        print(f"ERROR: --main-pkg not a directory: {main_pkg}", file=sys.stderr)
        sys.exit(1)
    if not pr_pkg.is_dir():
        print(f"ERROR: --pr-pkg not a directory: {pr_pkg}", file=sys.stderr)
        sys.exit(1)

    main_results: dict[str, list[float]] = {s: [] for s in SCENARIOS}
    pr_results: dict[str, list[float]] = {s: [] for s in SCENARIOS}

    serve_dir = setup_serve_dir(main_pkg, pr_pkg)
    server_proc = start_http_server(serve_dir)
    try:
        with sync_playwright() as p:
            browser = p.chromium.launch(
                headless=True,
                args=[
                    "--disable-background-timer-throttling",
                    "--disable-renderer-backgrounding",
                    "--disable-backgrounding-occluded-windows",
                ],
            )
            page = browser.new_page()
            try:
                for i in range(1, args.iterations + 1):
                    print(f"Iteration {i}/{args.iterations}", file=sys.stderr)

                    for scenario_name, params in SCENARIOS.items():
                        # --- main (ref) ---
                        sps = measure_scenario(page, "ref", params, args.duration)
                        main_results[scenario_name].append(sps)
                        print(
                            f"  [main] {scenario_name}: {sps:,.0f} steps/s",
                            file=sys.stderr,
                        )

                        # --- PR ---
                        sps = measure_scenario(page, "pr", params, args.duration)
                        pr_results[scenario_name].append(sps)
                        print(
                            f"  [pr] {scenario_name}: {sps:,.0f} steps/s",
                            file=sys.stderr,
                        )
            finally:
                page.close()
                browser.close()

        # Build and write outputs
        main_out = build_output(main_results, args.iterations, args.duration)
        pr_out = build_output(pr_results, args.iterations, args.duration)

        Path(args.main_output).write_text(json.dumps(main_out, indent=2) + "\n")
        Path(args.pr_output).write_text(json.dumps(pr_out, indent=2) + "\n")

        # Summary
        print("\n=== Summary ===", file=sys.stderr)
        for scenario in SCENARIOS:
            m = main_out[scenario]["steps_per_sec"]
            pr = pr_out[scenario]["steps_per_sec"]
            diff = ((pr - m) / m * 100) if m else 0.0
            sign = "+" if diff >= 0 else ""
            print(
                f"  {scenario}: main={m:,.1f}  pr={pr:,.1f}  ({sign}{diff:.1f}%)",
                file=sys.stderr,
            )

        print(
            f"\nResults written to {args.main_output} and {args.pr_output}",
            file=sys.stderr,
        )

    finally:
        if server_proc is not None:
            server_proc.terminate()
            server_proc.wait(timeout=5)
        shutil.rmtree(serve_dir, ignore_errors=True)


if __name__ == "__main__":
    main()
