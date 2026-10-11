# codmin Dockerfile.
#
# Build context: the arkret umbrella directory (parent of codmin).
# The Cargo workspace at codmin/Cargo.toml has path-deps on sibling
# repos that must be present in the build sandbox:
#
#   - ../arkret-rust-sdk/crates/...      (Arkret protocol types)
#   - ../coauth/crates/{account,admin}-types
#   - ../coland/crates/contracts
#
# Standalone single-repo build (the codmin docker.yml workflow) checks
# out coauth, coland and arkret-rust-sdk as siblings and invokes:
#   docker build -f codmin/Dockerfile ..
#
# The example-stack compose file does the equivalent via
#   build:
#     context: ../..
#     dockerfile: codmin/Dockerfile

FROM --platform=$BUILDPLATFORM rust:bookworm AS builder

ENV CARGO_HTTP_TIMEOUT=600
ENV CARGO_HTTP_MULTIPLEXING=false
ENV CARGO_NET_RETRY=10
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

ENV RUSTUP_HTTP_TIMEOUT=600
# Install wasm target (with retries for flaky networks)
RUN --network=default \
    for i in 1 2 3 4 5; do \
        if rustup target add wasm32-unknown-unknown; then exit 0; fi; \
        sleep 10; \
    done; exit 1
# Build tools and Cargo metadata decoding.
RUN apt-get update && apt-get install -y binaryen python3 && rm -rf /var/lib/apt/lists/*

RUN --mount=type=cache,id=codmin-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=codmin-cargo-git,target=/usr/local/cargo/git \
    cargo install dioxus-cli@0.7.10 --locked

WORKDIR /workspace

# Copy the sibling path-deps FIRST so cargo fetch / dx build can resolve
# the workspace `path = "../<sibling>/..."` entries. Order matches the
# umbrella layout: arkret/{arkret-rust-sdk,coauth,coland,codmin}/.
COPY arkret-rust-sdk/ /workspace/arkret-rust-sdk
COPY coauth/ /workspace/coauth
COPY coland/ /workspace/coland
COPY codmin/ /workspace/codmin

WORKDIR /workspace/codmin

# Public git dependencies are fetched without build credentials.
# Pre-fetch deps so source-only changes don't redownload the world.
RUN --mount=type=cache,id=codmin-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=codmin-cargo-git,target=/usr/local/cargo/git \
    cargo fetch --locked

RUN --network=default \
    --mount=type=cache,id=codmin-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=codmin-cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=codmin-target,target=/workspace/codmin/target \
    built=false; \
    for i in 1 2 3; do \
        if dx build --release --debug-symbols false; then built=true; break; fi; \
        echo "Retry $i..."; sleep 10; \
    done; \
    test "$built" = true && \
    target_dir="$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')" && \
    dist_dir="$(find "$target_dir/dx" -type d -path '*/release/web/public' | head -n 1)" && \
    test -f "$dist_dir/index.html" && \
    cp -r "$dist_dir" /workspace/dist

FROM nginx:alpine

COPY --from=builder /workspace/dist /usr/share/nginx/html
COPY --chmod=755 codmin/docker-entrypoint.sh /docker-entrypoint.sh
RUN mkdir -p /runtime && \
    chown -R 101:101 /usr/share/nginx/html /etc/nginx/conf.d /runtime

# Runtime configuration. COLAND_URL is the coland Station (admin
# + reducer surface) the proxy should forward to; COAUTH_URL is the
# coauth admin service the proxy forwards to (with bearer); COAUTH_PUBLIC_URL
# is the browser-facing coauth origin used for OAuth2 redirects.
#
ENV COLAND_URL=""
ENV COAUTH_URL=""
ENV COAUTH_PUBLIC_URL=""
ENV CODMIN_PORT="80"

EXPOSE 80
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD wget -q --spider "http://127.0.0.1:${CODMIN_PORT:-80}/healthz" || exit 1
CMD ["/docker-entrypoint.sh"]
