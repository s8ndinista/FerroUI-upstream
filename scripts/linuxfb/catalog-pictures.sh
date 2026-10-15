#!/bin/sh
# Runs the ControlCatalog on a (virtual) framebuffer device or on a KMS output of a (virtual) card and turns
# the frame, written per page by the catalog, into PNG files (docs/porting/linux-framebuffer-platform.md,
# section 10, level 6).
#
#   catalog-pictures.sh <control-catalog-desktop> <framebuffer node> <expected identification> <output directory> [ms per page]
#   catalog-pictures.sh <control-catalog-desktop> /dev/dri/cardN <expected driver> <output directory> [ms per page]
#
# The catalog refuses a framebuffer device whose identification does not begin with the expected one, and a card
# whose driver is not the expected one. What is turned into pictures is the memory of that framebuffer device, or
# the framebuffer the CRTC of the card scans out; nothing is captured from a screen. Run as root (the device).
set -u
bin="$1"
node="$2"
id="$3"
out="$4"
ms="${5:-2500}"
here="$(cd "$(dirname "$0")" && pwd)"
mkdir -p "$out/raw"
rm -f "$out"/raw/*.raw "$out"/*.png

case "$node" in
    /dev/dri/*)
        FERROUI_LINUXFB_EXPECT_DRIVER="$id" FERROUI_LINUXFB_DUMPS="$out/raw" FERROUI_LINUXFB_PAGE_MS="$ms" \
            timeout 600 "$bin" --drm --card "$node" --no-input > "$out/catalog.log" 2>&1
        status=$? ;;
    *)
        FRAMEBUFFER="$node" FERROUI_LINUXFB_EXPECT_ID="$id" FERROUI_LINUXFB_DUMPS="$out/raw" FERROUI_LINUXFB_PAGE_MS="$ms" \
            timeout 600 "$bin" --fbdev --no-input > "$out/catalog.log" 2>&1
        status=$? ;;
esac
echo "catalog: exit $status"
grep -E 'ControlCatalog:|Dumps|Selecting|No page|panicked|error' "$out/catalog.log" | cut -c1-300

pictures=0
varied=0
# Each line: Dump: <path> <width>x<height> <bits per pixel> <line length>
grep '^Dump: .*\.raw ' "$out/catalog.log" | while read -r _ raw size bits line; do
    name="$(basename "$raw" .raw)"
    order=bgra
    [ "$bits" = 16 ] && order=rgb565
    python3 "$here/fb-to-png.py" "$raw" "$out/$name.png" "$size" "$bits" "$order" "$line"
done > "$out/pictures.log" 2>&1
cat "$out/pictures.log"
pictures=$(ls "$out"/*.png 2>/dev/null | wc -l)
# A page of the catalog has many colours; a blank or single-colour frame has few.
varied=$(grep -E ', [0-9]+ distinct colours' "$out/pictures.log" | sed 's/.*, \([0-9]*\) distinct colours.*/\1/' | awk '$1 >= 16' | wc -l)
rm -rf "$out/raw"
echo "pictures: $pictures, of which $varied show a page (16 or more colours in the sample)"
if [ "$status" -ne 0 ] || [ "$pictures" -eq 0 ] || [ "$varied" -ne "$pictures" ]; then
    echo "catalog-pictures: FAILED"
    exit 1
fi
echo "catalog-pictures: passed"
