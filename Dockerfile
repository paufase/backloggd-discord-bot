FROM gcr.io/distroless/cc-debian12:latest-arm64
COPY target/aarch64-unknown-linux-gnu/release/backloggd-discord-bot /
ENTRYPOINT [ /entrypoint.sh ]