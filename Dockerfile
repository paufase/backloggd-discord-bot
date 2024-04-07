FROM gcr.io/distroless/cc-debian12:latest-arm64
COPY target/aarch64-unknown-linux-gnu/release/rust-gha-example /
ENTRYPOINT [ /entrypoint.sh ]