#!/bin/sh
# Converts the markup of the upstream ControlCatalog sample into samples/ControlCatalog and
# copies its assets (pass --check to verify that the converted files and the assets are up to date).
# usage: scripts/sync-control-catalog.sh <upstream checkout> [--check]
#
# Portable (POSIX sh, BSD and GNU tools): runs as it is on macOS and Linux. The upstream checkout
# is required; `--check` may come before or after it.
set -e
USAGE="usage: sync-control-catalog.sh <upstream checkout> [--check]"
UPSTREAM=""
CHECK=""
for argument in "$@"; do
    case "$argument" in
        --check) CHECK="--check" ;;
        -*) echo "unknown option: $argument" >&2; echo "$USAGE" >&2; exit 2 ;;
        *)
            if [ -n "$UPSTREAM" ]; then
                echo "more than one upstream checkout given" >&2; echo "$USAGE" >&2; exit 2
            fi
            UPSTREAM="$argument"
            ;;
    esac
done
if [ -z "$UPSTREAM" ]; then
    echo "no upstream checkout given" >&2
    echo "$USAGE" >&2
    exit 2
fi
if [ ! -d "$UPSTREAM/samples/ControlCatalog" ]; then
    echo "not an upstream checkout (no samples/ControlCatalog): $UPSTREAM" >&2
    exit 2
fi
# Absolute, so that no path below can be read as an option of the tools.
UPSTREAM="$(cd "$UPSTREAM" && pwd)"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE="$UPSTREAM/samples/ControlCatalog"
TARGET="$ROOT/samples/ControlCatalog"

# The assets of the upstream project file: `Assets/**`, the shared application icons linked
# into `Assets/`, and the embedded resource `Pages/teapot.bin`.
if [ -n "$CHECK" ]; then
    # Both halves are checked and reported before the result.
    status=0
    python3 "$ROOT/scripts/convert_catalog_xaml.py" "$SOURCE" "$TARGET" --check || status=1
    diff -r -x icon.ico -x icon-32.png "$SOURCE/Assets" "$TARGET/Assets" || status=1
    cmp "$UPSTREAM/build/Assets/icon.ico" "$TARGET/Assets/icon.ico" || status=1
    cmp "$UPSTREAM/build/Assets/icon-32.png" "$TARGET/Assets/icon-32.png" || status=1
    cmp "$SOURCE/Pages/teapot.bin" "$TARGET/Pages/teapot.bin" || status=1
    if [ "$status" -eq 0 ]; then
        echo "markup and assets checked"
    else
        echo "the catalog differs from the upstream checkout (above)" >&2
    fi
    exit "$status"
else
    python3 "$ROOT/scripts/convert_catalog_xaml.py" "$SOURCE" "$TARGET"
    mkdir -p "$TARGET/Assets" "$TARGET/Pages"
    cp -R "$SOURCE/Assets/." "$TARGET/Assets/"
    cp "$UPSTREAM/build/Assets/icon.ico" "$UPSTREAM/build/Assets/icon-32.png" "$TARGET/Assets/"
    cp "$SOURCE/Pages/teapot.bin" "$TARGET/Pages/teapot.bin"
    echo "assets copied"
fi
