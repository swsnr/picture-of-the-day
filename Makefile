# The app ID to use.
#
# Use de.swsnr.pictureoftheday for the standard app ID, and de.swsnr.pictureoftheday.Devel to
# build a nightly snapshot.  Other values are not supported.
APPID = de.swsnr.pictureoftheday
# The destination prefix to install files to.  Combines traditional DESTDIR and
# PREFIX variables; picture-of-the-day does not encode the prefix into its binary and thus
# does not need to distinguish between the prefix and the destdir.
DESTPREFIX = /app
# Installation directory for locale files.
LOCALEDIR = $(DESTPREFIX)/share/locale/

GIT_DESCRIBE = $(shell git describe --always)

BLUEPRINTS = $(wildcard ui/*.blp)
CATALOGS = $(wildcard po/*.po)

po/%.mo: po/%.po
	msgfmt --output-file $@ --check $<

# Compile binary message catalogs from message catalogs
.PHONY: msgfmt
msgfmt: $(addsuffix .mo,$(basename $(CATALOGS)))

$(LOCALEDIR)/%/LC_MESSAGES/$(APPID).mo: po/%.mo
	install -Dpm0644 $< $@

# Install compiled locale message catalogs.
.PHONY: install-locale
install-locale: $(addprefix $(LOCALEDIR)/,$(addsuffix /LC_MESSAGES/$(APPID).mo,$(notdir $(basename $(CATALOGS)))))

# Install Picture Of The Day into $DESTPREFIX using $APPID.
#
# You must run cargo build --release before invoking this target!
.PHONY: install
install: install-locale
	install -Dm0755 target/release/pictureoftheday $(DESTPREFIX)/bin/$(APPID)
	install -Dm0644 -t $(DESTPREFIX)/share/icons/hicolor/scalable/apps/ resources/icons/scalable/apps/$(APPID).svg
	install -Dm0644 resources/icons/symbolic/apps/de.swsnr.pictureoftheday-symbolic.svg \
		$(DESTPREFIX)/share/icons/hicolor/symbolic/apps/$(APPID)-symbolic.svg
	install -Dm0644 de.swsnr.pictureoftheday.desktop $(DESTPREFIX)/share/applications/$(APPID).desktop
	install -Dm0644 resources/de.swsnr.pictureoftheday.metainfo.xml $(DESTPREFIX)/share/metainfo/$(APPID).metainfo.xml
	install -Dm0644 dbus-1/de.swsnr.pictureoftheday.service $(DESTPREFIX)/share/dbus-1/services/$(APPID).service
	install -Dm0644 schemas/de.swsnr.pictureoftheday.gschema.xml $(DESTPREFIX)/share/glib-2.0/schemas/$(APPID).gschema.xml

# Install for flatpak.  Like install, but with some extra flatpak-specific steps
.PHONY: install-flatpak
install-flatpak: install
	glib-compile-schemas --strict $(DESTPREFIX)/share/glib-2.0/schemas

# Patch the current git describe version into Picture Of The Day.
.PHONY: patch-git-version
patch-git-version:
	sed -Ei 's/^version = "([^"]+)"/version = "\1+$(GIT_DESCRIBE)"/' Cargo.toml
	cargo update -p pictureoftheday

# Patch the app ID to use APPID in various files
.PHONY: patch-appid
patch-appid:
	sed -i '/$(APPID)/! s/de\.swsnr\.pictureoftheday/$(APPID)/g' \
		src/config.rs \
		de.swsnr.pictureoftheday.desktop.in \
		resources/de.swsnr.pictureoftheday.metainfo.xml.in \
		dbus-1/de.swsnr.pictureoftheday.service \
		schemas/de.swsnr.pictureoftheday.gschema.xml

# Remove compiled message catalogs and other generated files, and flatpak
# things
.PHONY: clean
clean:
	rm -fr po/*.mo builddir .flatpak-repo .flatpak-builder

# Build a development flatpak without sandbox.
.PHONY: flatpak-devel
flatpak-devel:
	flatpak run org.flatpak.Builder --force-clean --user --install \
		--install-deps-from=flathub --repo=.flatpak-repo \
		builddir flatpak/de.swsnr.pictureoftheday.Devel.yaml

# Build a regular flatpak (sandboxed build)
.PHONY: flatpak
flatpak:
	flatpak run org.flatpak.Builder --force-clean --sandbox --user --install \
		--install-deps-from=flathub --ccache \
		--mirror-screenshots-url=https://dl.flathub.org/media/ --repo=.flatpak-repo \
		builddir flatpak/de.swsnr.pictureoftheday.yaml

.PHONY: flatpak-lint-manifest
flatpak-lint-manifest:
	flatpak run --command=flatpak-builder-lint org.flatpak.Builder \
		manifest flatpak/de.swsnr.pictureoftheday.yaml

.PHONY: flatpak-lint-repo
flatpak-lint-repo:
	flatpak run --command=flatpak-builder-lint org.flatpak.Builder repo .flatpak-repo
