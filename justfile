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
    cargo update --manifest-path ./platform/core/Cargo.toml --aggressive
    cargo update --manifest-path ./backends/glow/Cargo.toml --aggressive
  
publish:
    cargo publish --no-verify --manifest-path ./platform/core/Cargo.toml
    sleep 1
    cargo publish --no-verify --manifest-path ./backends/glow/Cargo.toml
    sleep 1
    cargo publish --no-verify --manifest-path ./platform/_/Cargo.toml
