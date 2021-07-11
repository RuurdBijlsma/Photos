# Dockerfile

FROM node:16-buster-slim
WORKDIR /nodejs
COPY ./nodejs/. /
# Install dependencies via apt
RUN apt-get update && \
    apt-get install -y build-essential \
    wget \
    python3 \
    make \
    gcc \
    libc6-dev \
    ffmpeg
RUN npm install
CMD [ "npm", "start" ]

FROM nginx:stable
COPY ./nginx/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 3333
