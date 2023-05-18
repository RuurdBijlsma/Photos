#!/bin/bash
# This script allows you to restore a single volume from a container
# Data in restored in volume with same backupped path
#https://stackoverflow.com/questions/26331651/how-can-i-backup-a-docker-container-with-its-data-volumes
NEW_CONTAINER_NAME=$1

usage() {
  echo "Usage: $0 [container name]"
  exit 1
}

if [ -z $NEW_CONTAINER_NAME ]
then
  echo "Error: missing container name parameter."
  usage
fi

sudo docker run --rm --volumes-from $NEW_CONTAINER_NAME -v $(pwd):/backup busybox tar xvf /backup/backup.tar