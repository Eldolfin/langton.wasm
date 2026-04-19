set dotenv-load := true

DEV_PARAMS := "?debug&alpha_retention=255&final_speed=5&number_of_ants=1&speedup_frames=0&start_x=0.5&start_y=0.5"
DEPLOY_DIR := "deploy"

# Shows this help
help:
    just --list

# Build app with wasm-pack for the web
build-web *args:
    cd crates/app && rm -rf pkg && wasm-pack build --target web --no-typescript {{ args }}

# Build optimised wasm with debug symbols retained (for profiling)
build-web-profiling:
    cd crates/app && rm -rf pkg && wasm-pack build --target web --no-typescript --profile=release-with-debug --no-opt
    ~/.cache/.wasm-pack/wasm-opt-*/bin/wasm-opt -O -g \
        crates/app/pkg/app_bg.wasm \
        -o crates/app/pkg/app_bg.wasm

# Build app with wasm-pack for the bundlers
build-pkg *args:
    cd crates/app && rm -rf pkg && wasm-pack build --target bundler --scope codeberg {{ args }}

publish-pkg: build-pkg
    cd crates/app/pkg && npm publish --userconfig=../.npmrc

# Run interleaved benchmark comparing current branch vs main
benchmark main_ref="main" duration="5" iterations="2":
    #!/usr/bin/env bash
    set -euo pipefail
    rm -rf /tmp/pr-build /tmp/main-build
    # Build current branch (PR)
    just build-web
    mkdir -p /tmp/pr-build
    cp crates/app/index.html /tmp/pr-build/index.html
    cp -r crates/app/pkg /tmp/pr-build/pkg
    # Build main (handles both old crates/langton and new crates/app layouts)
    current=$(git rev-parse HEAD)
    git stash --include-untracked -q || true
    git checkout "origin/{{ main_ref }}" -q
    # Remove untracked crate dirs left from PR build (workspace glob would choke on them)
    git clean -fdx crates/
    just build-web
    mkdir -p /tmp/main-build
    for crate_dir in crates/app crates/langton; do
        if [ -d "$crate_dir/pkg" ]; then
            cp "$crate_dir/index.html" /tmp/main-build/index.html
            cp -r "$crate_dir/pkg" /tmp/main-build/pkg
            break
        fi
    done
    git checkout "$current" -q
    git stash pop -q 2>/dev/null || true
    # Run interleaved benchmark
    uv run --project tests/ python tests/benchmark_interleaved.py \
        --main-build /tmp/main-build \
        --pr-build /tmp/pr-build \
        --duration "{{ duration }}" \
        --iterations "{{ iterations }}" \
        --main-output main-results.json \
        --pr-output pr-results.json

# Run app and watch for changes
dev:
    #!/bin/sh
    killall live-server entr
    git ls-files | entr -c just build-web --dev &
    live-server --hard --open='{{ DEV_PARAMS }}' crates/app &

# Run end-to-end Playwright tests (Python)
test-e2e *args: build-web
    uv run --project tests pytest tests/ -n auto -v {{ args }}

# deploy build-web to `pages` branch
deploy: build-web
    #!/bin/sh
    set -xe
    deploy_msg="$(date --iso-8601=seconds)"
    git commit -am "$deploy_msg" || true

    mkdir -p {{ DEPLOY_DIR }}
    cp crates/app/index.html  {{ DEPLOY_DIR }}
    cp crates/app/favicon.png {{ DEPLOY_DIR }}
    cp -r crates/app/pkg      {{ DEPLOY_DIR }}
    rm deploy/pkg/.gitignore
    git switch pages
    git ls-files ':!/.gitignore' -z | xargs -0 rm -f
    mv deploy/* .

    git add .
    git commit --no-verify -m "$deploy_msg"
    git push
    git switch -

# Build and push the builder image (multi-arch, tagged with content hash + latest)
build-push-build-image:
    #!/usr/bin/env bash
    set -euo pipefail
    HASH=$(cat .github/workflows/Dockerfile mise.toml tests/pyproject.toml tests/uv.lock | sha256sum | cut -c1-16)
    IMAGE=codeberg.org/eldolfin/langton.wasm/build-image
    echo "Building $IMAGE:$HASH"
    docker buildx build --platform linux/amd64,linux/arm64 \
        -t "$IMAGE:$HASH" \
        -t "$IMAGE:latest" \
        -f .github/workflows/Dockerfile --push .
    # Update workflow files to reference the new content hash tag
    sed -i "s|build-image:[a-f0-9]\{16\}|build-image:$HASH|g" \
        .github/workflows/ci.yml \
        .github/workflows/benchmark.yml \
        .github/workflows/pages.yml
    echo "Updated workflow files to use $IMAGE:$HASH"

ci:
    cargo fmt --check
    cargo clippy --verbose -- -Dwarnings
    cargo test --verbose

fix:
    cargo fmt
    cargo clippy --fix --allow-dirty --allow-staged

# Open remote-pr-opener submit page with hot reload
dev-remote-pr-opener:
    #!/usr/bin/sh
    cd dev/remote-pr-opener
    xdg-open localhost:3000/submit &
    export RUST_BACKTRACE=1
    export RUST_LOG=debug
    git ls-files | entr -cr cargo r

# Open remote-pr-opener submit page with docker compose
docker-remote-pr-opener:
    #!/usr/bin/sh
    cd dev/remote-pr-opener
    xdg-open localhost:3000/submit &
    export RUST_BACKTRACE=1
    export RUST_LOG=debug
    docker compose up -d --build
