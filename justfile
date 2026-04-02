set dotenv-load
DEV_PARAMS := "?debug&alpha_retention=255&final_speed=5&number_of_ants=1&speedup_frames=0&start_x=0.5&start_y=0.5"
DEPLOY_DIR := "deploy"

# Shows this help
help:
    just --list

# Build langton-ant with wasm-pack for the web
build-web *args:
    cd crates/langton && rm -rf pkg && wasm-pack build --target web --no-typescript {{args}}

# Build langton-ant with wasm-pack for the bundlers
build-pkg *args:
    cd crates/langton && rm -rf pkg && wasm-pack build --target bundler --scope codeberg {{args}}

publish-pkg: build-pkg
    cd crates/langton/pkg && npm publish --userconfig=../.npmrc

# Run langton-ant and watch for changes
dev:
    #!/bin/sh
    killall live-server entr
    git ls-files | entr -c just build-web --dev &
    live-server --hard --open='{{DEV_PARAMS}}' src/langton &

# deploy build-web to `pages` branch
deploy: build-web
    #!/bin/sh
    set -xe
    deploy_msg="$(date --iso-8601=seconds)"
    git commit -am "$deploy_msg" || true

    mkdir -p {{DEPLOY_DIR}}
    cp src/langton/index.html  {{DEPLOY_DIR}}
    cp src/langton/favicon.png {{DEPLOY_DIR}}
    cp -r src/langton/pkg      {{DEPLOY_DIR}}
    rm deploy/pkg/.gitignore
    git switch pages
    git ls-files ':!/.gitignore' -z | xargs -0 rm -f
    mv deploy/* .

    git add .
    git commit --no-verify -m "$deploy_msg"
    git push
    git switch -
