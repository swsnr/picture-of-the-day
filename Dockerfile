FROM docker.io/fedora:42
LABEL org.opencontainers.image.description "CI image for de.swsnr.pictureoftheday"

# We need:
# - blueprint-compiler to format our blueprint files.
# - libadwaita-devel and libsoup-devel for the libadwaita and libsoup Gtk bindings.  Gtk4 comes in transitively.
# - gcc and pkgconf to build Rust bindings for the above libraries.
# - git obviously
# - gettext for xgettext in our gettext workflow.
# - appstream to provide the gettext extraction rules for appstream metadata XML files
# - glib-networking for TLS supports in our unit tests
RUN dnf install -y --setopt=install_weak_deps=False blueprint-compiler libsoup3-devel libadwaita-devel gcc pkgconf git gettext make appstream glib-networking && \
    dnf clean all && rm -rf /var/cache/yum
