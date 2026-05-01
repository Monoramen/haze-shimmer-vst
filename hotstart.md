### автоматический ребилд при изменении
cargo install cargo-watch
cargo watch -x "run --bin shimmer_granular_standalone"


### Сборка VST3 bundle

cargo nih-plug bundle shimmer_granular --release