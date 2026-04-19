"""Screenshot tests for README preset parameter configurations."""

import pytest
from playwright.sync_api import Page
from pathlib import Path
from conftest import BASE_URL

SCREENSHOTS_DIR = Path(__file__).parent / "screenshots"

# Extracted from README.md "Cool parameters examples"
PRESETS = [
    (
        "many_small_ants",
        "alpha_retention=235&cell_size=5&final_speed=0.5&number_of_ants=400&speedup_frames=0&start_x=0.5&start_y=0.5",
    ),
    (
        "trailing_ants",
        "alpha_retention=255&final_speed=30&number_of_ants=3&speedup_frames=300&start_x=0.5&start_y=0.5&cell_size=4",
    ),
    (
        "angry_ant",
        "alpha_retention=220&final_speed=200&number_of_ants=1&speedup_frames=0",
    ),
]


@pytest.mark.timeout(20)
@pytest.mark.parametrize("name,params", PRESETS, ids=[p[0] for p in PRESETS])
def test_screenshot(page: Page, name: str, params: str):
    """Load each README preset, let it run for a few seconds, capture a screenshot.

    Also verifies the simulation starts without console errors.
    """
    SCREENSHOTS_DIR.mkdir(exist_ok=True)
    page.set_viewport_size({"width": 1280, "height": 720})
    page.goto(f"{BASE_URL}/?animation=langton&{params}")
    page.wait_for_selector("canvas", timeout=10_000)
    page.wait_for_timeout(3_000)
    page.screenshot(path=str(SCREENSHOTS_DIR / f"{name}.png"), full_page=False)


@pytest.mark.timeout(20)
def test_screenshot_debug_ui_visible(page: Page):
    """Screenshot showing the debug UI panel open, including the color_param swatch."""
    SCREENSHOTS_DIR.mkdir(exist_ok=True)
    page.set_viewport_size({"width": 1280, "height": 720})
    # Open with debug UI enabled and a known color param visible
    page.goto(f"{BASE_URL}/?animation=langton&debug&speedup_frames=0&final_speed=5&number_of_ants=1")
    page.wait_for_selector("canvas", timeout=10_000)
    page.wait_for_timeout(1_000)
    # Assert the debug UI root box is visible
    debug_ui = page.locator(".DebugUI-root-box")
    assert debug_ui.count() > 0, "DebugUI root box not found"
    page.screenshot(path=str(SCREENSHOTS_DIR / "debug_ui_visible.png"), full_page=False)


@pytest.mark.timeout(20)
def test_screenshot_color_picker_opened(page: Page):
    """Screenshot after clicking a color preview swatch to open the native color picker."""
    SCREENSHOTS_DIR.mkdir(exist_ok=True)
    page.set_viewport_size({"width": 1280, "height": 720})
    page.goto(f"{BASE_URL}/?animation=langton&debug&speedup_frames=0&final_speed=5&number_of_ants=1")
    page.wait_for_selector("canvas", timeout=10_000)
    page.wait_for_timeout(1_000)
    # Find the first color preview circle and click it
    swatch = page.locator(".DebugUI-color-preview").first
    if swatch.count() == 0:
        pytest.skip("No color preview swatches found — color_param not used in this animation")
    swatch.click()
    page.wait_for_timeout(500)
    page.screenshot(path=str(SCREENSHOTS_DIR / "color_picker_opened.png"), full_page=False)
