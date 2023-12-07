lint:
    cargo clippy
flamegraph:
    cargo flamegraph --profile flamegraph -o flamegraphs/scanner-api.svg
dhat:
    cargo run --profile dhat --features dhat-heap
prereqs:
    cargo install cross
cross-build:
    CROSS_CONTAINER_OPTS="--platform linux/amd64" cross build --target armv7-unknown-linux-gnueabihf
clean:
    cargo clean
cross-release:
    CROSS_CONTAINER_OPTS="--platform linux/amd64" cross build --release --target armv7-unknown-linux-gnueabihf
format:
    rustfmt --edition 2021 src/*.rs