# Dockerfile

FROM node:buster-slim
WORKDIR /nodejs
COPY . .

# Install dependencies via apt
RUN apt-get update && \
    apt-get install -y build-essential \
    wget \
    python3 \
    make \
    gcc \
    libc6-dev \
    ffmpeg \
    lsb-release

# Add postgres-13 repo for pg_dump and pg_restore
RUN wget --quiet -O - https://www.postgresql.org/media/keys/ACCC4CF8.asc | apt-key add - && \
    echo "deb http://apt.postgresql.org/pub/repos/apt/ `lsb_release -cs`-pgdg main" | tee  /etc/apt/sources.list.d/pgdg.list

RUN apt-get update && \
    apt-get install -y postgresql-client-13

RUN npm install
EXPOSE 3000
CMD [ "npm", "start" ]
