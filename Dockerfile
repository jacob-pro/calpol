FROM rust:latest AS build
WORKDIR /build/
COPY ./ ./
RUN cargo build --release

FROM ubuntu:20.04
RUN apt update && apt install -y libssl-dev libpq-dev ca-certificates
COPY --from=build /build/target/release/calpol /opt/calpol/
WORKDIR /opt/calpol/
ENTRYPOINT ["/opt/calpol/calpol"]
CMD ["-c", "/etc/calpol/config.toml", "server"]
