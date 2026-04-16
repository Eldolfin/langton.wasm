# PROJECT KNOWLEDGE BASE

**Generated:** 2025-04-16
**Commit:** aa71870
**Branch:** fix/benchmark

## OVERVIEW

Parametrized Langton's Ant simulator — Rust workspace (3 crates) compiled to WASM via wasm-pack, runs in browser. Hosted at cv.eldolfin.top, repo on Codeberg.

## STRUCTURE

```
./
├── crates/
│   ├── langton/        # cdylib entry point: start_langton_ant(), Game, Ant, GameConfig
│   ├── canvas/         # Canvas2D abstraction: queue-based batched rendering, dedup, color sort
│   └── debug_ui/       # Interactive param UI: sliders, URL persistence, mpsc channels
├── tests/              # Python e2e: Playwright browser tests + benchmark scripts (NOT Rust)
├── .github/workflows/  # CI (fmt/test/clippy/e2e), benchmark (interleaved PR vs main), pages deploy
├── index.html          # Browser entry: imports WASM, calls start_langton_ant()
├── justfile            # Task runner: build-web, dev, benchmark, test-e2e, deploy
└── flake.nix           # Nix dev env with pre-commit hooks
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Simulation logic | `crates/langton/src/lib.rs` | Game::run() animation closure, Ant::move_forward() |
| Rendering pipeline | `crates/canvas/src/lib.rs` | Queue → optimise_queue() → flush(). Dedup + skip unchanged + sort by color |
| Adding a parameter | `crates/langton/src/lib.rs` | Add `debug_ui.param(ParamParam{...})`, wire into GameConfig |
| URL param persistence | `crates/debug_ui/src/lib.rs` | add_url_param(), remove_url_param() via History.pushState |
| Benchmark scenarios | `tests/benchmark_scenarios.py` | Shared SCENARIOS dict, all benchmark scripts import from here |
| CI debugging | See CLAUDE.md | `fj actions tasks`, curl logs. Job indices: fmt=0, check=1, clippy=2, e2e=3 |

## CODE MAP

| Symbol | Type | Location | Role |
|--------|------|----------|------|
| `start_langton_ant` | fn (wasm_bindgen) | langton/lib.rs:8 | **Sole WASM export**. Async, never returns (infinite loop) |
| `Game::run` | async method | langton/lib.rs:200 | Main simulation loop: step accumulation, ant movement, canvas flush |
| `Game::balance_ants` | method | langton/lib.rs:270 | Dynamic ant count: add/remove to match param |
| `Canvas::play_animation` | async fn | canvas/lib.rs:160 | requestAnimationFrame loop via Promise |
| `Canvas::optimise_queue` | method | canvas/lib.rs:208 | Dedup → skip unchanged → sort by color |
| `Canvas::flush` | method | canvas/lib.rs:234 | Batched fill_rect calls with border handling |
| `Canvas::fill_canvas` | method | canvas/lib.rs:175 | Alpha fade via destination-in compositing |
| `DebugUI::new` | fn | debug_ui/lib.rs:200 | Checks `?debug` URL param → Enabled or Disabled |
| `DebugUI::param` | method | debug_ui/lib.rs:271 | Creates slider+input, reads default from URL, returns mpsc Param<T> |
| `Param::get` | method | debug_ui/lib.rs:85 | Drains mpsc receiver, returns latest value |
| `Scale::scale/unscale` | methods | debug_ui/lib.rs:475 | Linear passthrough or logarithmic transform |

## CONVENTIONS

- **Edition 2024** across all crates (Rust 1.94.0 pinned)
- **Monolithic lib.rs** per crate — no submodules. All types, impls, tests in single file
- **CSS embedded via `include_str!`** in Rust crates (canvas/src/style.css, debug_ui/src/style.css)
- **Symlinks** in crates/langton/ → root files (index.html, README.md, LICENSE) for wasm-pack pkg
- **rstest** for parametrized Rust unit tests (`#[rstest]` + `#[case]`)
- **No wasm-bindgen-test** — WASM tested entirely via Python Playwright e2e
- URL param names: snake_case (e.g. `alpha_retention`, `cell_size`, `number_of_ants`)

## ANTI-PATTERNS (THIS PROJECT)

- **Zero clippy warnings**: CI enforces `cargo clippy -- -Dwarnings`. Any warning = build failure
- **No `unsafe`**: Codebase is 100% safe Rust
- **`.unwrap()` is acceptable** for DOM operations in WASM context (panic hook installed via `console_error_panic_hook`)

## UNIQUE STYLES

- `shit_ease_in()` — intentionally profane function name (langton/lib.rs:195)
- `selff` parameter name in `Canvas::play_animation` — avoids `self` keyword conflict
- `Est` variant in Direction enum — French spelling of "East"
- `ParamParam` struct name — "parameters of a parameter"
- Default param name: `"UNDEFINED 🤡"` (debug_ui/lib.rs:69)

## COMMANDS

```bash
just build-web              # Build WASM (wasm-pack --target web)
just build-web --dev        # Debug build
just dev                    # Hot-reload dev server (entr + live-server)
cargo test --verbose        # Run Rust unit tests
cargo clippy --verbose -- -Dwarnings  # Lint (zero warnings enforced)
just test-e2e               # Python Playwright e2e tests
just benchmark main 10 5    # Interleaved perf benchmark (PR vs main)
```

## NOTES

- **Debug UI activation**: Append `?debug` to URL. Without it, DebugUI::Disabled — no panel, no step counter
- **Animation restart**: Parameters with `needs_restart: true` (start_x, start_y) destroy and recreate Canvas
- **Benchmark interleaving**: Alternates main/PR measurements each iteration to eliminate system load drift
- **Canvas is ephemeral**: Dropped and recreated each game loop iteration (RAII cleanup via `impl Drop`)
- **Build image**: Custom Docker image cached by content hash of Dockerfile+mise.toml+pyproject.toml+uv.lock
- **Codeberg CI**: Runs on `codeberg-medium` runners with custom container, NOT GitHub-hosted
- **Release profile**: LTO + opt-level=3 + codegen-units=1 + wasm-opt -Oz (aggressive size optimization)
