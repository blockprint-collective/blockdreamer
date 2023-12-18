FROM rust:bullseye AS builder
ADD . /app/
WORKDIR app
RUN apt-get update; \
    apt-get install -y \
      cmake \
      libclang-dev
RUN cargo build --release

FROM debian:bullseye-slim
COPY --from=builder /app/target/release/blockdreamer /usr/local/bin/
RUN apt-get update; \
    apt-get install -y ca-certificates; \
    update-ca-certificates
WORKDIR /blockdreamer
CMD ["blockdreamer"]
