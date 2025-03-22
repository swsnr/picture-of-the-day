FROM docker.io/archlinux:latest

RUN pacman -Syu --noconfirm gcc pkgconf libadwaita blueprint-compiler libsoup3 && \
    rm -rf /var/cache/pacman/pkg /var/lib/pacman/sync
