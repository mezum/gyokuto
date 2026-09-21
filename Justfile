set windows-shell := ["pwsh", "-NoLogo", "-NoProfile", "-Command"]

[doc("Build all with release mode")]
build:
    cargo build

[doc("Check with fmt and clippy")]
lint:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings

[doc("Run all test")]
test:
    cargo test

[doc("Clean all build artifacts")]
clean:
    cargo clean
