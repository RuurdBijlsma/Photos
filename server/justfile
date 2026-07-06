set windows-shell := ["powershell.exe", "-Command"]
set export := true
ORT_DYLIB_PATH := "C:/Apps/onnxruntime/lib/onnxruntime.dll"
DATABASE_URL := "postgresql://photos_user:dev-password@localhost:5432/photos"

# --- Lints:

check: fmt clippy test

fmt:
    cargo fmt --all

clippy:
    cargo clippy --no-deps --all-features --tests --benches -- \
        -D clippy::all \
        -D clippy::pedantic \
        -D clippy::nursery

# --- Misc:

clean:
    cargo clean

setup:
    script/start_postgres.ps1

# --- Execution:

test:
    cargo test --profile release --features load-dynamic -- --nocapture

bench:
    cargo bench --bench system_bench --features load-dynamic
run:
    cargo run --example visualize --profile release --features load-dynamic

