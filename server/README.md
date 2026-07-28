# Photos Backend

Backend for **Ruurd Photos**, a self-hosted Google Photos alternative. Handles the API, media ingestion, classification,
and search.

## Features

* Photo and video ingestion
* ML-based analysis (tagging, embeddings, facial recognition)
* REST API for frontend integration
* Hybrid semantic/text search
* File system watcher for new media

## Prerequisites

* **nasm**: `winget install -e --id NASM.NASM`
* **sqlx**: `cargo install sqlx-cli`
* **Exiftool**: https://exiftool.org/install.html [ubuntu: `sudo apt install libimage-exiftool-perl`]
* **Rust** to compile the backend
* **Postgres** database set up with `pgvector` installed, docker/podman command for this available in `./scripts/start_postgres.ps1`.
* **libheif** Ubuntu: `sudo apt install libheif1 libheif-dev libde265-0 x265`, For Windows, see below

### libheif - Windows

install vcpkg:

```pwsh
cd C:\
mkdir src
cd src
git clone https://github.com/microsoft/vcpkg.git
cd vcpkg
.\bootstrap-vcpkg.bat
setx VCPKG_ROOT "C:\src\vcpkg"
```

Add vcpkg to PATH env variable. (add `C:\src\vcpkg`), then restart terminal.

Install libheif via vcpkg:

```pwsh
vcpkg integrate install
vcpkg version
# not sure if both are needed, maybe only second one
vcpkg install libheif:x64-windows-static-md
vcpkg install libheif:x64-windows-static
```

## Installation

### 1. Clone the repo

```bash
git clone https://github.com/RuurdBijlsma/photos-backend.git
cd photos-backend
```

### 2.

### 2. Set up `ml_analysis` environment

```bash
cd crates/libs/ml_analysis/py_ml
uv sync
```

### 4. Set environment variables (.env file or env variables)

```text
DATABASE_URL=postgres://user:pass@localhost/photos
APP__AUTH__JWT_SECRET=your123secret
```

### 5. Set up database

*Make sure postgres is running and the env variables are set*

To apply the migrations, setting up the database structure:

```bash
sqlx migrate run
```

### 6. (Optional) Configure settings

Edit `config/settings.yaml` to adjust backend settings.

---

## Usage

### 1. Run integration tests

```shell
cargo test -p test_integration -- --nocapture
```

### 2. Clippy

```shell
cargo clippy --no-deps --all-features -- -D clippy::all -D clippy::pedantic -D clippy::nursery
```

### 3. Run the backend crates

There are 3 binaries required for full backend functionality:

1. `crates/apps/api` – Web API
2. `crates/apps/watcher` – Watches media directories and enqueues jobs for created/deleted files
3. `crates/apps/worker` – Processes jobs (generates thumbnails, analyzes metadata, updates database)

Run each crate in a separate terminal:

```bash
cargo run -p api
cargo run -p watcher
cargo run -p worker
```

> Tip: You can run multiple workers simultaneously to speed up ingestion.

### 4. Run the frontend

1. Clone the
   frontend: [https://github.com/RuurdBijlsma/photos-frontend](https://github.com/RuurdBijlsma/photos-frontend)
2. Follow the frontend instructions to run it
3. Access the application

## Troubleshooting

### Dynamic Linking

If you are using the `load-dynamic` feature and encounter library errors:

1. Download the `onnxruntime` library from [GitHub Releases](https://github.com/microsoft/onnxruntime/releases).
2. Set the `ORT_DYLIB_PATH` environment variable:
   ```shell
   # Linux/macOS
   export ORT_DYLIB_PATH="/path/to/libonnxruntime.so"
   # Windows (PowerShell)
   $env:ORT_DYLIB_PATH = "C:/Apps/onnxruntime/lib/onnxruntime.dll"
   ```
