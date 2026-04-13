#!/usr/bin/env python3
"""Interleaved Playwright benchmark for Langton's Ant WASM app.

Instead of running all main iterations then all PR iterations (which is
sensitive to system load drift), this runner interleaves them:

    iteration 1: main light/medium/heavy, PR light/medium/heavy
    iteration 2: main light/medium/heavy, PR light/medium/heavy
    ...

Two pre-built pkg/ directories are swapped into crates/langton/pkg/
before each variant's measurement. A single HTTP server and browser
instance are reused throughout.
"""

import argparse
import json
import shutil
import socket
import subprocess
import sys
import time
from pathlib import Path
from statistics import mean

from playwright.sync_api import sync_playwright

REPO_ROOT = Path.cwd()
SERVE_DIR = str(REPO_ROOT / "crates" / "langton")
PKG_DST = REPO_ROOT / "crates" / "langton" / "pkg"
BASE_URL = "http://localhost:8765"

SCENARIOS = {
    "light": {"number_of_ants": 2, "cell_size": 20},
    "medium": {"number_of_ants": 50, "cell_size": 10},
    "heavy": {"number_of_ants": 500, "cell_size": 5},
}


def _port_open(port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.settimeout(0.1)
        return s.connect_ex(("localhost", port)) == 0


def start_http_server() -> subprocess.Popen | None:
    """Start HTTP server on port 8765 serving crates/langton/."""
    if _port_open(8765):
        print("Port 8765 already in use, reusing existing server", file=sys.stderr)
        return None

    proc = subprocess.Popen(
        ["python3", "-m", "http.server", "8765", "--directory", SERVE_DIR],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    for _ in range(40):
        if _port_open(8765):
            return proc
        time.sleep(0.1)
    proc.kill()
    raise RuntimeError("HTTP server did not start in time")


def parse_steps(text: str) -> int:
    """Parse step counter text like 'Steps: 12,345' → 12345."""
    return int(text.replace("Steps:", "").replace(",", "").strip())


def swap_pkg(src: Path) -> None:
    """Copy src pkg dir → crates/langton/pkg/, overwriting."""
    shutil.copytree(src, PKG_DST, dirs_exist_ok=True)


def measure_scenario(
    browser, scenario_name: str, params: dict, duration_s: float
) -> float:
    """Run one scenario, return steps_per_sec."""
    ants = params["number_of_ants"]
    cell_size = params["cell_size"]
    url = (
        f"{BASE_URL}/"
        f"?debug&speedup_frames=0&final_speed=1000"
        f"&number_of_ants={ants}&cell_size={cell_size}"
    )

    ctx = browser.new_context()
    page = ctx.new_page()
    try:
        page.goto(url)
        page.wait_for_selector("canvas", timeout=15_000)
        page.wait_for_selector(".DebugUI-step-counter", timeout=15_000)

        # Let simulation stabilize briefly
        page.wait_for_timeout(500)

        # Read initial step count
        el = page.query_selector(".DebugUI-step-counter")
        initial_steps = parse_steps(el.inner_text())
        t_start = time.monotonic()

        # Wait for benchmark duration
        page.wait_for_timeout(int(duration_s * 1000))

        # Read final step count
        el = page.query_selector(".DebugUI-step-counter")
        final_steps = parse_steps(el.inner_text())
        t_end = time.monotonic()

        elapsed = t_end - t_start
        steps = final_steps - initial_steps
        steps_per_sec = steps / elapsed if elapsed > 0 else 0.0

        return steps_per_sec
    finally:
        ctx.close()


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
            "steps_per_sec": round(mean(vals), 1) if vals else 0.0,
            "iterations": len(vals),
        }
    return out


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Interleaved benchmark: alternates main/PR each iteration"
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

    server_proc = start_http_server()
    try:
        with sync_playwright() as p:
            browser = p.chromium.launch(headless=True)
            try:
                for i in range(1, args.iterations + 1):
                    print(f"Iteration {i}/{args.iterations}", file=sys.stderr)

                    # --- main ---
                    swap_pkg(main_pkg)
                    for scenario_name, params in SCENARIOS.items():
                        sps = measure_scenario(
                            browser, scenario_name, params, args.duration
                        )
                        main_results[scenario_name].append(sps)
                        print(
                            f"  [main] {scenario_name}: {sps:,.0f} steps/s",
                            file=sys.stderr,
                        )

                    # --- PR ---
                    swap_pkg(pr_pkg)
                    for scenario_name, params in SCENARIOS.items():
                        sps = measure_scenario(
                            browser, scenario_name, params, args.duration
                        )
                        pr_results[scenario_name].append(sps)
                        print(
                            f"  [pr] {scenario_name}: {sps:,.0f} steps/s",
                            file=sys.stderr,
                        )
            finally:
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


if __name__ == "__main__":
    main()
