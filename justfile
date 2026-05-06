export RUST_BACKTRACE := "1"
alias c:= clean
alias f:= format
alias l:= clippy  # l for lint
alias b:= build
alias t:= test

@clean:
    rm -rf target  dist

@coverage:
    cargo +nightly tarpaulin --verbose --all-features --workspace --timeout 120 --out html


@format:
     cargo fmt

@build:
    cargo build --release

@clippy:
    cargo clippy --all-targets --all-features -- \
    -D warnings \
    -D clippy::pedantic \
    -D clippy::nursery \
    -D clippy::cargo \
    -W clippy::unwrap_used \
    -W clippy::expect_used \
    -W clippy::todo \
    -A incomplete_features
@test:
    cargo test