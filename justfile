list:
    just --list

format:
    cargo fmt --all

build:
    cargo build --all --all-features

test:
    cargo test --all --all-features
  
clippy:
    cargo clippy --all --all-features

checks:
    just format
    just build
    just clippy
    just test

clean:
  find . -name target -type d -exec rm -r {} +
  just remove-lockfiles

remove-lockfiles:
    find . -name Cargo.lock -type f -exec rm {} +

list-outdated:
    cargo outdated -R -w

update:
    cargo update --manifest-path ./crates/core/Cargo.toml --aggressive
    cargo update --manifest-path ./crates/glow/Cargo.toml --aggressive
    cargo update --manifest-path ./crates/fontdue/Cargo.toml --aggressive
    cargo update --manifest-path ./crates/draw/Cargo.toml --aggressive
    cargo update --manifest-path ./crates/input/Cargo.toml --aggressive
    cargo update --manifest-path ./crates/gui/Cargo.toml --aggressive

example NAME="hello_world":
    cargo run --all-features --example {{NAME}}

publish:
    cargo publish --no-verify --manifest-path ./crates/core/Cargo.toml
    sleep 1
    cargo publish --no-verify --manifest-path ./crates/glow/Cargo.toml
    sleep 1
    cargo publish --no-verify --manifest-path ./crates/fontdue/Cargo.toml
    sleep 1
    cargo publish --no-verify --manifest-path ./crates/draw/Cargo.toml
    sleep 1
    cargo publish --no-verify --manifest-path ./crates/input/Cargo.toml
    sleep 1
    cargo publish --no-verify --manifest-path ./crates/gui/Cargo.toml
    sleep 1
    cargo publish --no-verify --manifest-path ./crates/_/Cargo.toml
