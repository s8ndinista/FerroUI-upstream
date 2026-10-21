#!/usr/bin/env python3
"""Inventory the public Charts contract and validate its explicit Rust test mapping.

This checks provenance and mapping integrity. It does not execute Rust tests or
claim visual parity with the unavailable internal Charts implementation.
"""

from __future__ import annotations

import argparse
from collections import Counter
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "docs/porting/data/winui-charts"
REPOSITORY = "https://github.com/microsoft/microsoft-ui-xaml"
REVISION = "9427b7216337ea3d7b19ac32d48c327286ee7138"
PREFIXES = ("controls/dev/Charts", "Samples/ChartApp")
TEST_RE = re.compile(
    r"\[TestMethod\]\s*(?:\[[^\]]*\]\s*)*"
    r"public\s+(?:async\s+)?(?:void|Task)\s+(\w+)\s*\("
)
RUST_TEST_RE = re.compile(
    r"#\[test\]\s*(?:#\[[^\]]*\]\s*)*"
    r"(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)\s*\("
)
STATUSES = {"pending", "mapped", "adapted", "not-applicable"}


def git(upstream: Path, *arguments: str) -> bytes:
    return subprocess.check_output(["git", "-C", str(upstream), *arguments])


def read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def snake_case(value: str) -> str:
    value = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", value)
    return re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", value).lower()


def role(path: str) -> str:
    if "/APITests/" in path and path.endswith("Tests.cs"):
        return "api-tests"
    if "/InteractionTests/" in path and path.endswith("Tests.cs"):
        return "interaction-tests"
    if path.endswith("README.md"):
        return "source-availability-documentation"
    if "/Assets/" in path:
        return "windows-host-assets"
    if path.endswith((".cs", ".cpp", ".h", ".xaml", ".idl")):
        return "test-support" if path.startswith("controls/") else "engineering-sample"
    return "windows-build-and-deployment"


def proposed_target(path: str) -> list[str]:
    names = {
        "ChartTests.cs": ["chart_tests.rs", "markup_tests.rs"],
        "ChartAxisTests.cs": ["axis_tests.rs"],
        "ChartSeriesTests.cs": ["series_tests.rs"],
        "ChartSamplesTests.cs": ["samples_tests.rs"],
        "ChartAutomationTests.cs": ["automation_tests.rs"],
        "ChartsTests.cs": ["chart_tests.rs"],
    }
    return ["src/FerroUI.Controls.Charts/" + p for p in names.get(Path(path).name, [])]


def inventory(upstream: Path) -> tuple[dict, list[dict]]:
    # Reading the pinned tree (rather than HEAD/the worktree) makes this reproducible.
    tree = git(upstream, "ls-tree", "-rz", REVISION, "--", *PREFIXES)
    files = []
    tests = []
    for entry in tree.split(b"\0"):
        if not entry:
            continue
        attributes, raw_path = entry.split(b"\t", 1)
        mode, kind, sha = attributes.decode().split()
        path = raw_path.decode()
        if kind != "blob":
            raise ValueError(f"Unexpected non-file source entry: {path}")
        category = role(path)
        files.append({
            "path": path,
            "blob_sha": sha,
            "mode": mode,
            "role": category,
            "proposed_rust_targets": proposed_target(path),
        })
        if category in {"api-tests", "interaction-tests"}:
            content = git(upstream, "show", f"{REVISION}:{path}").decode("utf-8-sig")
            for match in TEST_RE.finditer(content):
                name = match.group(1)
                tests.append({
                    "id": f"{path}::{name}",
                    "upstream_file": path,
                    "upstream_method": name,
                    "upstream_line": content[:match.start(1)].count("\n") + 1,
                    "suite": category,
                    "suggested_rust_test": snake_case(name),
                })
    files.sort(key=lambda item: item["path"])
    tests.sort(key=lambda item: item["id"])
    manifest = {
        "schema_version": 1,
        "repository": REPOSITORY,
        "revision": REVISION,
        "discussion": REPOSITORY + "/discussions/12343",
        "implementation_source": {
            "availability": "not-published-in-pinned-public-tree",
            "evidence": "Samples/ChartApp/README.md",
            "package_named_by_upstream": "Microsoft.Internal.WinUIDetails",
        },
        "scope_roots": list(PREFIXES),
        "file_count": len(files),
        "test_count": len(tests),
        "test_suite_counts": dict(sorted(Counter(t["suite"] for t in tests).items())),
        "files": files,
        "tests": tests,
    }
    return manifest, tests


def generate(upstream: Path, check: bool) -> int:
    manifest, tests = inventory(upstream)
    destination = DATA / "source-manifest.json"
    if check:
        if not destination.exists() or read_json(destination) != manifest:
            print("Source manifest differs from the pinned upstream tree.", file=sys.stderr)
            return 1
        print(f"Pinned inventory verified: {len(manifest['files'])} files, {len(tests)} tests.")
        return 0
    write_json(destination, manifest)
    mapping_path = DATA / "test-map.json"
    previous = read_json(mapping_path) if mapping_path.exists() else {}
    old = {item["id"]: item for item in previous.get("tests", [])}
    mapping = {
        "schema_version": 1,
        "revision": REVISION,
        "meaning": "Mapping integrity only; mapped does not mean executed or pixel-equivalent.",
        "tests": [old.get(t["id"], {
            "id": t["id"], "status": "pending", "rust_tests": [], "notes": "",
        }) for t in tests],
    }
    write_json(mapping_path, mapping)
    print(f"Generated inventory: {len(manifest['files'])} files, {len(tests)} tests.")
    return 0


def local_test_functions() -> dict[str, set[str]]:
    crate = ROOT / "src/FerroUI.Controls.Charts"
    return {
        p.relative_to(ROOT).as_posix(): set(RUST_TEST_RE.findall(p.read_text(encoding="utf-8")))
        for p in sorted(crate.rglob("*.rs"))
    }


def bind_names() -> int:
    """Associate exact translated names; leave adaptation review and notes explicit."""
    manifest = read_json(DATA / "source-manifest.json")
    mapping = read_json(DATA / "test-map.json")
    candidates = local_test_functions()
    names = {t["id"]: t["suggested_rust_test"] for t in manifest["tests"]}
    bound = 0
    for entry in mapping["tests"]:
        if entry["status"] != "pending":
            continue
        name = names[entry["id"]]
        matches = [{"file": file, "function": name} for file, functions in candidates.items() if name in functions]
        if matches:
            entry.update(status="mapped", rust_tests=matches,
                         notes="Exact translated test name; execution is recorded separately.")
            bound += 1
    write_json(DATA / "test-map.json", mapping)
    print(f"Associated {bound} exact translated test names.")
    return 0


def validate(upstream: Path | None, require_complete: bool) -> int:
    errors = []
    if upstream is not None and generate(upstream, check=True):
        errors.append("Pinned source manifest verification failed")
    manifest = read_json(DATA / "source-manifest.json")
    mapping = read_json(DATA / "test-map.json")
    if manifest["revision"] != REVISION or mapping["revision"] != REVISION:
        errors.append("Manifest/mapping revision does not match the scanner's pinned revision")
    source_ids = [t["id"] for t in manifest["tests"]]
    map_ids = [t["id"] for t in mapping["tests"]]
    if len(source_ids) != len(set(source_ids)) or len(map_ids) != len(set(map_ids)):
        errors.append("Duplicate source or mapping test IDs")
    missing = sorted(set(source_ids) - set(map_ids))
    unknown = sorted(set(map_ids) - set(source_ids))
    errors.extend(f"Untracked upstream test: {item}" for item in missing)
    errors.extend(f"Unknown upstream test: {item}" for item in unknown)
    if manifest["test_count"] != len(source_ids) or manifest["file_count"] != len(manifest["files"]):
        errors.append("Manifest counts do not match its entries")
    source_files = {f["path"]: f for f in manifest["files"]}
    for item in manifest["tests"]:
        if item["upstream_file"] not in source_files:
            errors.append(f"Test references a file absent from source inventory: {item['id']}")
        if item["id"] != item["upstream_file"] + "::" + item["upstream_method"]:
            errors.append(f"Inconsistent source method identity: {item['id']}")
    rust_tests = local_test_functions()
    for item in mapping["tests"]:
        status = item["status"]
        if status not in STATUSES:
            errors.append(f"Unknown mapping status {status}: {item['id']}")
        if status in {"mapped", "adapted"} and not item["rust_tests"]:
            errors.append(f"Mapped test has no Rust test: {item['id']}")
        if status in {"adapted", "not-applicable"} and not item["notes"].strip():
            errors.append(f"Adaptation requires a rationale: {item['id']}")
        if status == "pending" and item["rust_tests"]:
            errors.append(f"Pending test unexpectedly claims a Rust mapping: {item['id']}")
        for target in item["rust_tests"]:
            file, function = target["file"], target["function"]
            if file not in rust_tests:
                errors.append(f"Mapped Rust file does not exist: {file} ({item['id']})")
            elif function not in rust_tests[file]:
                errors.append(f"Mapped #[test] function does not exist: {file}::{function}")
    counts = Counter(t["status"] for t in mapping["tests"])
    if require_complete and counts["pending"]:
        errors.append(f"Incomplete mapping: {counts['pending']} upstream tests remain pending")
    print(f"Upstream tests: {len(source_ids)}; " + "; ".join(f"{status}: {counts[status]}" for status in sorted(STATUSES)))
    print("This command validates mapping integrity; run cargo test for execution evidence.")
    for error in errors:
        print("ERROR: " + error, file=sys.stderr)
    return bool(errors)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    inventory_parser = commands.add_parser("inventory", help="Generate or verify pinned source inventory")
    inventory_parser.add_argument("--upstream", type=Path, required=True)
    inventory_parser.add_argument("--check", action="store_true")
    commands.add_parser("bind-names", help="Map Rust tests with exact translated upstream names")
    check_parser = commands.add_parser("check", help="Validate source and Rust test mapping integrity")
    check_parser.add_argument("--upstream", type=Path)
    check_parser.add_argument("--require-complete", action="store_true")
    args = parser.parse_args()
    try:
        if args.command == "inventory":
            return generate(args.upstream, args.check)
        if args.command == "bind-names":
            return bind_names()
        return validate(args.upstream, args.require_complete)
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(f"Conformance error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
