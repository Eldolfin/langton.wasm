dev:
    kitty -e fish -c "git ls-files | entr -c fish -c 'cd src/langton && wasm-pack build --target web'" &
    kitty -e fish -c "live-server --open='/?debug&alpha_retention=255&final_speed=5&number_of_ants=1&speedup_frames=0&start_x=0.5&start_y=0.5' src/langton" &
