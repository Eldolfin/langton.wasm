set dotenv-load := true

DEV_PARAMS := "?debug&alpha_retention=255&final_speed=5&number_of_ants=1&speedup_frames=0&start_x=0.5&start_y=0.5"
DEPLOY_DIR := "deploy"

# Shows this help
help:
    just --list

# Build langton-ant with wasm-pack for the web
build-web *args:
    cd crates/langton && rm -rf pkg && wasm-pack build --target web --no-typescript {{ args }}

# Build langton-ant with wasm-pack for the bundlers
build-pkg *args:
    cd crates/langton && rm -rf pkg && wasm-pack build --target bundler --scope codeberg {{ args }}

publish-pkg: build-pkg
    cd crates/langton/pkg && npm publish --userconfig=../.npmrc

# Run interleaved benchmark comparing current branch vs main
benchmark main_ref="main" duration="5" iterations="2":
    #!/usr/bin/env bash
    set -euo pipefail
    # Build current branch
    just build-web
    cp -r crates/langton/pkg /tmp/pr-pkg
    # Build main
    current=$(git rev-parse HEAD)
    git stash --include-untracked -q || true
    git checkout "origin/{{main_ref}}" -q
    just build-web
    cp -r crates/langton/pkg /tmp/main-pkg
    git checkout "$current" -q
    git stash pop -q 2>/dev/null || true
    # Run interleaved benchmark
    uv run --project tests/ python tests/benchmark_interleaved.py \
        --main-pkg /tmp/main-pkg \
        --pr-pkg /tmp/pr-pkg \
        --duration "{{duration}}" \
        --iterations "{{iterations}}" \
        --main-output main-results.json \
        --pr-output pr-results.json

# Run langton-ant and watch for changes
dev:
    #!/bin/sh
    killall live-server entr
    git ls-files | entr -c just build-web --dev &
    live-server --hard --open='{{ DEV_PARAMS }}' crates/langton &

# Run end-to-end Playwright tests (Python)
test-e2e *args: build-pkg
    uv run --project tests pytest tests/ -n auto -v {{ args }}

# deploy build-web to `pages` branch
deploy: build-web
    #!/bin/sh
    set -xe
    deploy_msg="$(date --iso-8601=seconds)"
    git commit -am "$deploy_msg" || true

    mkdir -p {{ DEPLOY_DIR }}
    cp src/langton/index.html  {{ DEPLOY_DIR }}
    cp src/langton/favicon.png {{ DEPLOY_DIR }}
    cp -r src/langton/pkg      {{ DEPLOY_DIR }}
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
