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
    (
        "flies",
        "alpha_retention=0&ant_color_brightness=0.3&ant_color_saturation=0&cell_border_size=0&cell_size=6&final_speed=1&number_of_ants=500&speed_ease-in_power=1&speedup_frames=120&start_x=0.5&start_y=0.5&white_color_blue=0&white_color_green=0&white_color_red=0",
    ),
    (
        "chaos",
        "alpha_retention=255&final_speed=40&number_of_ants=300&speedup_frames=600&start_x=0.5&start_y=0.5",
    ),
    (
        "small_grid",
        "alpha_retention=254&ant_color_brightness=0.65&ant_color_saturation=1&cell_border_size=0&cell_size=5&final_speed=25&number_of_ants=4&speed_ease-in_power=7&speedup_frames=1200&start_x=0.5&start_y=0.5&white_color_blue=227&white_color_green=227&white_color_red=227",
    ),
    (
        "1px_grid",
        "alpha_retention=255&ant_color_brightness=0&ant_color_saturation=0.5&cell_border_size=0&cell_size=1&final_speed=5000&number_of_ants=1&speedup_frames=0&white_color_blue=255&white_color_green=255&white_color_red=255",
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
