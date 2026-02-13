FROM rust:1.93.0-alpine3.23 AS app-build

WORKDIR /build

RUN apk --no-cache add musl-dev elfutils pkgconfig libressl-dev perl make mold upx

COPY . /build

RUN cargo build --release && \
    eu-elfcompress target/release/cronbird && \
    strip target/release/cronbird && \
    upx -9 --lzma target/release/cronbird && \
    chmod +x target/release/cronbird

FROM alpine:3.23.3

WORKDIR /app

RUN adduser -u 1001 -h /app -D cronbird && \
    chmod 700 /app && \
    chown -R cronbird: /app && \
    mkdir -p /data && \
    chown cronbird:cronbird /data && \
    chmod 755 /data

COPY --from=app-build /build/target/release/cronbird /app/cronbird

RUN chown -R cronbird: /app && chmod +x /app/cronbird

USER cronbird

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

CMD ["/app/cronbird"]
