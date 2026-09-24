# The environment of the self-hosting bootstrap (ESP-015): a C compiler, its
# C library and Python, and nothing else. There is no Rust toolchain and no
# stage0. The bootstrap runs in it with `docker run --network none`. CI
# records the base image's digest with the report and builds from exactly
# that digest (BASE_IMAGE).
ARG BASE_IMAGE=debian:stable-slim
FROM ${BASE_IMAGE}
ARG CC_PACKAGE=gcc
RUN apt-get update \
 && apt-get install -y --no-install-recommends "${CC_PACKAGE}" libc6-dev python3 \
 && rm -rf /var/lib/apt/lists/*
