# ─── Build Stage ───
FROM rust:1.82-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY packages/onecrawl-rust/ ./

# Build release binaries
RUN cargo build --release --bin onecrawl && \
    cargo build --release --bin onecrawl-mcp

# ─── Runtime Stage ───
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    chromium \
    ca-certificates \
    fonts-liberation \
    libasound2 \
    libatk-bridge2.0-0 \
    libatk1.0-0 \
    libcups2 \
    libdbus-1-3 \
    libdrm2 \
    libgbm1 \
    libgtk-3-0 \
    libnss3 \
    libx11-xcb1 \
    libxcomposite1 \
    libxdamage1 \
    libxrandr2 \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

# Copy binaries
COPY --from=builder /app/target/release/onecrawl /usr/local/bin/onecrawl
COPY --from=builder /app/target/release/onecrawl-mcp /usr/local/bin/onecrawl-mcp

# Set Chrome path for headless mode
ENV CHROME_PATH=/usr/bin/chromium
ENV ONECRAWL_HEADLESS=true

# MCP server port
EXPOSE 3100

# Default: run MCP server
CMD ["onecrawl-mcp"]
