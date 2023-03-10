FROM rust:latest as builder

WORKDIR /usr/src/clash-server

# NASTY HACK: Our build script relies on a proper git repo state to generate version numbers.
COPY ./.git ./.git/
RUN git restore .

RUN cargo install --path ./crates/clash-server

# ---------------------------------------------------------------------------------------------

FROM debian:buster-slim
COPY --from=builder /usr/local/cargo/bin/clash-server /usr/local/bin/clash-server
CMD ["clash-server"]