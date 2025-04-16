#!/usr/bin/env bash

set -euo pipefail

DIR="$(mktemp -d)"
trap 'rm -rf -- "${DIR}"' EXIT

variables=(
    # Run app with default settings: Force the in-memory gsettings backend to
    # block access to standard Gtk settings, and tell Adwaita not to access
    # portals to prevent it from getting dark mode and accent color from desktop
    # settings.
    #
    # Effectively this makes our app run with default settings.
    GSETTINGS_BACKEND=memory
    ADW_DISABLE_PORTAL=1
    XDG_CONFIG_HOME="${DIR}/config"
    XDG_DATA_HOME="${DIR}/share"
    LC_MESSAGES=en_US.UTF-8
)

exec env "${variables[@]}" cargo run -- --date=2025-03-08 "${@}"
