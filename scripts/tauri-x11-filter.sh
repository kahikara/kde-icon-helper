#!/usr/bin/env bash
set -euo pipefail

GDK_BACKEND=x11 WINIT_UNIX_BACKEND=x11 tauri "$@" \
  2> >(grep -vE "^Couldn't get key from code: Unidentified\\(Gtk\\([0-9]+\\)\\)$" >&2)
