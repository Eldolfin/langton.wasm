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
# Restart-param tests — the last one is EXPECTED TO FAIL
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


def test_cell_size_change_restarts_with_clean_canvas(page: Page):
    """
    EXPECTED TO FAIL: cell_size has needs_restart=true but changing it does
    not result in a fresh single-canvas state.

    After changing cell_size the simulation should:
      1. Stop the current animation.
      2. Discard the old canvas.
      3. Create a new canvas sized according to the new cell_size.

    What actually happens: the old canvas element is never removed from the
    DOM, so after the restart there are 2 (or more) <canvas> elements instead
    of exactly 1.
    """
    load_and_wait(page)
    assert canvas_count(page) == 1, "Precondition: exactly one canvas on load"

    set_param_value(page, "cell size", 10)
    page.wait_for_timeout(600)

    count = canvas_count(page)
    # This assertion SHOULD pass (1 clean canvas), but FAILS because the old
    # canvas is leaked and count == 2.
    assert count == 1, (
        f"Expected exactly 1 canvas after cell_size restart, got {count}. "
        "The old canvas element is not removed when the game restarts — "
        "needs_restart params do not produce a clean canvas state."
    )
