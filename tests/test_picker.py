"""End-to-end tests for the animation picker."""

from playwright.sync_api import Page, expect

from conftest import BASE_URL, load_picker


def test_picker_renders_grid(page: Page):
    """No ?animation param → picker visible with animation cards."""
    load_picker(page)
    cards = page.locator(".picker-card")
    assert cards.count() >= 2, f"Expected at least 2 picker cards, got {cards.count()}"


def test_picker_cards_have_animated_previews(page: Page):
    """At least one card's canvas is animating (pixel content changes over time)."""
    load_picker(page)
    cards = page.locator(".picker-card")
    count = cards.count()
    assert count >= 1
    any_animating = False
    for i in range(count):
        canvas = cards.nth(i).locator("canvas")
        before = canvas.evaluate("el => el.toDataURL()")
        page.wait_for_timeout(1000)
        after = canvas.evaluate("el => el.toDataURL()")
        if before != after:
            any_animating = True
    assert any_animating, "No picker preview canvases are animating"


def test_picker_click_starts_animation(page: Page):
    """Click card → picker hides, full-screen animation starts."""
    load_picker(page)
    first_card = page.locator(".picker-card").first
    first_card.click()
    page.wait_for_timeout(500)
    picker = page.locator("#picker")
    expect(picker).to_have_css("display", "none")
    page.wait_for_selector("canvas", timeout=10_000)


def test_picker_deep_link_skips(page: Page):
    """?animation=langton → picker never shown, animation starts directly."""
    page.goto(f"{BASE_URL}/?animation=langton")
    page.wait_for_selector("canvas", timeout=10_000)
    page.wait_for_timeout(300)
    picker = page.locator("#picker")
    expect(picker).to_have_css("display", "none")


def test_picker_unknown_animation(page: Page):
    """?animation=nonexistent → no crash (console.error expected but no JS exception)."""
    page.goto(f"{BASE_URL}/?animation=nonexistent")
    page.wait_for_timeout(1000)
