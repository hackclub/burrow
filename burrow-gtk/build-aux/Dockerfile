FROM fedora:39

ENV DEBIAN_FRONTEND=noninteractive

RUN set -eux && \
    dnf update -y && \
    dnf install -y clang ninja-build cmake meson gtk4-devel glib2-devel libadwaita-devel desktop-file-utils libappstream-glib util-linux wget fuse fuse-libs file sqlite-devel protobuf-compiler protobuf-devel

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app
COPY . /app

ENV SQLITE3_STATIC=1

RUN cd /app/burrow-gtk/ && \
    ./build-aux/build_appimage.sh


