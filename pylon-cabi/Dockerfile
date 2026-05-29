# syntax=docker/dockerfile:1.7
#
# Reproducible build of pylon-cabi.so for linux-x86_64.
#
# The Basler pylon SDK is installed from a pinned .deb.
#
# Build (run from the repo root):
#   docker buildx build \
#       --file pylon-cabi/Dockerfile \
#       --output type=local,dest=pylon-cabi/out \
#       .
#
# Result: pylon-cabi/out/pylon-cabi.so
#
# Reproducibility: same Dockerfile + same PYLON_DEB_URL/SHA256 + same Debian
# base tag => same .so.

# This is bookworm-20250908-slim
FROM debian@sha256:df52e55e3361a81ac1bead266f3373ee55d29aa50cf0975d440c2be3483d8ed3 AS builder

# This is an internal URL and not generally accessible. We pin to a specific
# version for reproducibility. The .deb contains the pylon SDK and can be
# downloaded from Basler's website.
ARG PYLON_DEB_URL=https://internal-static.strawlab.org/software/pylon/pylon_7.3.0.27189-deb0_amd64.deb
ARG PYLON_DEB_SHA256=145d69f0eb081410317a510c0033bd25037e6d2fb51d6caacee560908ab74816

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
           build-essential \
           ca-certificates \
           curl \
    && rm -rf /var/lib/apt/lists/*

# Download + verify + install the pylon .deb.
RUN curl --show-error --fail --silent "${PYLON_DEB_URL}" -o /tmp/pylon.deb \
    && echo "${PYLON_DEB_SHA256}  /tmp/pylon.deb" | sha256sum -c - \
    && apt-get update \
    && apt-get install -y --no-install-recommends /tmp/pylon.deb \
    && rm /tmp/pylon.deb \
    && rm -rf /var/lib/apt/lists/* \
    && test -f /opt/pylon/lib/libpylonbase.so \
    && test -d /opt/pylon/include/pylon

# Stage shim sources.
COPY pylon-cabi/include/pylon-cabi.h /work/include/pylon-cabi.h
COPY pylon-cabi/src/pylon-cabi.cc /work/src/pylon-cabi.cc
COPY pylon-cabi/Makefile /work/Makefile

WORKDIR /work
RUN make PYLON_ROOT=/opt/pylon \
    && test -f pylon-cabi.so \
    && ldd pylon-cabi.so

# Export stage: emits only the .so so `docker buildx build --output` writes
# a single file to the host without docker cp or a running container.
FROM scratch AS export
COPY --from=builder /work/pylon-cabi.so /
