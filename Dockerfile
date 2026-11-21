# Base image indexes pinned after build/runtime validation; refresh during release QA.
# Stage 1: Build frontend
FROM node:22-alpine@sha256:1314fb30ec37592d5845e5a160d522c0e680972aa534549940b8d73bde88481a AS frontend
WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

# Stage 2: Build Rust backend
FROM rust:1-bookworm@sha256:ade39a710ef85f3506cffcfec3e8a24ba090badef1dff2fb798f71d878833fea AS backend
ARG CARGO_BUILD_JOBS=2
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
# Compile dependencies in a layer invalidated only by the manifests. UI/source
# changes then rebuild the application crate while reusing those dependencies.
RUN mkdir src && printf 'fn main() {}\n' > src/main.rs \
    && cargo build --release --locked -j "$CARGO_BUILD_JOBS" \
    && rm -rf src target/release/rustdesk-address-book target/release/deps/rustdesk_address_book*
COPY src/ src/
COPY migrations/ migrations/
COPY --from=frontend /app/web/dist/ web/dist/
ARG RUSTBOOK_BUILD_COMMIT=unknown
ENV RUSTBOOK_BUILD_COMMIT=$RUSTBOOK_BUILD_COMMIT
RUN rm -rf target/release/.fingerprint/rustdesk-address-book-* \
    && touch src/main.rs \
    && cargo build --release --locked -j "$CARGO_BUILD_JOBS"

# Stage 3: Minimal runtime
FROM debian:bookworm-slim@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc
ARG RUSTBOOK_BUILD_COMMIT=unknown
LABEL org.opencontainers.image.source="https://github.com/mk-tecnologia/RUSTBOOK" \
      org.opencontainers.image.revision=$RUSTBOOK_BUILD_COMMIT
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=backend /app/target/release/rustdesk-address-book .
RUN groupadd --gid 10001 rustdesk && useradd --uid 10001 --gid rustdesk --no-create-home rustdesk \
    && mkdir /data && chown rustdesk:rustdesk /data
USER 10001:10001
EXPOSE 21114
ENV RUSTDESK_AB_DB_PATH=/data/db.sqlite3
VOLUME /data
CMD ["./rustdesk-address-book"]
