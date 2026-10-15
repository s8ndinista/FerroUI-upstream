#!/bin/bash
# Counts the keys of the C library (pthread_key_create) a native library of an Android application
# creates for thread-local storage, from its symbol table (docs/porting/android-platform.md,
# section 5.1). Bionic has 128 keys for a whole process, and the virtual machine and the system
# libraries hold some of them.
#
#   scripts/android/tls-keys.sh <lib.so> [--max N] [--callers]
#
# What creates a key, on the Android targets of Rust (no thread-local statics):
#   - a `thread_local!` of the standard library or of a dependency: its storage static
#     (`<path>::{{closure}}::VAL`), one key each;
#   - the thread-local pointers of the standard library (`std::thread::current::CURRENT`, `id::ID`)
#     and the key that runs the clean-up of a thread (`guard::key::enable::CLEANUP`), one each;
#   - the table of `ferro_thread_local!` (`ferroui_base::ferro_thread_local::table::KEY`): one for
#     every thread-local variable of the framework;
#   - the emulated TLS of the compiler runtime (`emutls_pthread_key`), used by the `thread_local`
#     variables of the C and C++ code (Skia, the C++ runtime): one for all of them.
# Each is counted when the library contains it, whether or not a run reaches it: an upper bound.
#
# --max N     fail when the count is above N (default 16)
# --callers   also disassemble the library and list the functions that call pthread_key_create,
#             which have to be the three creators above (slow on a debug library)
#
# Needs FERROUI_ANDROID_TOOLCHAIN (scripts/android/env.sh) for llvm-nm and llvm-objdump, and a
# library with its symbol table (the one cargo writes, before apk.sh strips its copy).
set -euo pipefail

library=""
max=16
callers=0
while [ $# -gt 0 ]; do
    case "$1" in
        --max) max="${2:?--max needs a value}"; shift ;;
        --callers) callers=1 ;;
        -h|--help) sed -n '2,27p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) library="$1" ;;
    esac
    shift
done
if [ -z "$library" ] || [ ! -f "$library" ]; then
    echo "usage: scripts/android/tls-keys.sh <lib.so> [--max N] [--callers]" >&2
    exit 2
fi
if [ -z "${FERROUI_ANDROID_TOOLCHAIN:-}" ]; then
    repo="$(cd "$(dirname "$0")/../.." && pwd)"
    # shellcheck source=scripts/android/env.sh
    . "$repo/scripts/android/env.sh"
fi

symbols="$("$FERROUI_ANDROID_TOOLCHAIN/llvm-nm" -C "$library" 2>/dev/null | grep -E '^[0-9a-f]+ [bBdD] ' || true)"
if [ -z "$symbols" ]; then
    echo "$library has no symbol table: count on the library cargo built, before it is stripped" >&2
    exit 2
fi
keys="$(printf '%s\n' "$symbols" | sed -E 's/^[0-9a-f]+ [bBdD] //' | grep -E \
    -e '::_\$u7b\$\$u7b\$closure\$u7d\$\$u7d\$::VAL::h[0-9a-f]+$' \
    -e '::\{\{closure\}\}::VAL::h[0-9a-f]+$' \
    -e '^std::thread::current::(CURRENT|id::ID[0-9]*)::h[0-9a-f]+$' \
    -e '^std::sys::thread_local::guard::key::enable::(CLEANUP|DTORS)::h[0-9a-f]+$' \
    -e '^ferroui_base::ferro_thread_local::table::KEY::h[0-9a-f]+$' \
    -e '^emutls_pthread_key$' \
    | sed -E -e 's/::h[0-9a-f]+$//' -e 's/_\$u7b\$\$u7b\$closure\$u7d\$\$u7d\$/{{closure}}/' | sort || true)"
count="$(printf '%s\n' "$keys" | grep -c . || true)"
printf '%s\n' "$keys" | sed 's/^/  /'
echo "keys of the C library for thread-local storage: $count (at most $max allowed; the process has 128)"

if [ "$callers" = 1 ]; then
    echo "functions that call pthread_key_create:"
    "$FERROUI_ANDROID_TOOLCHAIN/llvm-objdump" -d --no-show-raw-insn -C "$library" \
        | awk '/^[0-9a-f]+ <.*>:$/ { fn = $2; gsub(/^<|>:$/, "", fn) } /bl.*<pthread_key_create@plt>/ { print "  " fn }' | sort -u
fi

if [ "$count" -gt "$max" ]; then
    echo "too many keys: a thread_local! that an Android application reaches belongs in ferro_thread_local! (docs/porting/android-platform.md, section 5.1)" >&2
    exit 1
fi
