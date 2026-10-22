#!/usr/bin/env python3
"""Preserve system MSFTEDIT probe observations with their CI/engine provenance.

This extracts the Windows-only probe in rich_text_tom_oracle_tests.rs. It does
not compare the observations with FerroUI or declare WinUI engine conformance.
Unknown probe rows, incomplete runs and contradictory setup are rejected.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path


PREFIX = "SYSTEM_MSFTEDIT_"
STATE = r"SelectionState \{ start: (-?\d+), end: (-?\d+), flags: (\d+) \}"
ENGINE = re.compile(
    r'ENGINE windows=\((\d+), (\d+), (\d+)\) dll=("(?:\\.|[^"\\])*") '
    r"file_version=(\d+\.\d+\.\d+\.\d+)"
)
FIXTURE = re.compile(
    r'(VERTICAL_|HORIZONTAL_|OPTIONS_)?FIXTURE name=(\S+) text=("(?:\\.|[^"\\])*") '
    r"(?:supplied_)?utf16_length=(\d+) native_story_length=(\d+)"
    r"(?: window_ex_style=(\d+))?"
)
HOME_END = re.compile(
    r"HOME_END fixture=(\S+) supplied_utf16_length=(\d+) native_story_length=(\d+) "
    r"unit=(\w+) extend=(true|false) operation=(Home|End) before="
    + STATE + r" after=" + STATE
    + r" anchor=(-?\d+) active=(-?\d+) delta=(-?\d+) active_displacement=(-?\d+) "
    r"hresult=(0x[0-9A-F]{8})"
)
REPEAT = re.compile(
    r"HOME_END_REPEAT fixture=(\S+) unit=(\w+) extend=(true|false) "
    r"operation=(Home|End) after=" + STATE
    + r" delta=(-?\d+) hresult=(0x[0-9A-F]{8})"
)
VERTICAL = re.compile(
    r"VERTICAL fixture=(\S+) unit=(\w+) count=(-?\d+) extend=(true|false) "
    r"operation=(Up|Down) before=" + STATE + r" after=" + STATE
    + r" anchor=(-?\d+) active=(-?\d+) delta=(-?\d+) active_displacement=(-?\d+) "
    r"hresult=(0x[0-9A-F]{8})"
)
HORIZONTAL = re.compile(
    r"HORIZONTAL fixture=(\S+) unit=(\w+) count=(-?\d+) extend=(true|false) "
    r"operation=(Left|Right) before=" + STATE + r" before_type=(-?\d+) after="
    + STATE + r" after_type=(-?\d+) anchor=(-?\d+) active=(-?\d+) "
    r"delta=(-?\d+) active_displacement=(-?\d+) hresult=(0x[0-9A-F]{8})"
)
OPTIONS = re.compile(
    r"OPTIONS fixture=(\S+) requested_flags=(\d+) before=" + STATE
    + r" before_type=(-?\d+) after_set=" + STATE
    + r" after_set_type=(-?\d+) set_hresult=(0x[0-9A-F]{8}) "
    r'typed=("(?:\\.|[^"\\])*") after=' + STATE
    + r" after_type=(-?\d+) type_hresult=(0x[0-9A-F]{8}) "
    r"native_story_length_after=(\d+) "
    r'story_text=("(?:\\.|[^"\\])*") story_text_utf16_length=(\d+)'
)


def rust_string(encoded: str) -> str:
    """Decode a Rust Debug string, including non-printing Unicode scalars.

    Rust writes a ZWJ as a braced Unicode escape rather than JSON's fixed-width
    form. Walk escapes in order so a literal backslash followed by 'u' stays
    literal, and reject malformed/unknown escapes rather than losing text.
    """
    if not (encoded.startswith('"') and encoded.endswith('"')):
        raise ValueError("fixture string is not quoted")
    escapes = {'"': '"', "\\": "\\", "n": "\n", "r": "\r", "t": "\t", "0": "\0"}
    result = []
    index = 1
    end = len(encoded) - 1
    while index < end:
        char = encoded[index]
        index += 1
        if char != "\\":
            if char == '"' or ord(char) < 0x20 or 0xD800 <= ord(char) <= 0xDFFF:
                raise ValueError("invalid literal in fixture string")
            result.append(char)
            continue
        if index >= end:
            raise ValueError("unterminated fixture escape")
        escape = encoded[index]
        index += 1
        if escape in escapes:
            result.append(escapes[escape])
        elif escape == "u" and index < end and encoded[index] == "{":
            closing = encoded.find("}", index + 1, end)
            digits = encoded[index + 1:closing] if closing >= 0 else ""
            if not re.fullmatch(r"[0-9a-fA-F]{1,6}", digits):
                raise ValueError("invalid fixture Unicode escape")
            value = int(digits, 16)
            if value > 0x10FFFF or 0xD800 <= value <= 0xDFFF:
                raise ValueError("fixture escape is not a Unicode scalar")
            result.append(chr(value))
            index = closing + 1
        else:
            raise ValueError("unsupported fixture string escape")
    return "".join(result)


def state(values: tuple[str, ...]) -> dict:
    start, end, flags = map(int, values)
    if not 0 <= start <= end:
        raise ValueError("invalid native normalized range")
    return {
        "start": start,
        "end": end,
        "flags": flags,
        "anchor": end if flags & 1 else start,
        "active": start if flags & 1 else end,
    }


def movement(values: tuple[str, ...], state_start: int) -> dict:
    before = state(values[state_start:state_start + 3])
    after = state(values[state_start + 3:state_start + 6])
    anchor, active, delta, displacement = map(int, values[state_start + 6:state_start + 10])
    if (anchor, active) != (after["anchor"], after["active"]):
        raise ValueError("logged directed endpoints disagree with native flags")
    if displacement != after["active"] - before["active"]:
        raise ValueError("logged displacement disagrees with native endpoints")
    return {
        "before": before,
        "after": after,
        "delta": delta,
        "active_displacement": displacement,
        "hresult": values[state_start + 10],
    }


def extract(log: str) -> dict:
    engine = None
    fixtures = []
    home_end = []
    vertical = []
    horizontal = []
    options = []
    completed = {}
    for line_number, line in enumerate(log.splitlines(), 1):
        marker = line.find(PREFIX)
        if marker < 0:
            continue
        row = line[marker + len(PREFIX):].rstrip()
        match = ENGINE.fullmatch(row)
        if match:
            current = {
                "windows_version": list(map(int, match.group(1, 2, 3))),
                "dll_path": json.loads(match[4]),
                "dll_file_version": match[5],
            }
            if engine is not None and engine != current:
                raise ValueError("engine changed within one probe record")
            engine = current
            continue
        match = FIXTURE.fullmatch(row)
        if match:
            text = rust_string(match[3])
            if len(text.encode("utf-16-le")) // 2 != int(match[4]):
                raise ValueError("fixture text disagrees with supplied UTF-16 length")
            fixture = {
                "family": {
                    None: "home_end",
                    "VERTICAL_": "vertical",
                    "HORIZONTAL_": "horizontal",
                    "OPTIONS_": "options",
                }[match[1]],
                "name": match[2],
                "text": text,
                "supplied_utf16_length": int(match[4]),
                "native_story_length": int(match[5]),
            }
            if fixture["native_story_length"] < fixture["supplied_utf16_length"]:
                raise ValueError("native story is shorter than its supplied text")
            if fixture["family"] in ("horizontal", "options"):
                if match[6] is None or int(match[6]) > 0xFFFFFFFF:
                    raise ValueError("horizontal/options fixture needs its native u32 window style")
                fixture["window_ex_style"] = int(match[6])
            elif match[6] is not None:
                raise ValueError("unexpected window style outside horizontal/options fixture")
            fixtures.append(fixture)
            continue
        match = HOME_END.fullmatch(row)
        if match:
            values = match.groups()
            home_end.append({
                "fixture": values[0],
                "supplied_utf16_length": int(values[1]),
                "native_story_length": int(values[2]),
                "unit": values[3],
                "extend": values[4] == "true",
                "operation": values[5],
                **movement(values, 6),
            })
            continue
        match = REPEAT.fullmatch(row)
        if match:
            values = match.groups()
            if not home_end:
                raise ValueError("repeat precedes initial Home/End command")
            previous = home_end[-1]
            identity = (values[0], values[1], values[2] == "true", values[3])
            if identity != tuple(previous[k] for k in ("fixture", "unit", "extend", "operation")):
                raise ValueError("repeat does not match its initial Home/End command")
            if "repeat" in previous:
                raise ValueError("duplicate Home/End repeat")
            previous["repeat"] = {
                "after": state(values[4:7]),
                "delta": int(values[7]),
                "hresult": values[8],
            }
            continue
        match = VERTICAL.fullmatch(row)
        if match:
            values = match.groups()
            vertical.append({
                "fixture": values[0],
                "unit": values[1],
                "count": int(values[2]),
                "extend": values[3] == "true",
                "operation": values[4],
                **movement(values, 5),
            })
            continue
        match = HORIZONTAL.fullmatch(row)
        if match:
            values = match.groups()
            # Reuse exact directed-state/displacement validation while keeping
            # the two native selection types adjacent to their own states.
            moved = movement(values[:8] + values[9:12] + values[13:], 5)
            moved["before"]["selection_type"] = int(values[8])
            moved["after"]["selection_type"] = int(values[12])
            horizontal.append({
                "fixture": values[0],
                "unit": values[1],
                "count": int(values[2]),
                "extend": values[3] == "true",
                "operation": values[4],
                **moved,
            })
            continue
        match = OPTIONS.fullmatch(row)
        if match:
            values = match.groups()
            before, after_set, after = (state(values[start:start + 3]) for start in (2, 6, 12))
            before["selection_type"] = int(values[5])
            after_set["selection_type"] = int(values[9])
            after["selection_type"] = int(values[15])
            flags = int(values[1])
            if flags > 31:
                raise ValueError("options probe requests undocumented flags")
            text = rust_string(values[18])
            text_length = int(values[19])
            native_length = int(values[17])
            if len(text.encode("utf-16-le")) // 2 != text_length or text_length > native_length:
                raise ValueError("typed story disagrees with its native UTF-16 length")
            if after["end"] > native_length:
                raise ValueError("typing result exceeds its changed native story")
            options.append({
                "fixture": values[0],
                "requested_flags": flags,
                "before": before,
                "after_set": after_set,
                "set_hresult": values[10],
                "typed": rust_string(values[11]),
                "after": after,
                "type_hresult": values[16],
                "native_story_length_after": native_length,
                "story_text": text,
                "story_text_utf16_length": text_length,
            })
            continue
        match = re.fullmatch(r"PROBE_COMPLETE commands=(\d+) repeated_noops=(\d+)", row)
        if match:
            if "home_end" in completed:
                raise ValueError("duplicate Home/End completion")
            completed["home_end"] = (int(match[1]), int(match[2]))
            continue
        match = re.fullmatch(r"VERTICAL_PROBE_COMPLETE commands=(\d+)", row)
        if match:
            if "vertical" in completed:
                raise ValueError("duplicate vertical completion")
            completed["vertical"] = int(match[1])
            continue
        match = re.fullmatch(r"HORIZONTAL_PROBE_COMPLETE commands=(\d+)", row)
        if match:
            if "horizontal" in completed:
                raise ValueError("duplicate horizontal completion")
            completed["horizontal"] = int(match[1])
            continue
        match = re.fullmatch(r"OPTIONS_PROBE_COMPLETE commands=(\d+)", row)
        if match:
            if "options" in completed:
                raise ValueError("duplicate options completion")
            completed["options"] = int(match[1])
            continue
        raise ValueError(f"unrecognized probe row on log line {line_number}: {row}")

    if engine is None or not completed:
        raise ValueError("no completed native probe with engine identity")
    if home_end or "home_end" in completed:
        counts = (len(home_end), sum("repeat" in row for row in home_end))
        if counts != completed.get("home_end") or counts[0] == 0:
            raise ValueError("Home/End observations do not match the completion counts")
    if vertical or "vertical" in completed:
        if len(vertical) != completed.get("vertical") or not vertical:
            raise ValueError("vertical observations do not match the completion count")
    if horizontal or "horizontal" in completed:
        if len(horizontal) != completed.get("horizontal") or not horizontal:
            raise ValueError("horizontal observations do not match the completion count")
    if options or "options" in completed:
        if len(options) != completed.get("options") or not options:
            raise ValueError("options observations do not match the completion count")
    by_fixture = {}
    for fixture in fixtures:
        identity = (fixture["family"], fixture["name"])
        if identity in by_fixture:
            raise ValueError("duplicate fixture identity")
        if fixture["family"] not in completed:
            raise ValueError("declared fixture family has no completed probe")
        by_fixture[identity] = fixture
    used_fixtures = set()
    for family, rows in (
        ("home_end", home_end), ("vertical", vertical), ("horizontal", horizontal),
        ("options", options),
    ):
        for row in rows:
            fixture = by_fixture.get((family, row["fixture"]))
            if fixture is None:
                raise ValueError("command has no exact text fixture")
            used_fixtures.add((family, row["fixture"]))
            for key in ("supplied_utf16_length", "native_story_length"):
                if key in row and row[key] != fixture[key]:
                    raise ValueError("command lengths disagree with its exact text fixture")
            states = [row["before"], row["after_set"] if family == "options" else row["after"]]
            if "repeat" in row:
                states.append(row["repeat"]["after"])
            if any(value["end"] > fixture["native_story_length"] for value in states):
                raise ValueError("command range exceeds its native story")
    if used_fixtures != by_fixture.keys():
        raise ValueError("declared fixture has no observed command")
    result = {
        "engine": engine,
        "fixtures": fixtures,
        "home_end": home_end,
        "vertical": vertical,
        "counts": {
            "home_end": len(home_end),
            "home_end_repeats": sum("repeat" in row for row in home_end),
            "vertical": len(vertical),
        },
    }
    # Keep regeneration of the earlier versioned records byte-identical.
    # The optional horizontal family is present only after an executed probe.
    if horizontal:
        result["horizontal"] = horizontal
        result["counts"]["horizontal"] = len(horizontal)
    if options:
        result["options"] = options
        result["counts"]["options"] = len(options)
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--log", required=True, type=Path)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--probe-blob", required=True)
    parser.add_argument("--run-url", required=True)
    parser.add_argument("--job-url", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    for value in (args.commit, args.probe_blob):
        if not re.fullmatch(r"[0-9a-f]{40}", value):
            parser.error("commit and probe blob must be full lowercase Git object IDs")
    observations = extract(args.log.read_text(encoding="utf-8"))
    result = {
        "schema_version": 1,
        "scope": "System MSFTEDIT characterization; not the external WinUI text engine.",
        "provenance": {
            "commit": args.commit,
            "probe_path": "src/Windows/FerroUI.Win32/rich_text_tom_oracle_tests.rs",
            "probe_git_blob": args.probe_blob,
            "run_url": args.run_url,
            "job_url": args.job_url,
            "command": "cargo test --locked -p ferroui-win32 --no-default-features --lib rich_text_tom_oracle -- --nocapture --test-threads=1",
        },
        **observations,
    }
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"output": str(args.output), "counts": observations["counts"]}))


if __name__ == "__main__":
    main()
