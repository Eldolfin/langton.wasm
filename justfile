export DEV_PARAMS := "/?debug&alpha_retention=255&final_speed=5&number_of_ants=1&speedup_frames=0&start_x=0.5&start_y=0.5"

# Build langton-ant with wasm-pack
build:
    cd src/langton && wasm-pack build --target web

# Run langton-ant and watch for changes
dev:
    kitty -e fish -c "git ls-files | entr -c just build" &
    kitty -e fish -c "live-server --open={{DEV_PARAMS}} src/langton" &
