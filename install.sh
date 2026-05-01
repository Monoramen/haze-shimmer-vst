#!/usr/bin/env bash
# Builds the release bundle and installs the VST3 locally.
# Default target: %LOCALAPPDATA%\Programs\Common\VST3  (no admin needed, user scope)
# Pass --system to install into %COMMONPROGRAMFILES%\VST3 (requires admin terminal).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

SYSTEM=0
for arg in "$@"; do
    case "$arg" in
        --system) SYSTEM=1 ;;
        *) echo "unknown arg: $arg" >&2; exit 2 ;;
    esac
done

cargo nih-plug bundle shimmer_granular --release

SRC="target/bundled/shimmer_granular.vst3"

if [ "$SYSTEM" -eq 1 ]; then
    DEST="/c/Program Files/Common Files/VST3"
else
    LOCAL="${LOCALAPPDATA:-$USERPROFILE/AppData/Local}"
    DEST="$(cygpath -u "$LOCAL" 2>/dev/null || echo "$LOCAL")/Programs/Common/VST3"
fi

mkdir -p "$DEST"

if [ -e "$DEST/shimmer_granular.vst3" ]; then
    rm -rf "$DEST/shimmer_granular.vst3"
fi

cp -r "$SRC" "$DEST/"
echo "Installed: $DEST/shimmer_granular.vst3"
