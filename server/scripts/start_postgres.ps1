# =============================================================================
#  PowerShell Script to run a PostgreSQL + pgvector server using podman
# =============================================================================

# --- Configuration ---
# You can change these values to suit your project.
$containerName = "photos-db-dev"
$dbName = "photos"
$dbUser = "photos_user"
$dbPassword = "dev-password"
$dbPort = 5432
$volumeName = "photos_pgdata_dev"
$postgresImage = "docker.io/pgvector/pgvector:pg18"

# =============================================================================

# --- Main Logic ---

Write-Host "Checking for existing container '$containerName'..." -ForegroundColor Yellow

# Find a container with the specified name, regardless of its status (running or stopped)
$existingContainer = podman ps -a --filter "name=$containerName" --format "{{.Names}}"

if ($existingContainer -eq $containerName) {
    # Container exists, check if it's running
    $containerStatus = podman ps --filter "name=$containerName" --format "{{.State}}"

    if ($containerStatus -eq "running") {
        Write-Host "Container '$containerName' is already running." -ForegroundColor Green
    } else {
        Write-Host "Container '$containerName' exists but is stopped. Starting it now..." -ForegroundColor Cyan
        podman start $containerName
    }
} else {
    # Container does not exist, so create it
    Write-Host "Container '$containerName' not found. Creating and starting a new one..." -ForegroundColor Cyan

    # The `podman run` command:
    # -d                  : Run in detached mode (in the background)
    # --name              : Assign a name to the container for easy management
    # -e POSTGRES_...     : Set environment variables to configure the database
    # -p                  : Map the host port to the container port (host:container)
    # -v                  : Mount a volume to persist database data
    # --restart always    : Optional: automatically restart the container if it crashes
    podman run `
        -d `
        --name $containerName `
        -e POSTGRES_DB=$dbName `
        -e POSTGRES_USER=$dbUser `
        -e POSTGRES_PASSWORD=$dbPassword `
        -p "${dbPort}:5432" `
        -v "${volumeName}:/var/lib/postgresql" `
        --restart always `
        $postgresImage

    Write-Host "Waiting a few seconds for the database to initialize..."
    Start-Sleep -Seconds 5
}

# --- Output Connection Info ---
Write-Host "--------------------------------------------------------" -ForegroundColor Green
Write-Host "PostgreSQL with pgvector is now running!" -ForegroundColor Green
Write-Host "You can connect to it using the following details:"
Write-Host "  Host: 127.0.0.1" -ForegroundColor Green
Write-Host "  Port: $dbPort"
Write-Host "  Database: $dbName"
Write-Host "  User: $dbUser"
Write-Host "  Password: $dbPassword"
Write-Host ""
Write-Host "Connection String:"
Write-Host "  postgresql://$dbUser`:$dbPassword@127.0.0.1:$dbPort/$dbName"
Write-Host "--------------------------------------------------------"