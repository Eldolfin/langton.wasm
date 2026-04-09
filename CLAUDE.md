# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Parametrized Langton's Ant simulator — Rust compiled to WebAssembly, runs in the browser. Live at cv.eldolfin.top.

## Build & Dev Commands

```bash
just build-web          # Build WASM for web (wasm-pack --target web)
just build-web --dev    # Debug build
just dev                # Dev server with hot reload (live-server + entr)
cargo test --verbose    # Run all tests
cargo clippy --verbose -- -Dwarnings  # Lint (CI enforces zero warnings)
```

Nix flake provides the dev environment (`use flake` via direnv). Alternatively, `.devcontainer/` for Docker-based setup.

## Architecture

Cargo workspace with three crates:

- **`src/langton`** — Main cdylib crate. Entry point: `start_langton_ant()` (wasm_bindgen). Contains `Game` (board state + simulation loop), `Ant` (position/direction), and `GameConfig` (all parameters). Implements the Langton's Ant cellular automaton rules.
- **`lib/canvas`** — HTML5 Canvas 2D abstraction. Queue-based batched rendering with optimization (dedup draws, skip unchanged cells, sort by color). Handles `requestAnimationFrame` loop via async/await promises.
- **`lib/debug_ui`** — Interactive parameter UI. Creates sliders/inputs, persists state in URL query params, communicates values via mpsc channels. Enabled when `?debug` is in the URL.

Data flow: DebugUI params → Game reads each frame → queues Canvas draw calls → Canvas flushes optimized batch.

## CI

GitHub Actions runs `cargo test` and `cargo clippy -- -Dwarnings` on push to main and PRs.

## Pre-commit Hooks

Configured via Nix flake: rustfmt, clippy, alejandra (Nix fmt), markdownlint, trailing whitespace, merge conflict checks.
