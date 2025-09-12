export DEV_PARAMS := "/?debug&alpha_retention=255&final_speed=5&number_of_ants=1&speedup_frames=0&start_x=0.5&start_y=0.5"
DEPLOY_DIR := "deploy"

# Build langton-ant with wasm-pack
build:
    cd src/langton && wasm-pack build --target web

# Run langton-ant and watch for changes
dev:
    kitty -e fish -c "git ls-files | entr -c just build" &
    kitty -e fish -c "live-server --open={{DEV_PARAMS}} src/langton" &

deploy:# build
    #!/bin/sh
    set -xe
    deploy_id="$(git commit -a --amend --no-edit)"
    git commit -am "$deploy_id"

    mkdir -p {{DEPLOY_DIR}}
    cp src/langton/index.html  {{DEPLOY_DIR}}
    cp src/langton/favicon.png {{DEPLOY_DIR}}
    cp -r src/langton/pkg      {{DEPLOY_DIR}}
    git switch pages
    mv -fT deploy/* .

    git add .
    git commit -m "$deploy_id"
    git push
    git switch -
