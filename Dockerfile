#
# Docker Image build script
#
# Based on:
# https://github.com/emk/rust-musl-builder/blob/master/examples/using-diesel/Dockerfile
#

# You can override this `--build-arg BASE_IMAGE=...` to use different
# version of Rust or OpenSSL.
ARG BASE_IMAGE=ekidd/rust-musl-builder:stable

# Our first FROM statement declares the build environment.
FROM ${BASE_IMAGE} AS builder

# Add our source code.
ADD --chown=rust:rust . ./

# Build our application.
RUN cargo build --release

# Now, we need to build our _real_ Docker container, copying in our binary.
FROM scratch

COPY --from=builder \
    /home/rust/src/target/x86_64-unknown-linux-musl/release/nanocron \
    /

CMD ["/nanocron"]
