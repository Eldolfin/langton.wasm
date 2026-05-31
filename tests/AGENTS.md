# tests/ — Python E2E & Benchmark Suite

## OVERVIEW

Playwright browser tests + interleaved performance benchmarks. Python-only (no Rust tests here).

## STRUCTURE

```
tests/
├── conftest.py                # Fixtures: http_server, browser, page, console error checking
├── test_langton.py            # 15 e2e tests: smoke, debug UI, live params, restart params
├── test_screenshots.py        # 7 screenshot tests: README preset configurations
├── benchmark_interleaved.py   # Perf runner: swaps pkg dirs, measures steps/s per scenario
├── benchmark_scenarios.py     # Shared SCENARIOS dict (light/medium/heavy/ultra_heavy/full_retention)
├── benchmark_comment.py       # Posts benchmark comparison table to PR via Forgejo API
├── ci_comment.py              # Posts e2e results + screenshots to PR via Forgejo API
├── pyproject.toml             # Dependencies: playwright, pytest, pytest-xdist, pytest-timeout
└── pytest.ini                 # Default 30s timeout
```

## WHERE TO LOOK

| Task | File | Notes |
|------|------|-------|
| Add a test | test_langton.py | Use `load_and_wait(page)`, helpers at top of file |
| Add video proof | test_langton.py | Use `Shift+R` logic to record ~10s of param tweaking |
| Add benchmark scenario | benchmark_scenarios.py | Add entry to SCENARIOS dict — auto-picked up everywhere |
| Change test fixtures | conftest.py | Session-scoped server/browser, function-scoped page |
| Modify CI comment format | ci_comment.py / benchmark_comment.py | Forgejo API, marker-based upsert |

## CONVENTIONS

- **`load_and_wait(page)`**: Forces `speedup_frames=0&final_speed=50` for deterministic frames
- **Console error checking**: Autouse fixture asserts zero console errors (allowlist in `ALLOWED_CONSOLE_MSGS`)
- **xdist-safe**: HTTP server reuses port 8765 if already bound (parallel workers)
- **Port 8765**: Hardcoded everywhere — serves `crates/langton/` directory
- **Benchmark interleaving**: main/PR alternate each iteration to cancel system load drift
- **SCENARIOS dict**: Single source of truth for benchmark scenarios — insertion order = table row order

## ANTI-PATTERNS

- Never hardcode `http://localhost:8765` — use `BASE_URL` from conftest
- Never skip `load_and_wait()` — default speedup ramp makes liveness checks unreliable
- Never start a second HTTP server — check `_port_open(8765)` first
- Screenshot tests have 20s timeout (`@pytest.mark.timeout(20)`), default is 30s
