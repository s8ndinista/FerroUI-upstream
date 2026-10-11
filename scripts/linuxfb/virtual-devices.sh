#!/bin/sh
# Virtual devices for the tests of the Linux framebuffer platform (docs/porting/linux-framebuffer-platform.md,
# section 10). Run as root.
#
#   virtual-devices.sh up <state file>     loads what the kernel has of vfb, vkms and uinput and writes the result
#   virtual-devices.sh down <state file>   unloads the modules that "up" loaded
#
# "up" writes shell assignments to the state file and prints them:
#   FB_NODE=/dev/fbN      a virtual framebuffer device, or empty
#   FB_ID=<prefix>        the beginning of its identification ("Virtual FB" of vfb, "vkms" of the emulation of vkms)
#   DRM_CARD=/dev/dri/cardN   the card of vkms, or empty
#   UINPUT=1              /dev/uinput can be opened, or empty
#   LOADED="vfb vkms uinput"  the modules this run loaded (those that were loaded before are left alone)
#
# How a virtual framebuffer is told from the console of the machine: the nodes of /sys/class/graphics are listed
# before a module is loaded; only a node that appeared afterwards AND whose name is that of the virtual driver is
# reported. A framebuffer the machine had before (its console) is never reported, whatever its name.
set -u
action="${1:-}"
state="${2:-}"
if [ -z "$action" ] || [ -z "$state" ]; then
    echo "usage: $0 up|down <state file>" >&2
    exit 2
fi

fb_nodes() { ls /sys/class/graphics 2>/dev/null | grep '^fb[0-9][0-9]*$' | sort; }
loaded() { [ -d "/sys/module/$1" ]; }

if [ "$action" = down ]; then
    LOADED=""
    [ -f "$state" ] && . "$state"
    status=0
    # In the reverse order of loading.
    for module in uinput vkms vfb; do
        case " $LOADED " in
            *" $module "*)
                if modprobe -r "$module" 2>/dev/null; then
                    echo "unloaded $module"
                else
                    echo "could not unload $module (still in use: $(cat /sys/module/$module/refcnt 2>/dev/null) references)"
                    status=1
                fi ;;
        esac
    done
    rm -f "$state"
    exit $status
fi

before="$(fb_nodes)"
echo "framebuffer devices before: $(echo $before | tr '\n' ' ')$(for n in $before; do printf '%s="%s" ' "$n" "$(cat /sys/class/graphics/$n/name 2>/dev/null)"; done)"
LOADED=""
FB_NODE=""
FB_ID=""
DRM_CARD=""
UINPUT=""

# A node that was not there before and has the expected name.
new_fb() {
    prefix="$1"
    for n in $(fb_nodes); do
        echo "$before" | grep -qx "$n" && continue
        name="$(cat /sys/class/graphics/$n/name 2>/dev/null)"
        case "$name" in
            "$prefix"*) echo "$n"; return 0 ;;
        esac
    done
    return 1
}

# vfb: the kernel's virtual framebuffer (CONFIG_FB_VIRTUAL).
if loaded vfb; then
    echo "vfb: already loaded, left alone"
elif modprobe vfb vfb_enable=1 2>/dev/null; then
    LOADED="$LOADED vfb"
    sleep 1
    if n="$(new_fb 'Virtual FB')"; then
        FB_NODE="/dev/$n"; FB_ID="Virtual FB"
        echo "vfb: loaded, $FB_NODE"
    else
        echo "vfb: loaded, but no new framebuffer device named \"Virtual FB\" appeared"
    fi
else
    echo "vfb: the module is not available for kernel $(uname -r) (CONFIG_FB_VIRTUAL is not set, or the module is in a package that is not installed)"
fi

# vkms: the virtual KMS driver (CONFIG_DRM_VKMS); its fbdev emulation is a framebuffer device as well.
if loaded vkms; then
    echo "vkms: already loaded, left alone"
elif modprobe vkms 2>/dev/null; then
    LOADED="$LOADED vkms"
    sleep 1
    for c in /sys/class/drm/card[0-9]*; do
        [ -e "$c/device/driver" ] || continue
        case "$(readlink -f "$c/device/driver")" in
            */vkms) DRM_CARD="/dev/dri/$(basename "$c")" ;;
        esac
    done
    echo "vkms: loaded, card ${DRM_CARD:-not found}"
    if [ -z "$FB_NODE" ]; then
        if n="$(new_fb vkms)"; then
            FB_NODE="/dev/$n"; FB_ID="vkms"
            echo "vkms: framebuffer emulation $FB_NODE"
        else
            echo "vkms: no new framebuffer device whose name begins with \"vkms\" appeared (no fbdev emulation in this kernel)"
        fi
    fi
else
    echo "vkms: the module is not available for kernel $(uname -r) (it is in linux-modules-extra on Ubuntu)"
fi

# uinput: virtual input devices.
if [ ! -e /dev/uinput ] && ! loaded uinput; then
    if modprobe uinput 2>/dev/null; then
        LOADED="$LOADED uinput"
        sleep 1
    fi
fi
if [ -c /dev/uinput ] && [ -w /dev/uinput ]; then
    UINPUT=1
    echo "uinput: /dev/uinput is available"
else
    echo "uinput: /dev/uinput is not available (not root, or the kernel has no uinput)"
fi

if [ -n "$FB_NODE" ]; then
    n="$(basename "$FB_NODE")"
    echo "virtual framebuffer: $FB_NODE \"$(cat /sys/class/graphics/$n/name)\" $(cat /sys/class/graphics/$n/virtual_size 2>/dev/null) $(cat /sys/class/graphics/$n/bits_per_pixel 2>/dev/null) bpp"
fi

{
    echo "FB_NODE=\"$FB_NODE\""
    echo "FB_ID=\"$FB_ID\""
    echo "DRM_CARD=\"$DRM_CARD\""
    echo "UINPUT=\"$UINPUT\""
    echo "LOADED=\"$LOADED\""
} > "$state"
cat "$state"
