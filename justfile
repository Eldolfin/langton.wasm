dev:
    kitty -e fish -c "git ls-files | entr -c fish -c 'cd src/langton && wasm-pack build --target web'" &
    kitty -e fish -c "live-server src/langton" &
