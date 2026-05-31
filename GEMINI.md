# Langton's Ant Wasm Project

A parametrized Langton's Ant simulator written in Rust, targeting WebAssembly for the browser. It features multiple simulation types, a real-time debug UI for parameter adjustment, and high-performance rendering.

## Architecture

The project is organized as a Rust workspace with multiple specialized crates:

- **`crates/app`**: The main entry point for the Wasm application. It registers simulations, handles the animation registry, and exports functions for the web interface.
- **`crates/engine`**: Core simulation framework. Defines the `Simulation` trait and the `SimulationRunner` which manages the animation loop, frame timing, and canvas updates.
- **`crates/langton`**: Implementation of the Langton's Ant simulation logic.
- **`crates/canvas`**: A lightweight wrapper around the HTML5 Canvas 2D API for efficient pixel-based rendering.
- **`crates/debug_ui`**: Provides a real-time UI overlay for tweaking simulation parameters (speed, colors, alpha retention, etc.).
- **Other Simulations**: Additional implementations like `blinker`, `cube` (3D), and `sierpinski` (Chaos Game).

## Prerequisites

- [Rust](https://www.rust-lang.org/) (via `rustup`)
- [`wasm-pack`](https://rustwasm.github.io/wasm-pack/) for building Wasm packages.
- [`just`](https://github.com/casey/just) as the task runner.
- [`uv`](https://github.com/astral-sh/uv) for Python environment and dependency management (used for tests and benchmarks).
- [`live-server`](https://www.npmjs.com/package/live-server) and [`entr`](https://eradman.com/entrproject/) for the development workflow.

## Building and Running

The project uses `just` to manage common tasks. Run `just` or `just --list` to see all available commands.

- **Development**:
  ```bash
  just dev
  ```
  Starts a development server with hot reload. It watches for file changes, rebuilds the Wasm, and refreshes the browser.

- **Build for Web**:
  ```bash
  just build-web
  ```
  Compiles the Wasm for the browser.

- **Run CI checks**:
  ```bash
  just ci
  ```
  Runs formatting, clippy (linting), and Rust unit tests.

- **Apply fixes**:
  ```bash
  just fix
  ```
  Runs `cargo fmt` and `cargo clippy --fix`.

- **Testing**:
  ```bash
  just test-e2e
  ```
  Runs end-to-end Playwright tests (written in Python).

- **Benchmarking**:
  ```bash
  just benchmark
  ```
  Runs performance benchmarks comparing the current branch against `main`.

## Development Conventions

- **Rust Workspace**: Always work within the workspace structure. Add new simulations as separate crates in `crates/` and register them in `crates/app/src/lib.rs`.
- **Simulation Trait**: New simulations must implement the `Simulation` trait defined in `crates/engine/src/lib.rs`.
- **Debug UI**: Use the `Param` and `DebugUI` types from `crates/debug_ui` to expose tunable parameters to the user.
- **Canvas Rendering**: Use the `Canvas` wrapper from `crates/canvas` for all drawing operations to maintain performance and consistency.
- **E2E Tests**: Significant UI or logic changes should be accompanied by updates to the Playwright tests in `tests/`. **Crucially, new features or simulations should include a video test (using `Shift+R`) that tweaks parameters to provide visual proof of stability and correctness.**
- **Task Runner**: Prefer using `just` commands for common operations (build, test, deploy) to ensure consistency across environments.
