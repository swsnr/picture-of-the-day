default:
    just --list

vet *ARGS:
    @# Only consider Linux dependencies, as that's all I care for.
    @# Seems to be unofficial, see https://github.com/mozilla/cargo-vet/issues/579, but works
    env CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu cargo vet {{ARGS}}

lint-blueprint:
    blueprint-compiler format resources/**/*.blp

lint-rust:
    cargo +stable deny --all-features --locked check
    cargo +stable fmt -- --check
    cargo +stable clippy --all-targets
    # Run clippy over the scraper binary
    cargo clippy --all-features --all-targets

lint-flatpak:
    flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest flatpak/de.swsnr.pictureoftheday.yaml
    flatpak run --command=flatpak-builder-lint org.flatpak.Builder appstream resources/de.swsnr.pictureoftheday.metainfo.xml

lint-data:
    appstreamcli validate --explain resources/de.swsnr.pictureoftheday.metainfo.xml

lint-all: lint-rust lint-blueprint lint-data lint-flatpak

test-rust:
    cargo +stable build
    cargo +stable test

test-all: (vet "--locked") lint-all test-rust
