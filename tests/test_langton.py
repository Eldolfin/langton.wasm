"""End-to-end tests for the Langton's Ant WASM application."""

import pytest
from playwright.sync_api import Page, expect

from conftest import load_and_wait

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def canvas_count(page: Page) -> int:
    return page.locator("canvas").count()


def canvas_is_animating(page: Page, wait_ms: int = 400) -> bool:
    """Return True if the canvas pixel content changes over time (loop is alive)."""
    before = page.evaluate("document.querySelector('canvas').toDataURL()")
    page.wait_for_timeout(wait_ms)
    after = page.evaluate("document.querySelector('canvas').toDataURL()")
    return before != after


def set_param_value(page: Page, label_text: str, value: float | int) -> None:
    """Change a parameter by typing into its number input next to its label."""
    # Each param row is: label > slider > number-input, all inside .DebugUI-param-container
    container = page.locator(
        ".DebugUI-param-container", has=page.locator(f"text={label_text}")
    )
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
# Infrastructure self-test
# ---------------------------------------------------------------------------


def test_console_error_detection(page: Page):
    """Verify the console error listener actually captures errors.

    Injects a console.error via page.evaluate, then asserts it was captured
    in page._console_errors. Clears the list afterwards so the autouse
    fixture does not double-fire on the same sentinel.
    """
    load_and_wait(page)
    page.evaluate("console.error('sentinel error from test infrastructure check')")
    page.wait_for_timeout(100)
    captured = list(page._console_msgs)  # type: ignore
    page._console_msgs.clear()  # type: ignore
    assert captured, (
        "Console error detection is broken: injected console.error was not captured. "
        "Check that page.on('console', ...) is wired up in the page fixture."
    )


# ---------------------------------------------------------------------------
# Basic smoke tests
# ---------------------------------------------------------------------------


def test_page_loads(page: Page):
    """The app serves a page with a canvas that is actively animating."""
    load_and_wait(page)
    assert canvas_count(page) >= 1, "Expected at least one <canvas> in the DOM"
    assert canvas_is_animating(page), (
        "Canvas must be producing new frames (animation loop running)"
    )


def test_debug_ui_visible(page: Page):
    """The debug UI panel renders when ?debug is in the URL."""
    load_and_wait(page)
    panel = page.locator(".DebugUI-root-box")
    expect(panel).to_be_visible()


def test_debug_ui_hidden_without_param(page: Page):
    """No debug panel without ?debug query param."""
    page.goto("http://localhost:8765/?animation=langton")
    page.wait_for_selector("canvas", timeout=10_000)
    page.wait_for_timeout(300)
    expect(page.locator(".DebugUI-root-box")).not_to_be_visible()


def test_param_sections_present(page: Page):
    """Expected section headings are visible in the debug panel."""
    load_and_wait(page)
    for section in ("Canvas", "Animation Speed", "Ants", "Visual", "Advanced"):
        expect(page.locator(".DebugUI-section-title", has_text=section).first).to_be_visible()


def test_live_param_change_does_not_restart(page: Page):
    """Changing a non-restart param (alpha retention) keeps exactly one canvas."""
    load_and_wait(page)
    before = canvas_count(page)
    set_param_value(page, "alpha retention", 200)
    page.wait_for_timeout(300)
    assert canvas_count(page) == before, (
        "Changing a live param should not add canvas elements"
    )


def test_close_button_hides_panel(page: Page):
    """Clicking the × button hides the debug UI panel."""
    load_and_wait(page)
    panel = page.locator(".DebugUI-root-box")
    page.locator(".DebugUI-close-btn").click()
    expect(panel).to_have_css("display", "none")


def test_shift_i_toggles_panel_after_close(page: Page):
    """After closing with button, shift+I can show the panel again."""
    load_and_wait(page)
    panel = page.locator(".DebugUI-root-box")
    # Close with button
    page.locator(".DebugUI-close-btn").click()
    expect(panel).to_have_css("display", "none")
    # Show with shift+I
    page.keyboard.press("Shift+I")
    expect(panel).not_to_have_css("display", "none")


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
    """Changing start_x restarts the simulation (canvas cleared, animation continues)."""
    load_and_wait(page)
    set_param_value(page, "start x", 0.3)
    page.wait_for_timeout(500)
    expect(page.locator("canvas")).to_have_count(1)
    assert canvas_is_animating(page), "Animation must keep running after restart"


def test_start_y_restarts_fresh(page: Page):
    """Changing start_y restarts the simulation (canvas cleared, animation continues)."""
    load_and_wait(page)
    set_param_value(page, "start y", 0.4)
    page.wait_for_timeout(500)
    expect(page.locator("canvas")).to_have_count(1)
    assert canvas_is_animating(page), "Animation must keep running after restart"


def test_restart_param_slider_triggers_restart(page: Page):
    """
    Dragging the slider for a needs_restart=true param should restart the game.

    The slider fires 'input' events on every movement. When the user releases
    the slider (or the value settles), a restart must be triggered — identical
    behaviour to typing in the number input.

    This test catches the bug where the slider 'input' handler sends the new
    value but never sets needs_restart, so the game silently ignores the change.
    """
    load_and_wait(page)

    container = page.locator(
        ".DebugUI-param-container", has=page.locator("text=start x")
    )
    slider = container.locator("input[type=range]")
    slider.fill("0.3")
    slider.dispatch_event("input")
    page.wait_for_timeout(500)

    expect(page.locator("canvas")).to_have_count(1)
    assert canvas_is_animating(page), (
        "Animation must keep running after slider change on a needs_restart param"
    )


# ---------------------------------------------------------------------------
# cell_size live-param tests
# ---------------------------------------------------------------------------


def test_cell_size_does_not_crash(page: Page):
    """
    cell_size is a live param (no restart). Changing it should update new cells
    without restarting or crashing the animation loop.
    """
    load_and_wait(page)
    assert canvas_count(page) == 1, "Precondition: exactly one canvas on load"
    assert canvas_is_animating(page), (
        "Precondition: animation must be running before test"
    )
    mark_canvas(page)
    set_param_value(page, "cell size", 10)
    page.wait_for_timeout(500)
    expect(page.locator("canvas")).to_have_count(1)
    assert not canvas_is_fresh(page), (
        "cell_size is a live param — canvas should not be replaced"
    )
    assert canvas_is_animating(page), (
        "Animation loop must still be running after cell_size change"
    )


def test_cell_size_slider_back_and_forth(page: Page):
    """
    Incrementally changing cell_size up and down should not crash the loop.
    The animation must survive repeated live updates and keep exactly one canvas.
    """
    load_and_wait(page)
    assert canvas_count(page) == 1, "Precondition: exactly one canvas on load"
    assert canvas_is_animating(page), (
        "Precondition: animation must be running before test"
    )

    steps = [25, 30, 35, 30, 25, 20, 15, 10, 15, 20]
    for value in steps:
        set_param_value(page, "cell size", value)
        page.wait_for_timeout(300)
        expect(page.locator("canvas")).to_have_count(1)

    assert canvas_is_animating(page), (
        "Animation loop must still be running after all cell_size changes"
    )


# ---------------------------------------------------------------------------
# Presets dropdown tests
# ---------------------------------------------------------------------------


def test_presets_dropdown_visible(page: Page):
    """Presets select is visible in the debug panel."""
    load_and_wait(page)
    select = page.locator(".DebugUI-presets-select")
    expect(select).to_be_visible()


def test_presets_default_option(page: Page):
    """Default option is the placeholder '— Presets —'."""
    load_and_wait(page)
    select = page.locator(".DebugUI-presets-select")
    first_option = select.locator("option").first
    expect(first_option).to_contain_text("Presets")


def test_presets_has_expected_options(page: Page):
    """All preset names appear as options."""
    load_and_wait(page)
    select = page.locator(".DebugUI-presets-select")
    for name in ["Many small ants", "3 trailing ants", "Angry ant", "Chaos"]:
        expect(select.locator(f"option[label='{name}'], option:text('{name}')")).to_be_attached()


def test_preset_selection_navigates(page: Page):
    """Selecting a preset navigates to a URL containing its params."""
    load_and_wait(page)
    page.locator(".DebugUI-presets-select").select_option(label="Angry ant")
    page.wait_for_load_state("networkidle", timeout=10_000)
    page.wait_for_selector("canvas", timeout=10_000)
    assert "number_of_ants=1" in page.url
    assert "alpha_retention=220" in page.url
    assert "final_speed=200" in page.url
    assert "animation=langton" in page.url
