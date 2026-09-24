# The environment of the native bootstrap (ESP-016): the GNU linker, the C
# library's start files and shared library, and Python. There is no C
# compiler, no Rust toolchain and no stage0. `bootstrap/native.py bootstrap
# --require-no-c-compiler` checks the path, and the step runs with
# `docker run --network none`. CI records the base image's digest with the
# report and builds from exactly that digest (BASE_IMAGE).
ARG BASE_IMAGE=debian:stable-slim
FROM ${BASE_IMAGE}
RUN apt-get update \
 && apt-get install -y --no-install-recommends binutils libc6-dev python3 \
 && rm -rf /var/lib/apt/lists/*
