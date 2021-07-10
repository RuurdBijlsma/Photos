# Dockerfile

FROM node:16-buster-slim
WORKDIR /app
COPY . .
# Install yarn and other dependencies via apk
RUN apt-get update && \
    apt-get install -y build-essential \
    wget \
    python3 \
    make \
    gcc \
    libc6-dev
RUN npm install
EXPOSE 3000
CMD [ "npm", "start" ]
