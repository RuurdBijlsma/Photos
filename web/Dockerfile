# ==========================================
# 1. Build Stage
# ==========================================
FROM node:22-alpine AS builder
WORKDIR /app

# Cache node_modules using lockfile
COPY package.json package-lock.json ./
RUN npm ci

# Copy web source and build
COPY . .
RUN npm run build

# ==========================================
# 2. Production Web Server
# ==========================================
FROM nginx:alpine-slim AS runtime

# Copy custom Nginx proxy config
COPY nginx.conf /etc/nginx/conf.d/default.conf

# Copy compiled static assets
COPY --from=builder /app/dist /usr/share/nginx/html

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]