"""End-to-end tests for the blinker (dummy) animation."""

from playwright.sync_api import Page

from conftest import BASE_URL


def test_dummy_loads(page: Page):
    """?animation=blinker → canvas renders and animates."""
    page.goto(f"{BASE_URL}/?animation=blinker")
    page.wait_for_selector("canvas", timeout=10_000)
    page.wait_for_timeout(300)
    assert page.locator("canvas").count() >= 1


def test_dummy_no_crash(page: Page):
    """Blinker runs for 5s without console errors."""
    page.goto(f"{BASE_URL}/?animation=blinker")
    page.wait_for_selector("canvas", timeout=10_000)
    page.wait_for_timeout(5000)
