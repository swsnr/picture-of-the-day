# The app ID to use.
#
# Use de.swsnr.picture-of-the-day for the standard app ID, and de.swsnr.picture-of-the-day.Devel to
# build a nightly snapshot.  Other values are not supported.
APPID = de.swsnr.picture-of-the-day
# The destination prefix to install files to.  Combines traditional DESTDIR and
# PREFIX variables; picture-of-the-day does not encode the prefix into its binary and thus
# does not need to distinguish between the prefix and the destdir.
DESTPREFIX = /app
# Installation directory for locale files.
LOCALEDIR = $(DESTPREFIX)/share/locale/

GIT_DESCRIBE = $(shell git describe)

BLUEPRINTS = $(wildcard ui/*.blp)
CATALOGS = $(wildcard po/*.po)

XGETTEXT_OPTS = \
	--package-name=$(APPID) \
	--foreign-user --copyright-holder "Sebastian Wiesner <sebastian@swsnr.de>" \
	--sort-by-file --from-code=UTF-8 --add-comments

# Extract the message template from all source files.
#
# You typically do not need to run this manually: The gettext Github workflow
# watches for changes to relevant source files, runs this target, and opens a
# pull request with the corresponding changes.
#
# When changing the set of files taken into account for xgettext also update the
# paths list in the gettext.yml workflow to make sure that updates to these
# files are caught by the gettext workflows.
#
# We strip the POT-Creation-Date from the resulting POT because xgettext bumps
# it everytime regardless if anything else changed, and this just generates
# needless diffs.
.PHONY: pot
pot:
	find src -name '*.rs' > po/POTFILES.rs
	find resources/ -name '*.blp' > po/POTFILES.blp
	xgettext $(XGETTEXT_OPTS) --language=C --keyword=_ --keyword=C_:1c,2 --files-from=po/POTFILES.blp --output=po/de.swsnr.picture-of-the-day.blp.pot
	xgettext $(XGETTEXT_OPTS) --output=po/de.swsnr.picture-of-the-day.pot \
		po/de.swsnr.picture-of-the-day.blp.pot \
		de.swsnr.picture-of-the-day.desktop.in
	rm -f po/POTFILES* po/de.swsnr.picture-of-the-day.rs.pot po/de.swsnr.picture-of-the-day.blp.pot
	sed -i /POT-Creation-Date/d po/de.swsnr.picture-of-the-day.pot

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
	install -Dm0755 target/release/picture-of-the-day $(DESTPREFIX)/bin/$(APPID)
	install -Dm0644 -t $(DESTPREFIX)/share/icons/hicolor/scalable/apps/ resources/icons/scalable/apps/$(APPID).svg
	install -Dm0644 resources/icons/symbolic/apps/de.swsnr.picture-of-the-day-symbolic.svg \
		$(DESTPREFIX)/share/icons/hicolor/symbolic/apps/$(APPID)-symbolic.svg
	install -Dm0644 de.swsnr.picture-of-the-day.desktop.in $(DESTPREFIX)/share/applications/$(APPID).desktop
	install -Dm0644 dbus-1/de.swsnr.picture-of-the-day.service $(DESTPREFIX)/share/dbus-1/services/$(APPID).service

# Patch the current git describe version into Picture Of The Day.
.PHONY: patch-git-version
patch-git-version:
	sed -Ei 's/^version = "([^"]+)"/version = "\1+$(GIT_DESCRIBE)"/' Cargo.toml
	cargo update -p picture-of-the-day

# Patch the app ID to use APPID in various files
.PHONY: patch-appid
patch-appid:
	sed -i '/$(APPID)/! s/de\.swsnr\.picture-of-the-day/$(APPID)/g' \
		src/config.rs \
		de.swsnr.picture-of-the-day.desktop.in \
		dbus-1/de.swsnr.picture-of-the-day.service
