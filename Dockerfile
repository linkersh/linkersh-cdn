
ARG RUST_VERSION=1.81.0
ARG APP_NAME=linkersh-panel

FROM rust:${RUST_VERSION}-slim-bullseye AS build
ARG APP_NAME

WORKDIR /app

RUN apt-get update -y
RUN apt-get install build-essential libssl-dev pkg-config -y

RUN --mount=type=bind,source=./src,target=/app/src,rw \
    --mount=type=bind,source=./Cargo.toml,target=/app/Cargo.toml \
    --mount=type=bind,source=./Cargo.lock,target=/app/Cargo.lock \
    --mount=type=bind,source=./.env,target=/app/.env \
    --mount=type=cache,target=/app/target/,id=rust-cache-${APP_NAME}-${TARGETPLATFORM} \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    <<EOF
set -e
cargo build --locked --release --target-dir ./target
cp ./target/release/$APP_NAME /bin/linkersh-panel
EOF

FROM debian:bullseye-slim AS final

RUN apt-get update -y && \
    apt-get install ca-certificates -y && \
    apt-get clean

RUN mkdir /ocr

ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser
USER appuser

COPY --from=build /bin/linkersh-panel /bin/
COPY ocr/text-detection.rten /ocr/text-detection.rten
COPY ocr/text-recognition.rten /ocr/text-recognition.rten

EXPOSE 6601

CMD ["/bin/linkersh-panel"]