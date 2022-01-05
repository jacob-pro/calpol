FROM rust:latest AS build
WORKDIR /build/
COPY ./ ./
RUN cargo build --release

FROM ubuntu:latest
RUN apt update && apt-get --assume-yes install libssl-dev libpq-dev ca-certificates
COPY --from=build /build/target/release/calpol /opt/calpol/
WORKDIR /calpol/
ENTRYPOINT ["/opt/calpol/calpol", "-c", "/config/config.toml" "server"]
