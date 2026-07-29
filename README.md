# Install

## 1. Create a directory for the application

mkdir photos-app && cd photos-app

## 2. Download the docker-compose.yml and example.env

wget https://raw.githubusercontent.com/ruurdbijlsma/Photos/main/cloud-compose/compose.yml
wget -O .env https://raw.githubusercontent.com/ruurdbijlsma/Photos/main/example.env

## 3. Edit .env to set your MEDIA_LOCATION and JWT_SECRET, then run:

docker compose up -d