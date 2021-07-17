# Dockerfile
FROM node:buster-slim as build-stage
WORKDIR /frontend
COPY ./frontend/. .
COPY ./env/. ./src/env/.
RUN npm install
RUN npm run build

FROM nginx:stable
COPY --from=build-stage /frontend/dist/ /usr/share/nginx/html
EXPOSE 3333 80
