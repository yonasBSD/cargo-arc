# cargo-arc

default:
    @just --list

build:
    cargo build

test-rust:
    cargo test

test-js:
    bun test

# Rust + JS
test: test-rust test-js

# clippy + biome + tsc typecheck + format check + cycle detection
lint:
    cargo clippy -- -D warnings
    cargo fmt --check
    bunx biome check js/
    npx tsc --project jsconfig.json
    cargo run -- arc --check

# format Rust + JS
fmt:
    cargo fmt
    bunx biome format --write js/

# auto-fix lint warnings
fix:
    cargo clippy --fix --allow-dirty
    bunx biome check --write js/

diagram:
    cargo run -- arc

install:
    cargo install --path .

# validate, test, commit, and tag a release (bump version + changelog first)
release:
    #!/usr/bin/env bash
    set -euo pipefail
    version=$(cargo metadata --no-deps --format-version=1 | grep -oP '"version":"\K[^"]+')
    tag="v${version}"
    # guard: tag must not exist yet
    if git rev-parse "$tag" >/dev/null 2>&1; then
      echo "❌ Tag $tag already exists. Bump version in Cargo.toml first." >&2; exit 1
    fi
    # guard: changelog must mention this version
    if ! grep -qF "[${version}]" CHANGELOG.md; then
      echo "❌ CHANGELOG.md has no entry for [${version}]. Add one first." >&2; exit 1
    fi
    echo "Releasing ${tag}..."
    just lint
    just test
    cargo package --allow-dirty
    git add -A
    git commit -m "Release ${tag}"
    git tag "$tag"
    echo ""
    echo "✅ Tagged ${tag}. Next steps:"
    echo "   git push origin development --tags"
    echo "   cargo publish"

clean:
    cargo clean
