#!/usr/bin/env python3
"""Post a markdown PR comment with test results and screenshots via Forgejo API."""

import json
import os
import sys
import xml.etree.ElementTree as ET
from pathlib import Path
from urllib import parse, request
from urllib.error import HTTPError

FORGEJO_URL = os.environ.get("FORGEJO_URL", "https://codeberg.org")
TOKEN = os.environ["FORGEJO_TOKEN"]
REPO = os.environ["GITHUB_REPOSITORY"]  # e.g. eldolfin/langton.wasm
PR_NUMBER = os.environ["PR_NUMBER"]
JUNIT_PATH = Path("test-results/junit.xml")
SCREENSHOTS_DIR = Path("tests/screenshots")

API = f"{FORGEJO_URL}/api/v1"
HEADERS = {"Authorization": f"token {TOKEN}", "Content-Type": "application/json"}


def api(method: str, path: str, body: dict | None = None) -> dict:
    data = json.dumps(body).encode() if body else None
    req = request.Request(f"{API}{path}", data=data, headers=HEADERS, method=method)
    try:
        with request.urlopen(req) as r:
            body = r.read()
            return json.loads(body) if body else {}
    except HTTPError as e:
        print(f"API error {e.code}: {e.read().decode()}", file=sys.stderr)
        raise


def upload_attachment(issue_index: str, path: Path) -> str:
    """Upload a file as an issue attachment and return its download URL."""
    import email.generator
    import io
    import mimetypes

    mime = mimetypes.guess_type(path.name)[0] or "application/octet-stream"
    boundary = "boundary_forgejo_ci"
    body_parts = (
        (
            f"--{boundary}\r\n"
            f'Content-Disposition: form-data; name="attachment"; filename="{path.name}"\r\n'
            f"Content-Type: {mime}\r\n\r\n"
        ).encode()
        + path.read_bytes()
        + f"\r\n--{boundary}--\r\n".encode()
    )

    upload_headers = {
        "Authorization": f"token {TOKEN}",
        "Content-Type": f"multipart/form-data; boundary={boundary}",
    }
    req = request.Request(
        f"{API}/repos/{REPO}/issues/{issue_index}/assets",
        data=body_parts,
        headers=upload_headers,
        method="POST",
    )
    try:
        with request.urlopen(req) as r:
            d = json.load(r)
            return d["browser_download_url"]
    except HTTPError as e:
        print(f"Upload failed for {path.name}: {e.read().decode()}", file=sys.stderr)
        return ""


def parse_junit(path: Path) -> dict:
    if not path.exists():
        return {
            "total": 0,
            "passed": 0,
            "failed": 0,
            "errors": 0,
            "skipped": 0,
            "xfailed": 0,
            "failures": [],
        }

    tree = ET.parse(path)
    root = tree.getroot()
    # pytest junit: root is <testsuites> or <testsuite>
    suites = root.findall("testsuite") if root.tag == "testsuites" else [root]

    total = errors = failures = skipped = 0
    failure_details = []

    for suite in suites:
        total += int(suite.get("tests", 0))
        errors += int(suite.get("errors", 0))
        failures += int(suite.get("failures", 0))
        skipped += int(suite.get("skipped", 0))
        for tc in suite.findall("testcase"):
            f = tc.find("failure")
            if f is not None:
                failure_details.append((tc.get("name", "?"), f.text or ""))
            e = tc.find("error")
            if e is not None:
                failure_details.append((tc.get("name", "?"), e.text or ""))

    passed = total - failures - errors - skipped
    return {
        "total": total,
        "passed": passed,
        "failed": failures + errors,
        "skipped": skipped,
        "failures": failure_details,
    }


def build_comment(results: dict, screenshot_urls: dict[str, str]) -> str:
    total = results["total"]
    passed = results["passed"]
    failed = results["failed"]
    skipped = results["skipped"]

    status_icon = "✅" if failed == 0 else "❌"
    lines = [
        f"## {status_icon} E2E Test Results",
        "",
        f"| | Count |",
        f"|---|---|",
        f"| ✅ Passed | {passed} |",
    ]
    if failed:
        lines.append(f"| ❌ Failed | {failed} |")
    if skipped:
        lines.append(f"| ⏭ Skipped/xfail | {skipped} |")
    lines.append(f"| **Total** | **{total}** |")

    if results["failures"]:
        lines += ["", "### Failures", ""]
        for name, detail in results["failures"]:
            short = (detail or "").strip().splitlines()[-1][:200] if detail else ""
            lines.append(f"- **{name}**: `{short}`")

    if screenshot_urls:
        lines += ["", "### Screenshots", ""]
        for name, url in sorted(screenshot_urls.items()):
            label = name.replace("_", " ").title()
            lines.append(f"**{label}**")
            lines.append(f"![{label}]({url})")
            lines.append("")

    return "\n".join(lines)


def delete_bot_comments(issue_index: str) -> None:
    """Remove previous bot comments to avoid spam on repeated runs."""
    comments = api("GET", f"/repos/{REPO}/issues/{issue_index}/comments")
    for c in comments:
        if "E2E Test Results" in c.get("body", ""):
            api("DELETE", f"/repos/{REPO}/issues/comments/{c['id']}")


def main() -> None:
    results = parse_junit(JUNIT_PATH)

    # Upload screenshots
    screenshot_urls: dict[str, str] = {}
    if SCREENSHOTS_DIR.exists():
        for png in sorted(SCREENSHOTS_DIR.glob("*.png")):
            print(f"Uploading {png.name}...")
            url = upload_attachment(PR_NUMBER, png)
            if url:
                screenshot_urls[png.stem] = url

    body = build_comment(results, screenshot_urls)

    # Delete stale bot comment, then post fresh one
    delete_bot_comments(PR_NUMBER)
    api("POST", f"/repos/{REPO}/issues/{PR_NUMBER}/comments", {"body": body})
    print("Comment posted.")

    if results["failed"] > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
