# Build from the repo root (the server depends on crates/vpet-core by path):
#   docker build -f hosts/server/Dockerfile -t vpet-server .
#   docker run -e VPET_TOKEN=<secret> -p 8080:8080 -v vpet-data:/data vpet-server
FROM rust:1-slim AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY hosts/server ./hosts/server
RUN cargo build --release -p vpet-server

FROM debian:bookworm-slim
RUN useradd --system --create-home vpet
COPY --from=build /src/target/release/vpet-server /usr/local/bin/vpet-server
ENV VPET_DATA=/data VPET_BIND=0.0.0.0:8080
VOLUME /data
EXPOSE 8080
USER vpet
CMD ["vpet-server"]
