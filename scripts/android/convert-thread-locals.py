#!/usr/bin/env python3
"""Declares the thread-local variables of the workspace with `ferro_thread_local!`.

The Android targets of Rust keep every `thread_local!` under a key of the C library of its own, and
bionic has 128 of them for a process; `ferro_thread_local!` of ferroui-system (which ferroui-base
re-exports at the path it had there) is `thread_local!` on every other target and a slot of one
table per thread on Android (docs/porting/android-platform.md, section 5.1). This script rewrites
every `thread_local!` (also written `std::thread_local!` or `::std::thread_local!`) of the crates of
the workspace, and the types `std::thread::LocalKey` and `std::thread::AccessError` where a signature
names them:

- in ferroui-system and ferroui-base, `::std::thread_local!` (the form their exported macros use)
  becomes `$crate::ferro_thread_local!` and any other form `crate::ferro_thread_local!`, a type
  `crate::ferro_thread_local::LocalKey` (or `AccessError`);
- in a crate that depends on ferroui-base, `ferroui_base::ferro_thread_local!` and
  `ferroui_base::ferro_thread_local::LocalKey` (or `AccessError`);
- in a crate that depends on ferroui-system but not on ferroui-base (the reusable libraries),
  `ferroui_system::ferro_thread_local!` and `ferroui_system::ferro_thread_local::LocalKey` (or
  `AccessError`).

Only the name of the macro changes: the declarations keep their grammar. Running it again changes
nothing. A crate that has a `thread_local!` and depends on neither is reported and left alone; it
has to be on the allow-list below (with the reason) or be changed by hand.

    scripts/android/convert-thread-locals.py            rewrite, and list what changed
    scripts/android/convert-thread-locals.py --check    change nothing; fail when a `thread_local!`
                                                        is left outside the allow-list (the
                                                        conventions job of CI runs this)
"""

import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SCANNED = ["src", "samples", "tests", "external"]
SKIPPED_DIRECTORIES = {"target", ".git", "node_modules", ".claude"}

# Files and directories (relative to the repository, with forward slashes) that keep `thread_local!`.
ALLOWED = {
    # The definition of the macro: it expands to `std::thread_local!` on targets with thread-local
    # statics, and its documentation names it.
    "src/FerroUI.System/ferro_thread_local.rs": "the definition of ferro_thread_local!",
    "src/FerroUI.System/ferro_thread_local/": "the table, which has the methods of the standard library",
    # The tests of the table compare its errors with those of a variable of the standard library.
    "src/FerroUI.System/tests/ferro_thread_local_tests.rs": "compares the table with the standard library",
    # XamlX is a port of a third-party compiler of its own, with no dependency on the framework. It
    # runs where markup is compiled: the build scripts on the development machine, and the loader of
    # markup at run time, which no Android application of the repository links. Where one would, its
    # two variables are one key of the C library each.
    "external/XamlX/": "a third-party compiler without a dependency on ferroui-base",
}

PATTERN = re.compile(r"(?<![\w:$])((?:::)?std::)?thread_local!")
# The type of a variable and the error of `try_with`, named in a signature: those of
# `ferroui_base::ferro_thread_local`, which are the standard library's where the macro is
# `thread_local!` and the table's on Android.
TYPE_PATTERN = re.compile(r"(?<![\w:$])(?:::)?std::thread::(LocalKey|AccessError)\b")


def is_allowed(rel):
    return any(rel == entry or (entry.endswith("/") and rel.startswith(entry)) for entry in ALLOWED)


def crate_of(file_path):
    """The directory of the manifest the file belongs to, and the manifest's text."""
    directory = os.path.dirname(file_path)
    while directory.startswith(ROOT):
        manifest = os.path.join(directory, "Cargo.toml")
        if os.path.exists(manifest):
            with open(manifest, encoding="utf-8") as f:
                return directory, f.read()
        directory = os.path.dirname(directory)
    return None, ""


def package_name(manifest):
    match = re.search(r'^\[package\][^\[]*?^name\s*=\s*"([^"]+)"', manifest, re.M | re.S)
    return match.group(1) if match else None


def depends_on_base(manifest):
    return re.search(r"^ferroui-base\s*(=|\.)", manifest, re.M) is not None


def depends_on_system(manifest):
    return re.search(r"^ferroui-system\s*(=|\.)", manifest, re.M) is not None


def code_matches(text):
    """The matches of both patterns outside of line comments, in the order of the text."""
    matches = list(PATTERN.finditer(text)) + list(TYPE_PATTERN.finditer(text))
    for match in sorted(matches, key=lambda m: m.start()):
        line_start = text.rfind("\n", 0, match.start()) + 1
        if "//" in text[line_start:match.start()]:
            continue
        yield match


def rust_files():
    for top in SCANNED:
        for directory, directories, files in os.walk(os.path.join(ROOT, top)):
            directories[:] = sorted(d for d in directories if d not in SKIPPED_DIRECTORIES)
            for name in sorted(files):
                if name.endswith(".rs"):
                    full = os.path.join(directory, name)
                    yield full, os.path.relpath(full, ROOT).replace(os.sep, "/")


def main():
    check = "--check" in sys.argv[1:]
    changed = {}
    left = []
    for full, rel in rust_files():
        if is_allowed(rel):
            continue
        with open(full, encoding="utf-8") as f:
            text = f.read()
        matches = list(code_matches(text))
        if not matches:
            continue
        if check:
            for match in matches:
                line = text.count("\n", 0, match.start()) + 1
                left.append(f"{rel}:{line}: {match.group(0)}")
            continue
        _, manifest = crate_of(full)
        name = package_name(manifest)
        if name in ("ferroui-system", "ferroui-base"):
            def replacement(match):
                if match.re is TYPE_PATTERN:
                    return f"crate::ferro_thread_local::{match.group(1)}"
                return "$crate::ferro_thread_local!" if match.group(1) == "::std::" else "crate::ferro_thread_local!"
        elif depends_on_base(manifest):
            def replacement(match):
                if match.re is TYPE_PATTERN:
                    return f"ferroui_base::ferro_thread_local::{match.group(1)}"
                return "ferroui_base::ferro_thread_local!"
        elif depends_on_system(manifest):
            def replacement(match):
                if match.re is TYPE_PATTERN:
                    return f"ferroui_system::ferro_thread_local::{match.group(1)}"
                return "ferroui_system::ferro_thread_local!"
        else:
            for match in matches:
                line = text.count("\n", 0, match.start()) + 1
                left.append(f"{rel}:{line} (the crate {name} depends on neither ferroui-system nor ferroui-base)")
            continue
        pieces = []
        last = 0
        for match in matches:
            pieces.append(text[last:match.start()])
            pieces.append(replacement(match))
            last = match.end()
        pieces.append(text[last:])
        with open(full, "w", encoding="utf-8") as f:
            f.write("".join(pieces))
        changed[rel] = len(matches)

    if check:
        if left:
            for entry in left:
                print(f"{entry} (declare thread-local variables with ferro_thread_local! of ferroui-system)")
            print(
                f"{len(left)} uses of thread_local! or its types outside of the allow-list of "
                "scripts/android/convert-thread-locals.py; "
                "see docs/porting/android-platform.md, section 5.1",
                file=sys.stderr,
            )
            return 1
        print("no thread_local! outside of the allow-list")
        return 0

    per_crate = {}
    for rel, count in changed.items():
        directory, _ = crate_of(os.path.join(ROOT, rel))
        crate = os.path.relpath(directory, ROOT).replace(os.sep, "/")
        per_crate[crate] = per_crate.get(crate, 0) + count
    for crate, count in sorted(per_crate.items(), key=lambda item: (-item[1], item[0])):
        print(f"{count:5} {crate}")
    print(f"{sum(changed.values()):5} in {len(changed)} files")
    for entry in left:
        print(f"left: {entry}", file=sys.stderr)
    return 1 if left else 0


if __name__ == "__main__":
    sys.exit(main())
