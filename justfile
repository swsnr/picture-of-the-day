# The app ID to use (either de.swsnr.pictureoftheday or de.swsnr.pictureoftheday.Devel).
APPID := 'de.swsnr.pictureoftheday'

xgettext_opts := '--package-name=' + APPID + \
    ' --foreign-user --copyright-holder "Sebastian Wiesner <sebastian@swsnr.de>"' + \
    ' --sort-by-file --from-code=UTF-8 --add-comments'

version := `git describe`

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

# Extract the message template from all source files.
pot:
    find src -name '*.rs' > po/POTFILES.rs
    find resources/ -name '*.blp' > po/POTFILES.blp
    xgettext {{xgettext_opts}} --language=C --keyword=dpgettext2:2c,3 --files-from=po/POTFILES.rs --output=po/de.swsnr.pictureoftheday.rs.pot
    xgettext {{xgettext_opts}} --language=C --keyword=_ --keyword=C_:1c,2 --files-from=po/POTFILES.blp --output=po/de.swsnr.pictureoftheday.blp.pot
    xgettext {{xgettext_opts}} --output=po/de.swsnr.pictureoftheday.pot \
        po/de.swsnr.pictureoftheday.blp.pot \
        po/de.swsnr.pictureoftheday.rs.pot \
        resources/de.swsnr.pictureoftheday.metainfo.xml.in \
        de.swsnr.pictureoftheday.desktop.in \
        schemas/de.swsnr.pictureoftheday.gschema.xml
    rm -f po/POTFILES* po/de.swsnr.pictureoftheday.rs.pot po/de.swsnr.pictureoftheday.blp.pot
    @# We strip the POT-Creation-Date from the resulting POT because xgettext bumps
    @# it everytime regardless if anything else changed, and this just generates
    @# needless diffs.
    sed -i /POT-Creation-Date/d po/de.swsnr.pictureoftheday.pot

# Build and install development flatpak without sandboxing
flatpak-devel-install:
    flatpak run org.flatpak.Builder --force-clean --user --install \
        --install-deps-from=flathub --repo=.flatpak-repo \
        builddir flatpak/de.swsnr.pictureoftheday.Devel.yaml

# Lint the flatpak repo (you must run flatpak-build first)
lint-flatpak-repo:
    flatpak run --command=flatpak-builder-lint org.flatpak.Builder repo .flatpak-repo

# Build (but not install) regular flatpak
flatpak-build: && lint-flatpak-repo
    flatpak run org.flatpak.Builder --force-clean --sandbox \
        --install-deps-from=flathub --ccache \
        --mirror-screenshots-url=https://dl.flathub.org/media/ --repo=.flatpak-repo \
        builddir flatpak/de.swsnr.pictureoftheday.yaml

# Patch files for the Devel build
patch-devel:
    sed -Ei 's/^version = "([^"]+)"/version = "\1+{{version}}"/' Cargo.toml
    cargo update -p pictureoftheday
    sed -i '/{{APPID}}/! s/de\.swsnr\.pictureoftheday/{{APPID}}/g' \
        src/config.rs \
        de.swsnr.pictureoftheday.desktop.in \
        resources/de.swsnr.pictureoftheday.metainfo.xml.in \
        dbus-1/de.swsnr.pictureoftheday.service \
        schemas/de.swsnr.pictureoftheday.gschema.xml
