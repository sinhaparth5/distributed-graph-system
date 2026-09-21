# ── Stage 1: build WASM (Rust → src/wasm-pkg) ─────────────────────────────────
FROM rust:1-slim AS wasm-builder

RUN apt-get update && \
    apt-get install -y --no-install-recommends curl ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Prebuilt wasm-pack binary (installs wasm32 target on first build)
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

WORKDIR /build
COPY wasm ./wasm
# out-dir is relative to the crate root → /build/src/wasm-pkg
RUN wasm-pack build ./wasm --target bundler --out-dir ../src/wasm-pkg

# ── Stage 2: build frontend ────────────────────────────────────────────────────
FROM node:24-alpine AS builder

# Enable corepack for pnpm
RUN corepack enable && corepack prepare pnpm@latest --activate

WORKDIR /app

# Install dependencies first (layer cache)
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
RUN pnpm install --frozen-lockfile

# Copy source and the pre-built WASM package
COPY . .
COPY --from=wasm-builder /build/src/wasm-pkg ./src/wasm-pkg

# Accept API base URL as build arg (default: localhost for dev)
ARG VITE_API_BASE=http://localhost:8000
ENV VITE_API_BASE=$VITE_API_BASE

# build:app only — the WASM was already built in the Rust stage
RUN pnpm run build:app

# ── Stage 3: serve ─────────────────────────────────────────────────────────────
FROM nginx:1.31-alpine

COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
