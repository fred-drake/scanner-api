lint:
    cargo clippy
flamegraph:
    cargo flamegraph --profile flamegraph -o flamegraphs/scanner-api.svg
dhat:
    cargo run --profile dhat --features dhat-heap
prereqs:
    cargo install cross
cross-build:
    CROSS_CONTAINER_OPTS="--platform linux/amd64" cross build --target aarch64-unknown-linux-gnu
clean:
    cargo clean
cross-release:
    CROSS_CONTAINER_OPTS="--platform linux/amd64" cross build --release --target aarch64-unknown-linux-gnu
format:
    rustfmt --edition 2021 src/*.rs