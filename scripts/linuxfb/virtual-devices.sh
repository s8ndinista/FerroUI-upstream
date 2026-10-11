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
# A module that "down" cannot unload (something of the machine opened the new device) is written to a marker
# file that survives the state file, $marker below; the next "up" or "down" tries again before anything else,
# and both say loudly what is still loaded and what holds it. While vkms is loaded by this script a udev rule
# in /run (gone at reboot) assigns its devices to a seat of their own, so that the display manager of the
# machine does not open them; before unloading, a synthetic "remove" event tells whoever opened them anyway.
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

marker=/var/tmp/ferroui-linuxfb-modules.marker
rule=/run/udev/rules.d/00-ferroui-linuxfb-vkms.rules

# The device nodes of vkms: its card and its framebuffer emulation.
vkms_nodes() {
    for c in /sys/class/drm/card[0-9]*; do
        [ -e "$c" ] || continue
        case "$(readlink -f "$c/device")" in
            */vkms|*/vkms.[0-9]*) echo "/dev/dri/$(basename "$c")" ;;
        esac
    done
    for n in /sys/class/graphics/fb[0-9]*; do
        [ -e "$n" ] || continue
        case "$(cat "$n/name" 2>/dev/null)" in
            vkms*) echo "/dev/$(basename "$n")" ;;
        esac
    done
}

# The processes that have a node of vkms open.
holders() {
    nodes="$(vkms_nodes)"
    [ -n "$nodes" ] || return 0
    for fd in /proc/[0-9]*/fd/*; do
        target="$(readlink "$fd" 2>/dev/null)" || continue
        for node in $nodes; do
            if [ "$target" = "$node" ]; then
                pid="$(echo "$fd" | cut -d/ -f3)"
                echo "$node is open in process $pid ($(cat /proc/$pid/comm 2>/dev/null))"
            fi
        done
    done | sort -u
}

# Unloads one module this script loaded; for vkms, whoever opened its devices is told that they are going.
unload() {
    module="$1"
    [ -d "/sys/module/$module" ] || { echo "$module is not loaded"; return 0; }
    if [ "$module" = vkms ]; then
        for c in /sys/class/drm/card[0-9]*; do
            case "$(readlink -f "$c/device" 2>/dev/null)" in
                */vkms|*/vkms.[0-9]*)
                    for d in "$c"-* "$c"; do [ -e "$d" ] && udevadm trigger --action=remove "$d" 2>/dev/null; done ;;
            esac
        done
        sleep 2
    fi
    for attempt in 1 2 3; do
        if modprobe -r "$module" 2>/dev/null; then
            echo "unloaded $module"
            return 0
        fi
        sleep 1
    done
    echo "COULD NOT UNLOAD $module: $(cat /sys/module/$module/refcnt 2>/dev/null) references; it stays loaded until it is released or the machine restarts"
    if [ "$module" = vkms ]; then
        h="$(holders)"
        if [ -n "$h" ]; then
            echo "$h"
        else
            echo "no process has its devices open: the holder is in the kernel (the framebuffer console, if it took the new device: $(cat /proc/fb 2>/dev/null | tr '\n' ';'))"
        fi
    fi
    return 1
}

# Modules an earlier run could not unload.
retry_marker() {
    [ -f "$marker" ] || return 0
    left=""
    for module in $(cat "$marker"); do
        echo "left by an earlier run: $module"
        unload "$module" || left="$left $module"
    done
    if [ -n "$left" ]; then echo "$left" > "$marker"; else rm -f "$marker"; fi
}

fb_nodes() { ls /sys/class/graphics 2>/dev/null | grep '^fb[0-9][0-9]*$' | sort; }
loaded() { [ -d "/sys/module/$1" ]; }

if [ "$action" = down ]; then
    LOADED=""
    [ -f "$state" ] && . "$state"
    retry_marker
    left="$(cat "$marker" 2>/dev/null)"
    # In the reverse order of loading.
    for module in uinput vkms vfb; do
        case " $LOADED " in
            *" $module "*) unload "$module" || left="$left $module" ;;
        esac
    done
    rm -f "$state"
    if [ -f "$rule" ]; then
        rm -f "$rule"
        udevadm control --reload 2>/dev/null
    fi
    if [ -n "$left" ]; then
        echo "$left" > "$marker"
        echo "final state: STILL LOADED:$left (recorded in $marker; the next run tries again)"
        exit 1
    fi
    rm -f "$marker"
    echo "final state: nothing of this script is loaded (vfb, vkms and uinput are as the machine had them)"
    exit 0
fi

retry_marker
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
if loaded vkms && grep -qw vkms "$marker" 2>/dev/null; then
    # Loaded by an earlier run of this script, which could not unload it (the marker is the proof): its devices
    # are the script's own and are used; "down" tries to unload it again.
    for node in $(vkms_nodes); do
        case "$node" in
            /dev/dri/*) DRM_CARD="$node" ;;
            /dev/fb*) if [ -z "$FB_NODE" ]; then FB_NODE="$node"; FB_ID="vkms"; fi ;;
        esac
    done
    echo "vkms: still loaded from an earlier run of this script (see $marker): its devices are used: card ${DRM_CARD:-not found}, framebuffer ${FB_NODE:-not found}"
elif loaded vkms; then
    echo "vkms: already loaded, left alone"
elif {
    # The devices of vkms get a seat of their own, so that the display manager of the machine leaves them alone.
    mkdir -p "$(dirname "$rule")" 2>/dev/null
    printf '%s\n' 'SUBSYSTEM=="drm", DEVPATH=="*/vkms*", ENV{ID_SEAT}="seat-ferroui-vkms", TAG+="mutter-device-ignore"' \
        'SUBSYSTEM=="graphics", DEVPATH=="*/vkms*", ENV{ID_SEAT}="seat-ferroui-vkms"' > "$rule" 2>/dev/null
    udevadm control --reload 2>/dev/null
    modprobe vkms 2>/dev/null
}; then
    LOADED="$LOADED vkms"
    sleep 1
    for c in /sys/class/drm/card[0-9]*; do
        # The device of the card is the platform (or faux) device "vkms".
        case "$(readlink -f "$c/device")" in
            */vkms|*/vkms.[0-9]*) DRM_CARD="/dev/dri/$(basename "$c")" ;;
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
