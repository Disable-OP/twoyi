# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# Ubuntu 22.04 base with all twoyi build dependencies pre-installed.
# We use a Dockerfile (instead of devcontainer features) because the
# feature-based approach failed silently and fell back to the default
# Alpine image, which doesn't have glibc (required by the Android
# emulator binary).

FROM mcr.microsoft.com/devcontainers/base:ubuntu-22.04

# Install everything we need in one layer to keep the image small.
# - JDK 17 (Temurin via the deadsnakes PPA would be ideal, but the
#   default-jdk package on Ubuntu 22.04 is OpenJDK 17 which works)
# - Rust (via rustup, installed as the vscode user)
# - Android SDK cmdline-tools (downloaded from Google)
# - Android NDK r27c
# - Docker CLI (for redroid testing)
# - QEMU + KVM utilities
# - Build essentials (gcc, g++, make, pkg-config)
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
        openjdk-17-jdk \
        ca-certificates \
        curl \
        wget \
        unzip \
        zip \
        git \
        build-essential \
        gcc \
        g++ \
        make \
        pkg-config \
        libssl-dev \
        libpulse0 \
        libnss3 \
        libxkbfile1 \
        libgl1 \
        libegl1 \
        libx11-6 \
        libx11-xcb1 \
        libxcb1 \
        libxcomposite1 \
        libxcursor1 \
        libxdamage1 \
        libxext6 \
        libxfixes3 \
        libxi6 \
        libxrandr2 \
        libxrender1 \
        libxtst6 \
        libgtk-3-0 \
        cpu-checker \
        qemu-kvm \
        libvirt-clients \
        bridge-utils \
        docker.io \
        docker-compose \
        apt-transport-https \
        software-properties-common \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

# Set up environment variables for Android development
ENV ANDROID_HOME=/workspaces/twoyi/.android-sdk
ENV ANDROID_NDK_HOME=/workspaces/twoyi/.android-ndk
ENV ANDROID_SDK_ROOT=/workspaces/twoyi/.android-sdk
ENV JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64
ENV PATH="${PATH}:${ANDROID_HOME}/cmdline-tools/latest/bin:${ANDROID_HOME}/platform-tools:${ANDROID_HOME}/emulator:${HOME}/.cargo/bin"

# Install Rust as the vscode user (not root)
USER vscode
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable \
    && . $HOME/.cargo/env \
    && rustup target add aarch64-linux-android x86_64-linux-android \
    && cargo install cargo-xdk

# The SDK and NDK will be downloaded by the postCreateCommand script
# (in .devcontainer/scripts/setup.sh) because they're large and caching
# them in the image would bloat it.
