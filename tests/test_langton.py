"""End-to-end tests for the Langton's Ant WASM application."""

from playwright.sync_api import Page, expect
from conftest import load_and_wait


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def canvas_count(page: Page) -> int:
    return page.locator("canvas").count()


def set_param_value(page: Page, label_text: str, value: float | int) -> None:
    """Change a parameter by typing into its number input next to its label."""
    # Each param row is: label > slider > number-input, all inside .DebugUI-param-container
    container = page.locator(".DebugUI-param-container", has=page.locator(f"text={label_text}"))
    number_input = container.locator("input[type=number]")
    number_input.click(click_count=3)
    number_input.fill(str(value))
    number_input.dispatch_event("change")


def mark_canvas(page: Page) -> None:
    """Tag the current canvas element so we can detect if it gets replaced."""
    page.evaluate("document.querySelector('canvas')._test_marker = true")


def canvas_is_fresh(page: Page) -> bool:
    """Return True if the current canvas has no test marker (i.e. it is a new element)."""
    return not page.evaluate("!!(document.querySelector('canvas') || {})._test_marker")


# ---------------------------------------------------------------------------
# Basic smoke tests
# ---------------------------------------------------------------------------


def test_page_loads(page: Page):
    """The app serves a page with at least one canvas element."""
    load_and_wait(page)
    assert canvas_count(page) >= 1, "Expected at least one <canvas> in the DOM"


def test_debug_ui_visible(page: Page):
    """The debug UI panel renders when ?debug is in the URL."""
    load_and_wait(page)
    panel = page.locator(".DebugUI-root-box")
    expect(panel).to_be_visible()


def test_debug_ui_hidden_without_param(page: Page):
    """No debug panel without ?debug query param."""
    page.goto("http://localhost:8765/")
    page.wait_for_selector("canvas", timeout=10_000)
    page.wait_for_timeout(300)
    assert page.locator(".DebugUI-root-box").count() == 0


def test_param_sections_present(page: Page):
    """Expected section headings are visible in the debug panel."""
    load_and_wait(page)
    for section in ("Canvas", "Animation Speed", "Ants", "Visual", "Advanced"):
        expect(page.locator(f"text={section}").first).to_be_visible()


def test_live_param_change_does_not_restart(page: Page):
    """Changing a non-restart param (alpha retention) keeps exactly one canvas."""
    load_and_wait(page)
    before = canvas_count(page)
    set_param_value(page, "alpha retention", 200)
    page.wait_for_timeout(300)
    assert canvas_count(page) == before, (
        "Changing a live param should not add canvas elements"
    )


def test_close_button_removes_panel(page: Page):
    """Clicking the × button hides the debug UI."""
    load_and_wait(page)
    page.locator(".DebugUI-close-btn").click()
    expect(page.locator(".DebugUI-root-box")).to_have_count(0)


def test_url_updated_on_param_change(page: Page):
    """Changing a parameter persists it to the URL query string."""
    load_and_wait(page)
    set_param_value(page, "alpha retention", 200)
    assert "alpha_retention=200" in page.url, (
        f"Expected alpha_retention=200 in URL, got: {page.url}"
    )


# ---------------------------------------------------------------------------
# Restart-param tests
# ---------------------------------------------------------------------------


def test_start_position_params_trigger_restart(page: Page):
    """
    Changing start_x (needs_restart=true) should restart the simulation exactly
    once: the old canvas is removed and a single fresh canvas is created.
    """
    load_and_wait(page)
    assert canvas_count(page) == 1
    set_param_value(page, "start x", 0.3)
    page.wait_for_timeout(500)
    count = canvas_count(page)
    assert count == 1, (
        f"Expected exactly 1 canvas after restart (old removed, new created), got {count}."
    )


def test_start_x_restarts_fresh(page: Page):
    """Changing start_x replaces the canvas element with a new one."""
    load_and_wait(page)
    mark_canvas(page)
    set_param_value(page, "start x", 0.3)
    page.wait_for_timeout(500)
    expect(page.locator("canvas")).to_have_count(1)
    assert canvas_is_fresh(page), "Canvas element should be a new element after restart"


def test_start_y_restarts_fresh(page: Page):
    """Changing start_y replaces the canvas element with a new one."""
    load_and_wait(page)
    mark_canvas(page)
    set_param_value(page, "start y", 0.4)
    page.wait_for_timeout(500)
    expect(page.locator("canvas")).to_have_count(1)
    assert canvas_is_fresh(page), "Canvas element should be a new element after restart"


# ---------------------------------------------------------------------------
# cell_size tests — EXPECTED TO FAIL (known crash bug)
# ---------------------------------------------------------------------------


def test_cell_size_does_not_crash(page: Page):
    """
    cell_size is a live param (no restart). Changing it should update new cells
    without restarting or crashing the animation loop.
    """
    load_and_wait(page)
    assert canvas_count(page) == 1, "Precondition: exactly one canvas on load"
    mark_canvas(page)
    set_param_value(page, "cell size", 10)
    page.wait_for_timeout(500)
    expect(page.locator("canvas")).to_have_count(1)
    assert not canvas_is_fresh(page), "cell_size is a live param — canvas should not be replaced"
    # console error check happens automatically via autouse fixture


def test_cell_size_slider_back_and_forth(page: Page):
    """
    Incrementally changing cell_size up and down should not crash the loop.
    The animation must survive repeated live updates and keep exactly one canvas.
    """
    load_and_wait(page)
    assert canvas_count(page) == 1, "Precondition: exactly one canvas on load"

    steps = [25, 30, 35, 30, 25, 20, 15, 10, 15, 20]
    for value in steps:
        set_param_value(page, "cell size", value)
        page.wait_for_timeout(300)
        expect(page.locator("canvas")).to_have_count(1)
        # console errors caught by autouse fixture after the test
