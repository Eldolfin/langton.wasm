#!/usr/bin/env python3
"""Standalone Playwright benchmark for Langton's Ant WASM app.

Runs three scenarios (light/medium/heavy) measuring steps/sec,
outputs JSON results to stdout or a file.
"""

import argparse
import http.server
import json
import socket
import subprocess
import sys
import threading
import time
from pathlib import Path

from playwright.sync_api import sync_playwright

REPO_ROOT = Path.cwd()
SERVE_DIR = str(REPO_ROOT / "crates" / "langton")
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


def run_scenario(browser, scenario_name: str, params: dict, duration_s: float) -> dict:
    """Run a single benchmark scenario, return results dict."""
    ants = params["number_of_ants"]
    cell_size = params["cell_size"]
    url = (
        f"{BASE_URL}/"
        f"?debug&speedup_frames=0&final_speed=1000"
        f"&number_of_ants={ants}&cell_size={cell_size}"
    )

    print(
        f"  [{scenario_name}] ants={ants} cell_size={cell_size} duration={duration_s}s",
        file=sys.stderr,
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
        initial_text = el.inner_text()
        initial_steps = parse_steps(initial_text)
        t_start = time.monotonic()

        # Wait for benchmark duration
        page.wait_for_timeout(int(duration_s * 1000))

        # Read final step count
        el = page.query_selector(".DebugUI-step-counter")
        final_text = el.inner_text()
        final_steps = parse_steps(final_text)
        t_end = time.monotonic()

        actual_duration = t_end - t_start
        steps = final_steps - initial_steps
        steps_per_sec = steps / actual_duration if actual_duration > 0 else 0

        print(
            f"  [{scenario_name}] {steps} steps in {actual_duration:.1f}s → {steps_per_sec:.1f} steps/s",
            file=sys.stderr,
        )

        return {
            "steps": steps,
            "duration_s": round(actual_duration, 2),
            "steps_per_sec": round(steps_per_sec, 2),
        }
    finally:
        ctx.close()


def main() -> None:
    parser = argparse.ArgumentParser(description="Benchmark Langton's Ant WASM app")
    parser.add_argument(
        "--duration",
        type=float,
        default=30,
        help="Duration per scenario in seconds (default: 30)",
    )
    parser.add_argument(
        "--output",
        type=str,
        default=None,
        help="Output JSON file path (default: stdout)",
    )
    args = parser.parse_args()

    server_proc = start_http_server()
    try:
        with sync_playwright() as p:
            browser = p.chromium.launch(headless=True)
            try:
                results = {}
                for name, params in SCENARIOS.items():
                    results[name] = run_scenario(browser, name, params, args.duration)
            finally:
                browser.close()

        output = json.dumps(results, indent=2)

        if args.output:
            Path(args.output).write_text(output + "\n")
            print(f"Results written to {args.output}", file=sys.stderr)
        else:
            print(output)

    finally:
        if server_proc is not None:
            server_proc.terminate()
            server_proc.wait(timeout=5)


if __name__ == "__main__":
    main()
