# Install

## 1. Create a directory for the application

```bash
mkdir photos-app && cd photos-app
```

## 2. Download the docker-compose.yml and example.env

```bash
wget https://github.com/ruurdbijlsma/Photos/releases/latest/download/compose.yml
wget -O .env https://github.com/ruurdbijlsma/Photos/releases/latest/download/example.env
```

## 3. Edit .env to set your MEDIA_LOCATION and JWT_SECRET, then run:

docker compose up -d