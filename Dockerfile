FROM docker.io/fedora:42
LABEL org.opencontainers.image.description "CI image for de.swsnr.pictureoftheday"

RUN dnf install -y blueprint-compiler libsoup3-devel libadwaita-devel gcc pkgconf git gettext make appstream && \
    dnf clean all && rm -rf /var/cache/yum
