#!/bin/sh
# Runs the smoke example of the Linux framebuffer platform in every mode the machine allows
# (docs/porting/linux-framebuffer-platform.md, section 10, level 4).
#
#   smoke.sh <path of the built linuxfb_smoke example> <work directory> [state file of virtual-devices.sh]
#
# Always: a plain file as the framebuffer, in the three pixel formats, drawn through the back buffer, directly
# and with the blit thread. With a state file: the virtual framebuffer device and the virtual input devices it
# names (both need root). What is not available is said and skipped; the exit code is the number of failed runs.
set -u
bin="$1"
work="$2"
state="${3:-}"
mkdir -p "$work"
FB_NODE=""; FB_ID=""; UINPUT=""
[ -n "$state" ] && [ -f "$state" ] && . "$state"
failed=0

run() {
    name="$1"; shift
    echo "== $name: $*"
    if timeout 120 "$bin" "$@" > "$work/$name.log" 2>&1; then
        echo "   passed ($(grep -c '\[ ok \]' "$work/$name.log") checks)"
    else
        echo "   FAILED (exit $?)"
        failed=$((failed + 1))
    fi
    grep -E '\[ ok \]|\[FAILED\]|SMOKE|panicked|error' "$work/$name.log" | cut -c1-600
}

run file-bgra --file "$work/fb-bgra.raw"
run file-rgba --file "$work/fb-rgba.raw" --format=rgba
run file-rgb565 --file "$work/fb-rgb565.raw" --format=rgb565
run file-direct --file "$work/fb-direct.raw" --direct
run file-async --file "$work/fb-async.raw" --async --size=800x480

if [ -n "$UINPUT" ]; then
    run file-uinput --file "$work/fb-uinput.raw" --uinput
    if ldconfig -p 2>/dev/null | grep -q 'libinput\.so\.10'; then
        run file-libinput --file "$work/fb-libinput.raw" --uinput --libinput
    else
        echo "== file-libinput: skipped, libinput.so.10 is not installed"
    fi
else
    echo "== file-uinput: skipped, /dev/uinput is not available"
fi

if [ -n "$FB_NODE" ]; then
    # vfb starts with 8 bits per pixel: the output is asked for a 32-bit format.
    request=""
    [ "$FB_ID" = "Virtual FB" ] && request="--request-format=bgra"
    if [ -n "$UINPUT" ]; then
        run fbdev-uinput --fbdev "$FB_NODE" --expect-id "$FB_ID" --uinput $request
    else
        run fbdev --fbdev "$FB_NODE" --expect-id "$FB_ID" $request
    fi
    run fbdev-direct --fbdev "$FB_NODE" --expect-id "$FB_ID" --direct $request
    run fbdev-async --fbdev "$FB_NODE" --expect-id "$FB_ID" --async $request
    # The example must refuse a device that is not the one it was told to expect, before it writes.
    echo "== fbdev-refused: --fbdev $FB_NODE --expect-id not-this-device"
    if timeout 60 "$bin" --fbdev "$FB_NODE" --expect-id not-this-device > "$work/fbdev-refused.log" 2>&1; then
        echo "   FAILED: the example ran on a device it should have refused"
        failed=$((failed + 1))
    elif grep -q 'nothing was written to it' "$work/fbdev-refused.log"; then
        echo "   passed (refused)"
    else
        echo "   FAILED: it failed for another reason"
        failed=$((failed + 1))
    fi
else
    echo "== fbdev: skipped, no virtual framebuffer device (neither vfb nor the emulation of vkms is available)"
fi

echo "== smoke runs failed: $failed"
exit $failed
