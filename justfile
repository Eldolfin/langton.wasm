export DEV_PARAMS := "/?debug&alpha_retention=255&final_speed=5&number_of_ants=1&speedup_frames=0&start_x=0.5&start_y=0.5"
DEPLOY_DIR := "deploy"

# Shows this help
help:
    just --list

# Build langton-ant with wasm-pack
build:
    cd src/langton && wasm-pack build --target web

# Run langton-ant and watch for changes
dev:
    kitty -e fish -c "git ls-files | entr -c just build" &
    kitty -e fish -c "live-server --open={{DEV_PARAMS}} src/langton" &

# deploy build to `pages` branch
deploy: build
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
