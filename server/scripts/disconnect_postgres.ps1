# Configuration
$containerName = "photos-db-dev"
$dbName = "photos"
$dbUser = "photos_user"

# SQL command to kill all connections to the database
$sql = @"
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE datname = '$dbName'
  AND pid <> pg_backend_pid();
"@

# Execute the SQL command inside the container
podman exec -i $containerName psql -U $dbUser -d $dbName -c "$sql"
