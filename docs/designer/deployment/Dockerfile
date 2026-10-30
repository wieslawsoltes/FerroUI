FROM rust:1.89-bookworm AS build
WORKDIR /source
COPY . .
ENV CARGO_INCREMENTAL=0 CARGO_PROFILE_RELEASE_DEBUG=0
RUN cargo build --locked --release -p ferroui-designer-collaboration-host --features server --bin ferroui-collaboration-server

FROM debian:bookworm-slim
RUN groupadd --gid 10001 ferroui && useradd --uid 10001 --gid 10001 --no-create-home ferroui \
    && mkdir -p /var/lib/ferroui && chown ferroui:ferroui /var/lib/ferroui && chmod 700 /var/lib/ferroui
COPY --from=build /source/target/release/ferroui-collaboration-server /usr/local/bin/ferroui-collaboration-server
USER ferroui
VOLUME /var/lib/ferroui
ENTRYPOINT ["ferroui-collaboration-server"]
CMD ["--storage", "/var/lib/ferroui", "--bind", "127.0.0.1:7491"]
