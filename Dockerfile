FROM arm64v8/debian:latest
COPY target/aarch64-unknown-linux-gnu/release/backloggd-discord-bot /
ENTRYPOINT [ /backloggd-discord-bot ]