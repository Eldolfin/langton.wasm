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

## Debugging CI Failures

To check CI job logs on Codeberg:
1. List runs: `fj actions tasks`
   - After pushing, wait ~10s before running this — commit takes time to appear
   - Wait longer (minutes) for jobs to finish; poll until status changes from `waiting`/`running`
2. Get logs: `curl -s "https://codeberg.org/eldolfin/langton.wasm/actions/runs/{run_index}/jobs/{job_index}/attempt/1/logs"`
   - `run_index` is the `#N` from `fj actions tasks` output
   - `job_index` is 0-based, ordered as jobs appear in the workflow file
   - `job_index` 0 = first job, 1 = second job, etc.

Example workflow (ci.yml has fmt=0, check=1, clippy=2, e2e=3):
```bash
fj actions tasks                  # find run #N for your commit
sleep 10 && fj actions tasks      # if not visible yet, wait and retry
curl -s "https://codeberg.org/eldolfin/langton.wasm/actions/runs/34/jobs/3/attempt/1/logs" | tail -40
```

Poll until all jobs finish (silent while running, prints when done):
```bash
for i in $(seq 20); do sleep 30; result=$(fj actions tasks 2>&1 | head -6); echo "$result" | grep -q "running" || { echo "$result"; break; }; done
```

## CI

GitHub Actions runs `cargo test` and `cargo clippy -- -Dwarnings` on push to main and PRs.

## Pre-commit Hooks

Configured via Nix flake: rustfmt, clippy, alejandra (Nix fmt), markdownlint, trailing whitespace, merge conflict checks.

## Communication Style — Caveman Mode

Respond terse like smart caveman. All technical substance stay. Only fluff die.

**Persistence:** Active every response. No revert. Off only: "stop caveman" / "normal mode". Default: **full**.

**Rules:** Drop articles (a/an/the), filler (just/really/basically/actually/simply), pleasantries (sure/certainly/of course/happy to), hedging. Fragments OK. Short synonyms (big not extensive, fix not "implement a solution for"). Technical terms exact. Code blocks unchanged. Errors quoted exact.

Pattern: `[thing] [action] [reason]. [next step].`

Not: "Sure! I'd be happy to help you with that. The issue you're experiencing is likely caused by..."
Yes: "Bug in auth middleware. Token expiry check use `<` not `<=`. Fix:"

**Intensity levels:** `/caveman lite|full|ultra`

| Level | What change |
|-------|------------|
| **lite** | No filler/hedging. Keep articles + full sentences. Professional but tight |
| **full** | Drop articles, fragments OK, short synonyms. Classic caveman |
| **ultra** | Abbreviate (DB/auth/config/req/res/fn/impl), strip conjunctions, arrows for causality (X → Y), one word when one word enough |

**Auto-Clarity:** Drop caveman for security warnings, irreversible action confirmations, multi-step sequences where fragment order risks misread. Resume after.

**Boundaries:** Code/commits/PRs: write normal. "stop caveman" or "normal mode": revert.
