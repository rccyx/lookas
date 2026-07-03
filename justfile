export RUST_BACKTRACE := "1"
alias c:= clean
alias f:= format
alias l:= clippy  # l for lint
alias b:= build
alias t:= test
alias r:= run
alias fc:= format-check

@clean:
    rm -rf target  dist

@coverage:
    cargo +nightly tarpaulin --verbose --all-features --workspace --timeout 120 --out html


@format:
    cargo fmt --all

@format-check:
    cargo fmt --all -- --check

@clippy:
    cargo clippy --all-targets --all-features --locked -- -D warnings -A incomplete_features -W clippy::dbg_macro -W clippy::print_stdout

@build:
    cargo build --release --all-features --locked

@test:
    cargo test --all-features --workspace --locked

@run:
    cargo run