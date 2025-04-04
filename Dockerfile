FROM docker.io/alpine:edge

RUN apk upgrade --no-cache && apk add --nocache gcc pkgconf \
    blueprint-compiler \
    libadwaita-dev libsoup3-dev
