SELF := justfile_directory()
RUST_DIR:= SELF
RUST_MANIFEST := './Cargo.toml'
SHORTSHA := `git rev-parse --short HEAD`
CURRENT_BRANCH := `git rev-parse --abbrev-ref HEAD`
export TARGET := arch()
GON_CONFIG := SELF / 'dist/sign_' + TARGET / 'config.hcl'

# Set the test runner to 'cargo nextest' if not in CI or on Windows
# (Windows ARM64 has an install issue with cargo-nextest at the moment)
TEST_RUNNER := if env_var_or_default("CI", '0') == "1" { 'cargo test' } else { if os() == 'windows' { 'cargo test' } else { 'cargo nextest run' } }
ARG_SEP := if TEST_RUNNER == "cargo nextest run" { '' } else { '--' }

@_default:
    @just --list

@about:
    echo "OS: {{ os() }}"
    echo "Family: {{ os_family() }}"
    echo "Arch: {{ arch() }}"
    echo "Rust: $(rustc --version)"
    echo "Cargo: $(cargo --version)"
    echo "Invocation Dir: {{ invocation_directory() }}"

# Run cargo-audit to scan for vulnerable crates
audit: (_cargo-install 'cargo-audit')
    cargo audit

# Build all documentation
doc: doc-rust

# Build All Rust documentation
doc-rust: _doc-rust-crate 

# # ============================ formatter ============================
# Check if code formatter would make changes
fmt-check: fmt-check-rust 

# Check if code formatter would make changes to the Rust SDK
fmt-check-rust:
    cargo fmt --all --check

# Format all the code
[unix]
fmt:
    fmt-rust
    fmt-nix
    
# Format the Rust code
fmt-rust:
    cargo fmt --all

# Format the nix code
fmt-nix:
    nixfmt -- *.nix
# # ============================ linters ============================
# Run all checks and lints
[unix]
lint:
    lint-rust

# Run all lint checks against the Rust SDK
lint-rust: spell-check fmt-check-rust _lint-rust-crate (_lint-rust-crate RUST_MANIFEST '--features unstable')

# ============================ build & test ============================

# Run basic integration and unit tests for all Rust crates
test: test-rust

# Run basic integration and unit tests for the Rust SDK
test-rust: 
    _test-rust-crate _test-rust-ibkr-crate _test-rust-doc-crate _doc-rust-crate

# Update all third party licenses
update-licenses: (_cargo-install 'cargo-lichking')
    cargo lichking bundle --variant name-only > third_party_licenses.md

# Spell check the entire repo
spell-check: _install-spell-check
    typos


# ============================ run & testing ============================

# Run bin in production environment
run $SEEKINGEDGE_PROFILE="RELEASE" $RUST_BACKTRACE="1":
    clear
    cargo lrun --bin seeking-edge

# Run bin in testing environment
run-testing $SEEKINGEDGE_PROFILE="TESTING":
    clear
    cargo lrun --bin seeking-edge

# Run bin in production environment
profile $SEEKINGEDGE_PROFILE="TESTING":
    clear
    cargo lrun --release --features bevy/trace_chrome --bin seeking-edge
#
# Rust Helpers
#

# Run basic integration and unit tests for a Rust crate
_test-rust-crate MANIFEST=RUST_MANIFEST FEATURES='' $RUSTFLAGS='-A warnings': #TODO: -D warning
    {{ TEST_RUNNER }} --no-default-features --manifest-path {{ MANIFEST }} --test-threads=4
    {{ TEST_RUNNER }} {{ FEATURES }} --manifest-path {{ MANIFEST }} --test-threads=4

# build documentation for a Rust crate
_doc-rust-crate MANIFEST=RUST_MANIFEST $RUSTDOCFLAGS="-A warnings":
    cargo doc --manifest-path {{ MANIFEST }} --no-deps --all-features --document-private-items

# Lint a Rust crate
_lint-rust-crate MANIFEST=RUST_MANIFEST RUST_FEATURES='':
    cargo clippy --no-default-features --manifest-path {{ MANIFEST }} --all-targets -- -D warnings
    cargo clippy --all-features --manifest-path {{ MANIFEST }} --all-targets -- -D warnings
    cargo clippy {{ RUST_FEATURES }} --manifest-path {{ MANIFEST }} --all-targets -- -D warnings

# Run API tests using a mock HTTP server
_test-rust-ibkr-crate MANIFEST=RUST_MANIFEST EXTRA_FEATURES='' $RUSTFLAGS='-A warnings':
    {{ TEST_RUNNER }} --features ibkr_client_test {{ EXTRA_FEATURES }} --manifest-path {{ MANIFEST }} {{ ARG_SEP }} --test-threads=1
    {{ TEST_RUNNER }} --features unstable,api_tests{{ EXTRA_FEATURES }} --manifest-path {{ MANIFEST }} {{ ARG_SEP }} --test-threads=1

# Run documentation tests
_test-doc-crate MANIFEST=RUST_MANIFEST FEATURES='':
    cargo test --doc --manifest-path {{ MANIFEST }} {{ FEATURES }}
    cargo test --doc --manifest-path {{ MANIFEST }} --no-default-features
    cargo test --doc --manifest-path {{ MANIFEST }} --all-features

#
# Small Helpers
#

[unix]
_install-spell-check:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! command -v typos 2>&1 >/dev/null ; then
      cargo install typos-cli --force
    fi

[windows]
_install-spell-check:
    #!powershell.exe
    $ret = Get-Command typos >$null 2>$null

    if (! $?) {
      cargo install typos-cli --force
    }

# List TODO items in current branch only
todos-in-branch:
    git diff --name-only {{ CURRENT_BRANCH}} $(git merge-base {{ CURRENT_BRANCH }} main) | xargs rg -o 'TODO:.*$'

# List 'TODO:' items
todos:
    rg -o 'TODO:.*$' -g '!justfile'

#
# Private/Internal Items
#
_cargo-install +TOOLS:
    cargo install {{ TOOLS }}

nextest := `command -v cargo-nextest >/dev/null && echo true || echo false`

debug_log := '{ RUST_LOG = "warn" ,wgpu = "error" ,bevy_ecs = "error", ibkr_rust_api = "warn", bevy_app = "warn" }'

[unix]
test-single *args:
    #!/usr/bin/env bash
    env = "$debug_log"
    set -e
    clear
    cargo clippy --version
    rustc --version
    cargo test --no-fail-fast --workspace -- {{args}} | bunyan
    # cargo nextest run --test-threads=1 --no-fail-fast --workspace -- {{args}}
    # nix-shell --cores 0 --pure --run 'cargo nextest run --all-features --no-fail-fast --workspace {{args}}'

# ============================ build & test ============================

rust_build_and_test:
    just _rust_build_and_test_single ibkr --features ibkr_client_test

test-ibkr-client *args:
    #!/usr/bin/env bash
    env = "$debug_log"
    set -e
    clear
    cargo clippy --version
    rustc --version
    cargo test --features ibkr_client_test --  {{args}} | bunyan

_rust_build_and_test_single directory *args:
    #!/usr/bin/env bash
    set -e
    clear
    cd {{directory}} && cargo build {{args}}
    env = "$debug_log"
    cd {{directory}} && cargo nextest run {{args}} --no-fail-fast --workspace 
    cargo clippy --version
    rustc --version

# ============================ code generators ============================

codegen:
    clear
    cargo run --package se_types_builder
