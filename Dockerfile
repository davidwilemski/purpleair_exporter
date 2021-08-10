FROM rust:1.54 AS builder

RUN mkdir -p /opt/purpleair_exporter
WORKDIR /opt/purpleair_exporter

COPY . ./

RUN cargo build --release

FROM debian:stable

# We need libssl available for hyper servers even if we are not utilizing SSL
RUN apt update && apt install -y openssl ca-certificates

RUN mkdir -p /opt/purpleair_exporter/bin
COPY --from=builder /opt/purpleair_exporter/target/release/purpleair_exporter /opt/purpleair_exporter/bin/purpleair_exporter

EXPOSE 3000
ENTRYPOINT /opt/purpleair_exporter/bin/purpleair_exporter
