FROM rust:1.88-slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p opencatalog-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/opencatalog-server /usr/local/bin/
EXPOSE 8080
CMD ["opencatalog-server"]
