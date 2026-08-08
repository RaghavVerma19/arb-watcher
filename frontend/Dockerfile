# syntax=docker/dockerfile:1
FROM node:20-alpine AS builder
WORKDIR /app

RUN npm install -g npm@10

COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci --production=false

COPY frontend/ ./
RUN npm run build

FROM nginx:1.27-alpine AS runtime
RUN rm -rf /usr/share/nginx/html/* && \
    mkdir -p /usr/share/nginx/html && \
    addgroup -S nginx-app && adduser -S nginx-app -G nginx-app && \
    chown -R nginx-app:nginx-app /usr/share/nginx/html

USER nginx-app
COPY --from=builder --chown=nginx-app:nginx-app /app/dist /usr/share/nginx/html

EXPOSE 8080
CMD ["nginx", "-g", "daemon off;"]
