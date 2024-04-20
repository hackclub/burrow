FROM docker.io/library/rust:1.76.0-slim-bookworm AS builder

ARG TARGETPLATFORM
ARG LLVM_VERSION=16

ENV KEYRINGS /etc/apt/keyrings

RUN set -eux && \
    mkdir -p $KEYRINGS && \
    apt-get update && \
    apt-get install --no-install-recommends -y gpg curl musl-dev && \
    curl --proto '=https' --tlsv1.2 -sSf https://apt.llvm.org/llvm-snapshot.gpg.key | gpg --dearmor --output $KEYRINGS/llvm.gpg && \
    echo "deb [signed-by=$KEYRINGS/llvm.gpg] http://apt.llvm.org/bookworm/ llvm-toolchain-bookworm-$LLVM_VERSION main" > /etc/apt/sources.list.d/llvm.list && \
    apt-get update && \
    apt-get install --no-install-recommends -y clang-$LLVM_VERSION llvm-$LLVM_VERSION lld-$LLVM_VERSION build-essential sqlite3 libsqlite3-dev musl musl-tools musl-dev && \
    ln -s clang-$LLVM_VERSION /usr/bin/clang && \
    ln -s clang /usr/bin/clang++ && \
    ln -s lld-$LLVM_VERSION /usr/bin/ld.lld && \
    ln -s clang-$LLVM_VERSION /usr/bin/clang-cl && \
    ln -s llvm-ar-$LLVM_VERSION /usr/bin/llvm-lib && \
    ln -s lld-link-$LLVM_VERSION /usr/bin/lld-link && \
    update-alternatives --install /usr/bin/cc cc /usr/bin/clang 100 && \
    update-alternatives --install /usr/bin/c++ c++ /usr/bin/clang++ 100 && \
    apt-get remove -y --auto-remove && \
    rm -rf /var/lib/apt/lists/*

ARG SQLITE_VERSION=3400100

RUN case $TARGETPLATFORM in \
     "linux/arm64") LLVM_TARGET=aarch64-unknown-linux-musl MUSL_TARGET=aarch64-linux-musl ;; \
     "linux/amd64") LLVM_TARGET=x86_64-unknown-linux-musl MUSL_TARGET=x86_64-linux-musl ;; \
     *) exit 1 ;; \
    esac && \
    rustup target add $LLVM_TARGET && \
    curl --proto '=https' --tlsv1.2 -sSfO https://www.sqlite.org/2022/sqlite-autoconf-$SQLITE_VERSION.tar.gz && \
    tar xf sqlite-autoconf-$SQLITE_VERSION.tar.gz && \
    rm sqlite-autoconf-$SQLITE_VERSION.tar.gz && \
    cd sqlite-autoconf-$SQLITE_VERSION && \
    ./configure --disable-shared \
        CC="clang-$LLVM_VERSION -target $LLVM_TARGET" \
        CFLAGS="-I/usr/local/include -I/usr/include/$MUSL_TARGET" \
        LDFLAGS="-L/usr/local/lib -L/usr/lib/$MUSL_TARGET -L/lib/$MUSL_TARGET" && \
    make && \
    make install && \
    cd .. && \
    rm -rf sqlite-autoconf-$SQLITE_VERSION

ENV SQLITE3_STATIC=1 \
    SQLITE3_INCLUDE_DIR=/usr/local/include \
    SQLITE3_LIB_DIR=/usr/local/lib

ENV CC_x86_64_unknown_linux_musl=clang-$LLVM_VERSION \
    AR_x86_64_unknown_linux_musl=llvm-ar-$LLVM_VERSION \
    CC_aarch64_unknown_linux_musl=clang-$LLVM_VERSION \
    AR_aarch64_unknown_linux_musl=llvm-ar-$LLVM_VERSION \
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-L/usr/lib/x86_64-linux-musl -L/lib/x86_64-linux-musl -C linker=rust-lld" \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-L/usr/lib/aarch64-linux-musl -L/lib/aarch64-linux-musl -C linker=rust-lld" \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

COPY . .

RUN case $TARGETPLATFORM in \
     "linux/arm64") LLVM_TARGET=aarch64-unknown-linux-musl ;; \
     "linux/amd64") LLVM_TARGET=x86_64-unknown-linux-musl ;; \
     *) exit 1 ;; \
    esac && \
    cargo install --path burrow --target $LLVM_TARGET

WORKDIR /tmp/rootfs

RUN set -eux && \
    mkdir -p ./bin ./etc ./tmp ./data && \
    mv /usr/local/cargo/bin/burrow ./bin/burrow && \
    echo 'burrow:x:10001:10001::/tmp:/sbin/nologin' > ./etc/passwd && \
    echo 'burrow:x:10001:' > ./etc/group && \
    chown -R 10001:10001 ./tmp ./data && \
    chmod 0777 ./tmp

FROM scratch as runtime
LABEL \
    # https://github.com/opencontainers/image-spec/blob/master/annotations.md
    org.opencontainers.image.title="burrow" \
    org.opencontainers.image.description="Burrow is an open source tool for burrowing through firewalls, built by teenagers at Hack Club." \
    org.opencontainers.image.url="https://github.com/hackclub/burrow" \
    org.opencontainers.image.source="https://github.com/hackclub/burrow" \
    org.opencontainers.image.vendor="hackclub" \
    org.opencontainers.image.licenses="GPL-3.0"

USER 10001:10001
COPY --from=builder /tmp/rootfs /
WORKDIR /data

ENTRYPOINT ["/bin/burrow"]
