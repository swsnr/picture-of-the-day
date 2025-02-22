FROM ghcr.io/gtk-rs/gtk4-rs/gtk4:latest

RUN dnf update --assumeyes && \
    dnf --assumeyes install libnghttp2-devel brotli-devel sqlite-devel libpsl-devel gnutls-devel && \
    dnf clean all --assumeyes

RUN git clone https://gitlab.gnome.org/GNOME/glib-networking.git --depth 1 && \
    (cd /glib-networking && \
        meson setup builddir --prefix=/usr --buildtype release -Dlibproxy=disabled -Dgnome_proxy=disabled && \
        meson install -C builddir) && \
    git clone https://gitlab.gnome.org/GNOME/libsoup.git --depth=1 --branch libsoup-3-4 && \
    (cd /libsoup && \
        meson setup builddir --prefix=/usr --buildtype release -Dintrospection=disabled -Dvapi=disabled -Ddocs=disabled -Dtests=false -Dautobahn=disabled -Dsysprof=disabled -Dpkcs11_tests=disabled && \
        meson install -C builddir) && \
    rm -rf /libsoup /glib-networking
