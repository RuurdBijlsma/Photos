# Updates the sqlx offline query data for the workspace.
# Assumes 'sqlx-cli' is installed and DATABASE_URL is set.

Push-Location $(Resolve-Path (Join-Path $PSScriptRoot ".."))
Remove-Item -Path ".sqlx" -Recurse -Force -ErrorAction SilentlyContinue

cargo sqlx prepare --workspace

# Check the result and provide feedback.
if ($LASTEXITCODE -eq 0) {
    Write-Host "Successfully updated .sqlx query data." -ForegroundColor Green
} else {
    Write-Host "Error: Failed to update .sqlx query data. Ensure the database is running." -ForegroundColor Red
}

Pop-Location