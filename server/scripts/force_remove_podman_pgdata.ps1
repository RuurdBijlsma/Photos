$volume = "photos-backend_pg_data"

# Find containers using the volume
$containers = podman ps -a --filter volume=$volume --format "{{.ID}}"

if ($containers) {
    Write-Host "Stopping containers using volume '$volume'..."
    foreach ($c in $containers) {

        # Stop container (no --force on Podman stop)
        podman stop $c

        # Remove container (force allowed)
        podman rm -f $c
    }
} else {
    Write-Host "No containers using volume '$volume'."
}

# Remove the volume
Write-Host "Removing volume '$volume'..."
podman volume rm $volume

Write-Host "Done."
