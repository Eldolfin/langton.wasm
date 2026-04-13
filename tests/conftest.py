"""Pytest configuration and fixtures for end-to-end tests."""

import socket
import subprocess
import time
from pathlib import Path

import pytest
from playwright.sync_api import Page, sync_playwright

REPO_ROOT = Path(__file__).parent.parent
SERVE_DIR = str(REPO_ROOT / "crates" / "langton")
BASE_URL = "http://localhost:8765"


def _port_open(port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.settimeout(0.1)
        return s.connect_ex(("localhost", port)) == 0


@pytest.fixture(scope="session")
def http_server():
    """Serve crates/langton/ over HTTP for the duration of the test session.

    Safe for parallel xdist workers: if the port is already bound (by another
    worker or an external process) we reuse it instead of starting a second server.
    """
    # Brief retry loop so a sibling xdist worker that just won the race has time
    # to actually bind the port before we give up and try ourselves.
    for _ in range(10):
        if _port_open(8765):
            yield BASE_URL
            return
        time.sleep(0.05)

    proc = subprocess.Popen(
        ["python3", "-m", "http.server", "8765", "--directory", SERVE_DIR],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    for _ in range(20):
        if _port_open(8765):
            break
        time.sleep(0.1)
    else:
        proc.kill()
        raise RuntimeError("HTTP server did not start in time")

    yield BASE_URL
    # Don't terminate: with xdist parallel workers, other sessions may still
    # be using the server. The child process dies when pytest exits anyway.


@pytest.fixture(scope="session")
def browser():
    with sync_playwright() as p:
        br = p.chromium.launch(headless=True, args=["--window-size=1920,1080"])
        yield br
        br.close()


@pytest.fixture
def page(browser, http_server):
    """Fresh browser page for each test, with console error collection."""
    ctx = browser.new_context()
    pg = ctx.new_page()
    pg._console_msgs = []

    def _on_console(m):
        pg._console_msgs.append(f"[{m.type}]: {m.text} (location: {m.location})")

    pg.on("console", _on_console)
    pg.on("pageerror", lambda e: pg._console_msgs.append(str(e)))
    yield pg
    ctx.close()


ALLOWED_CONSOLE_MSGS = [
    "[LANGTON][CANVAS] body.scroll_height is 0, make sure to fully initialize the page before calling start_langton_ant otherwise the canvas might get cut off at the bottom"
]


@pytest.fixture(autouse=True)
def check_no_console_msgs(page):
    """Assert no console errors were emitted during any test."""
    yield
    msgs = page._console_msgs
    msgs = [msg for msg in msgs if msg not in ALLOWED_CONSOLE_MSGS]
    assert not msgs, "Console messages:\n" + "\n".join(msgs)


def load_and_wait(page: Page, extra_params: str = "") -> None:
    """Navigate to the app and wait for the canvas to appear.

    speedup_frames=0 and final_speed=50 are forced so the simulation produces
    frames immediately — without this the default 1300-frame ramp-up means
    near-zero steps in the first few hundred milliseconds, making liveness
    checks unreliable.
    """
    params = f"?debug&speedup_frames=0&final_speed=50{extra_params}"
    page.goto(f"{BASE_URL}/{params}")
    # Wait for the canvas element to be added to the DOM
    page.wait_for_selector("canvas", timeout=10_000)
    # Give the WASM a couple of frames to initialise
    page.wait_for_timeout(300)
