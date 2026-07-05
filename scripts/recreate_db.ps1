Push-Location (Join-Path $PSScriptRoot "..")

./scripts/disconnect_postgres.ps1

sqlx database drop -y
if ($LASTEXITCODE -ne 0) { throw "sqlx database drop failed" }
sqlx database create
if ($LASTEXITCODE -ne 0) { throw "sqlx database create failed" }
sqlx migrate run
if ($LASTEXITCODE -ne 0) { throw "sqlx migrate run failed" }

$env:ORT_DYLIB_PATH="C:/Apps/onnxruntime/lib/onnxruntime.dll"
cargo run --package common_services --example create_users --profile release --features load-dynamic

Pop-Location