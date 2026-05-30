"""Video proof tests for all animations."""

import pytest
from playwright.sync_api import Page, expect
from pathlib import Path
from conftest import BASE_URL, load_and_wait

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def set_param_value(page: Page, label_text: str, value: float | int) -> None:
    """Change a parameter by typing into its number input next to its label."""
    container = page.locator(
        ".DebugUI-param-container", has=page.locator(f"text={label_text}")
    )
    number_input = container.locator("input[type=number]")
    number_input.click(click_count=3, force=True)
    number_input.fill(str(value), force=True)
    number_input.dispatch_event("change")

def record_video_proof(page: Page, animation_id: str, tweak_fn):
    """Generic helper to record ~10s of an animation with parameter tweaks."""
    videos_dir = Path(__file__).parent / "videos"
    videos_dir.mkdir(exist_ok=True)
    video_path = videos_dir / f"proof_{animation_id}.webm"

    # Set viewport for HD recording
    page.set_viewport_size({"width": 1920, "height": 1080})

    # Start with forced deterministic settings
    params = f"?animation={animation_id}&debug&speedup_frames=0&final_speed=50"
    page.goto(f"{BASE_URL}/{params}")
    page.wait_for_selector("canvas", timeout=10_000)
    page.wait_for_timeout(300)

    # Start recording
    page.keyboard.press("Shift+R")
    page.wait_for_timeout(1000)

    # Re-enable UI (Shift+R hides it)
    page.keyboard.press("Shift+I")
    page.wait_for_timeout(500)

    # Run custom tweaks
    tweak_fn(page)

    # Wait to reach ~10s total
    page.wait_for_timeout(2000)

    # Stop recording and expect download
    with page.expect_download() as download_info:
        page.keyboard.press("Shift+R")

    download = download_info.value
    download.save_as(str(video_path))
    assert video_path.exists()
    assert video_path.stat().st_size > 0

# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------

def test_video_proof_langton(page: Page):
    def tweaks(p):
        set_param_value(p, "number of ants", 100)
        p.wait_for_timeout(2000)
        set_param_value(p, "cell size", 5)
        p.wait_for_timeout(2000)
        set_param_value(p, "alpha retention", 200)
        p.wait_for_timeout(2000)
    
    record_video_proof(page, "langton", tweaks)

def test_video_proof_blinker(page: Page):
    def tweaks(p):
        set_param_value(p, "cell size", 40)
        p.wait_for_timeout(3000)
        set_param_value(p, "alpha retention", 100)
        p.wait_for_timeout(3000)
    
    record_video_proof(page, "blinker", tweaks)

def test_video_proof_cube(page: Page):
    def tweaks(p):
        set_param_value(p, "rotation speed", 5)
        p.wait_for_timeout(2000)
        set_param_value(p, "cube size", 200)
        p.wait_for_timeout(2000)
        set_param_value(p, "alpha retention", 150)
        p.wait_for_timeout(2000)
    
    record_video_proof(page, "cube", tweaks)

def test_video_proof_sierpinski(page: Page):
    def tweaks(p):
        set_param_value(p, "final speed", 500)
        p.wait_for_timeout(3000)
        set_param_value(p, "alpha retention", 50)
        p.wait_for_timeout(3000)
    
    record_video_proof(page, "sierpinski", tweaks)
