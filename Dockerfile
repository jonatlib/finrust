# ==============================================================================
# 0. BASE STAGE (Shared Tools)
# ==============================================================================
FROM rust:1.91-slim-bookworm AS base
WORKDIR /app
RUN apt-get update && apt-get install -y build-essential pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef trunk
RUN rustup target add wasm32-unknown-unknown

# ==============================================================================
# 1. PLANNER STAGE
# ==============================================================================
FROM base AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ==============================================================================
# 2. BUILDER STAGE (Cache Dependencies)
# ==============================================================================
FROM base AS builder
COPY --from=planner /app/recipe.json recipe.json
# Cook Backend (Linux)
RUN cargo chef cook --release --recipe-path recipe.json
# Cook Frontend (WASM) -p frontend
RUN cargo chef cook --release --target wasm32-unknown-unknown --recipe-path recipe.json -p frontend

# ==============================================================================
# 3. FRONTEND BUILDER (Compile Assets)
# ==============================================================================
FROM builder AS frontend-builder
COPY . .
WORKDIR /app/workspace/frontend
RUN trunk build --release --public-url /

# ==============================================================================
# 4. BACKEND BUILDER (Compile Binary)
# ==============================================================================
FROM builder AS backend-builder
COPY . .
RUN cargo build --release --bin finrust

# ==============================================================================
# 5. FINAL STAGE: BACKEND (Rust Binary)
# ==============================================================================
FROM debian:bookworm-slim AS backend
WORKDIR /app
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=backend-builder /app/target/release/finrust /app/finrust
ENV HOST=0.0.0.0
ENV PORT=8080
EXPOSE 8080
CMD ["/app/finrust"]

# ==============================================================================
# 6. FINAL STAGE: FRONTEND (Nginx)
# ==============================================================================
FROM nginx:alpine AS frontend

# 1. Copy the built assets
COPY --from=frontend-builder /app/workspace/frontend/dist /usr/share/nginx/html

# 2. Generate Nginx Config with LAZY RESOLUTION
#    - resolver 127.0.0.11: Uses Docker's internal DNS
#    - set $backend_upstream: Forces Nginx to resolve the name at request time, not boot time
RUN printf 'server {\n\
    listen 80;\n\
    server_name localhost;\n\
    root /usr/share/nginx/html;\n\
    index index.html;\n\
    \n\
    # Use Docker internal DNS resolver\n\
    resolver 127.0.0.11 valid=30s;\n\
    \n\
    location / {\n\
        try_files $uri $uri/ /index.html;\n\
    }\n\
    \n\
    location /api/ {\n\
        # Using a variable forces dynamic resolution\n\
        set $backend_upstream "http://backend:8080";\n\
        proxy_pass $backend_upstream;\n\
        \n\
        proxy_http_version 1.1;\n\
        proxy_set_header Upgrade $http_upgrade;\n\
        proxy_set_header Connection "upgrade";\n\
        proxy_set_header Host $host;\n\
        proxy_cache_bypass $http_upgrade;\n\
    }\n\
}' > /etc/nginx/conf.d/default.conf

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
