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
EXPOSE 3000
CMD [ "npm", "start" ]
