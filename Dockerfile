FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev cmake make pkgconf

WORKDIR /build
COPY . .
RUN cargo build --release

FROM scratch
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /build/target/release/mcp-server-youtube /
ENTRYPOINT ["/mcp-server-youtube", "--transport", "streamable-http", "--host", "0.0.0.0"]
