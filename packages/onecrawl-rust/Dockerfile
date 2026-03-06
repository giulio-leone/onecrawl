# Stage 1: Build Rust binary
FROM rust:1.82-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .
RUN cargo build --release -p onecrawl-cli-rs && \
    cp target/release/onecrawl /usr/local/bin/onecrawl

# Stage 2: Runtime with Chromium
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    chromium \
    chromium-sandbox \
    fonts-liberation \
    fonts-noto-color-emoji \
    libglib2.0-0 \
    libnss3 \
    libx11-6 \
    libxcb1 \
    libxcomposite1 \
    libxdamage1 \
    libxext6 \
    libxfixes3 \
    libxrandr2 \
    ca-certificates \
    --no-install-recommends && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/onecrawl /usr/local/bin/onecrawl

ENV CHROME_PATH=/usr/bin/chromium
ENV ONECRAWL_HEADLESS=true

EXPOSE 9222 8080

ENTRYPOINT ["onecrawl"]
CMD ["--help"]
