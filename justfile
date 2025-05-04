# The app ID to use (either de.swsnr.pictureoftheday or de.swsnr.pictureoftheday.Devel).
APPID := 'de.swsnr.pictureoftheday'
# The destination prefix to install files to.  Combines traditional DESTDIR and
# PREFIX variables; picture-of-the-day does not encode the prefix into its binary
# and thus does not need to distinguish between the prefix and the destdir.
DESTPREFIX := '/app'

xgettext_opts := '--package-name=' + APPID + \
    ' --foreign-user --copyright-holder "Sebastian Wiesner <sebastian@swsnr.de>"' + \
    ' --sort-by-file --from-code=UTF-8 --add-comments'

version := `git describe`
release_archive := 'picture-of-the-day-' + version + '.tar.zst'
release_vendor_archive := 'picture-of-the-day-' + version + '-vendor.tar.zst'

default:
    just --list

# Remove build files from source code tree
clean:
    rm -fr builddir .flatpak-repo .flatpak-builder dist vendor

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

_install-po po_file:
    install -dm0755 '{{DESTPREFIX}}/share/locale/{{file_stem(po_file)}}/LC_MESSAGES'
    msgfmt -o '{{DESTPREFIX}}/share/locale/{{file_stem(po_file)}}/LC_MESSAGES/{{APPID}}.mo' '{{po_file}}'

# Install after cargo build --release
install:
    find po/ -name '*.po' -exec just version= DESTPREFIX='{{DESTPREFIX}}' APPID='{{APPID}}' _install-po '{}' ';'
    install -Dm0755 target/release/pictureoftheday '{{DESTPREFIX}}/bin/{{APPID}}'
    install -Dm0644 -t '{{DESTPREFIX}}/share/icons/hicolor/scalable/apps/' 'resources/icons/scalable/apps/{{APPID}}.svg'
    install -Dm0644 resources/icons/symbolic/apps/de.swsnr.pictureoftheday-symbolic.svg \
        '{{DESTPREFIX}}/share/icons/hicolor/symbolic/apps/{{APPID}}-symbolic.svg'
    install -Dm0644 de.swsnr.pictureoftheday.desktop '{{DESTPREFIX}}/share/applications/{{APPID}}.desktop'
    install -Dm0644 resources/de.swsnr.pictureoftheday.metainfo.xml '{{DESTPREFIX}}/share/metainfo/{{APPID}}.metainfo.xml'
    install -Dm0644 dbus-1/de.swsnr.pictureoftheday.service '{{DESTPREFIX}}/share/dbus-1/services/{{APPID}}.service'
    install -Dm0644 schemas/de.swsnr.pictureoftheday.gschema.xml '{{DESTPREFIX}}/share/glib-2.0/schemas/{{APPID}}.gschema.xml'
    glib-compile-schemas --strict '{{DESTPREFIX}}/share/glib-2.0/schemas'

_dist:
    rm -rf dist
    mkdir dist

# Build and sign a reproducible archive of cargo vendor sources
_vendor: _dist
    rm -rf vendor/
    cargo vendor --locked
    echo SOURCE_DATE_EPOCH="$(env LC_ALL=C TZ=UTC0 git show --quiet --date='format-local:%Y-%m-%dT%H:%M:%SZ' --format="%cd" HEAD)"
    # See https://reproducible-builds.org/docs/archives/
    env LC_ALL=C TZ=UTC0 tar --numeric-owner --owner 0 --group 0 \
        --sort name --mode='go+u,go-w' --format=posix \
        --pax-option=exthdr.name=%d/PaxHeaders/%f \
        --pax-option=delete=atime,delete=ctime \
        --mtime="$(env LC_ALL=C TZ=UTC0 git show --quiet --date='format-local:%Y-%m-%dT%H:%M:%SZ' --format="%cd" HEAD)" \
        -c -f "dist/{{release_vendor_archive}}" \
        --zstd vendor

# Build and sign a reproducible git archive bundle
_git-archive: _dist
    env LC_ALL=C TZ=UTC0 git archive --format tar \
        --prefix "{{without_extension(without_extension(release_archive))}}/" \
        --output "dist/{{without_extension(release_archive)}}" HEAD
    zstd --rm "dist/{{without_extension(release_archive)}}"

_release_notes: _dist
    appstreamcli metainfo-to-news resources/de.swsnr.pictureoftheday.metainfo.xml.in dist/news.yaml
    yq eval-all '[.]' -oj dist/news.yaml > dist/news.json
    jq -r --arg tag "$(git describe)" '.[] | select(.Version == ($tag | ltrimstr("v"))) | .Description | tostring' > dist/relnotes.md < dist/news.json
    rm dist/news.{json,yaml}

package: _git-archive _vendor _release_notes
    curl https://codeberg.org/swsnr.keys > dist/key
    ssh-keygen -Y sign -f dist/key -n file "dist/{{release_archive}}"
    ssh-keygen -Y sign -f dist/key -n file "dist/{{release_vendor_archive}}"
    rm dist/key

_post-release:
    @echo "Run just package to create dist archives."
    @echo "Create new release at https://codeberg.org/swsnr/picture-of-the-day/tags"
    @echo "Use dist/relnotes.md as release body"
    @echo "Attach archives and signatures in dist as release body"
    @echo "Then run just flatpak-update-manifest to update the flatpak manifest."

release *ARGS: test-all && _post-release
    cargo release {{ARGS}}

flatpak-update-manifest:
    yq eval -i '.modules.[1].sources.[0].url = "https://codeberg.org/swsnr/picture-of-the-day/releases/download/$TAG_NAME/picture-of-the-day-$TAG_NAME.tar.zst"' flatpak/de.swsnr.pictureoftheday.yaml
    yq eval -i '.modules.[1].sources.[0].sha256 = "$ARCHIVE_SHA256"' flatpak/de.swsnr.pictureoftheday.yaml
    yq eval -i '.modules.[1].sources.[1].url = "https://codeberg.org/swsnr/picture-of-the-day/releases/download/$TAG_NAME/picture-of-the-day-$TAG_NAME-vendor.tar.zst"' flatpak/de.swsnr.pictureoftheday.yaml
    yq eval -i '.modules.[1].sources.[1].sha256 = "$VENDOR_SHA256"' flatpak/de.swsnr.pictureoftheday.yaml
    env TAG_NAME="{{version}}" \
        ARCHIVE_SHA256={{sha256_file('dist' / release_archive)}} \
        VENDOR_SHA256={{sha256_file('dist' / release_vendor_archive)}} \
        yq eval -i '(.. | select(tag == "!!str")) |= envsubst' flatpak/de.swsnr.pictureoftheday.yaml
    git add flatpak/de.swsnr.pictureoftheday.yaml
    git commit -m 'Update flatpak manifest for {{version}}'
    @echo "Run git push and trigger sync workflow at https://github.com/flathub/de.swsnr.pictureoftheday/actions/workflows/sync.yaml"
