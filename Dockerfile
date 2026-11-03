# syntax=docker/dockerfile:1.10

FROM rust:1.95-trixie AS builder

ARG TARGETARCH
ENV Z3_SYS_Z3_VERSION=4.16.0
ENV CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="-C link-arg=--strip-debug"

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
        clang cmake build-essential pkg-config libssl-dev \
        curl ca-certificates \
 && rm -rf /var/lib/apt/lists/*

RUN case "$TARGETARCH" in \
        arm64) tw=tailwindcss-linux-arm64 ;; \
        *) tw=tailwindcss-linux-x64 ;; \
    esac \
 && curl -fsSL -o /usr/local/bin/tailwindcss \
        "https://github.com/tailwindlabs/tailwindcss/releases/download/v4.3.3/$tw" \
 && chmod +x /usr/local/bin/tailwindcss

RUN curl -L --proto '=https' --tlsv1.2 -sSf \
        https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh \
        | bash \
 && cargo binstall -y --force dioxus-cli@0.7.10

WORKDIR /build
COPY . ./

RUN tailwindcss -i frontend/src/input.css -o frontend/src/assets/tailwind.css --minify
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    --mount=type=secret,id=github_token \
    if [ -s /run/secrets/github_token ]; then \
        export READ_ONLY_GITHUB_TOKEN="$(cat /run/secrets/github_token)"; \
    fi \
 && dx bundle --release --platform web --package frontend --out-dir /out

FROM gcr.io/distroless/cc-debian13 AS runtime

WORKDIR /app
COPY --from=builder /out/ ./

ENV PORT=8080
ENV IP=0.0.0.0
EXPOSE 8080

CMD ["./server"]
