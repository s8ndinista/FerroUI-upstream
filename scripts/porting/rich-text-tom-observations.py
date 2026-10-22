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
DEBUG_STRING = r'"(?:\\.|[^"\\])*"'
STATE = r"SelectionState \{ start: (-?\d+), end: (-?\d+), flags: (\d+) \}"
ENGINE = re.compile(
    r'ENGINE windows=\((\d+), (\d+), (\d+)\) dll=("(?:\\.|[^"\\])*") '
    r"file_version=(\d+\.\d+\.\d+\.\d+)"
)
FIXTURE = re.compile(
    r'(VERTICAL_|HORIZONTAL_|OPTIONS_|LINK_)?FIXTURE name=(\S+) text=("(?:\\.|[^"\\])*") '
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


def link_snapshot_pattern(prefix: str) -> str:
    return (
        rf"LinkSnapshot \{{ range: RangeState \{{ start: (?P<{prefix}_start>-?\d+), "
        rf"end: (?P<{prefix}_end>-?\d+) \}}, "
        rf"range_text: (?P<{prefix}_range_text>{DEBUG_STRING}), "
        rf"range_text_utf16_length: (?P<{prefix}_range_text_utf16_length>\d+), "
        rf"story_text: (?P<{prefix}_story_text>{DEBUG_STRING}), "
        rf"story_text_utf16_length: (?P<{prefix}_story_text_utf16_length>\d+), "
        rf"story_length: (?P<{prefix}_story_length>\d+), "
        rf"selection: SelectionState \{{ start: (?P<{prefix}_selection_start>-?\d+), "
        rf"end: (?P<{prefix}_selection_end>-?\d+), flags: (?P<{prefix}_flags>\d+) \}} \}}"
    )


LINK = re.compile(
    r"LINK fixture=(?P<fixture>\S+) step=(?P<step>\d+) "
    r"role=(?P<role>Setup|Target|Readback) operation=(?P<operation>Get|Set) "
    r"initial_start=(?P<initial_start>-?\d+) initial_end=(?P<initial_end>-?\d+) "
    r"argument_null=(?P<argument_null>true|false) "
    rf"argument=(?P<argument>{DEBUG_STRING}) argument_utf16_length=(?P<argument_length>\d+) "
    r"before=" + link_snapshot_pattern("before") + r" after=" + link_snapshot_pattern("after")
    + r" hresult=(?P<hresult>0x[0-9A-F]{8}) result_null=(?P<result_null>true|false) "
    rf"result=(?P<result>{DEBUG_STRING}) result_utf16_length=(?P<result_length>\d+)"
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


def link_snapshot(values: dict[str, str], prefix: str) -> dict:
    def field(name: str) -> str:
        return values[prefix + "_" + name]

    start, end, story_length = (int(field(name)) for name in ("start", "end", "story_length"))
    if not 0 <= start <= end <= story_length:
        raise ValueError("Link range exceeds its native story")
    selected = state(tuple(field(name) for name in ("selection_start", "selection_end", "flags")))
    if selected["end"] > story_length:
        raise ValueError("Link snapshot selection exceeds its native story")
    result = {"range": {"start": start, "end": end}}
    for name in ("range_text", "story_text"):
        text = rust_string(field(name))
        length = int(field(name + "_utf16_length"))
        if len(text.encode("utf-16-le")) // 2 != length:
            raise ValueError("Link text disagrees with its explicit UTF-16 length")
        result[name] = text
        result[name + "_utf16_length"] = length
    if result["story_text_utf16_length"] > story_length:
        raise ValueError("Link story text exceeds its native story length")
    return {**result, "story_length": story_length, "selection": selected}


def link_command(match: re.Match) -> dict:
    values = match.groupdict()
    result = {
        "fixture": values["fixture"],
        "step": int(values["step"]),
        "role": values["role"],
        "operation": values["operation"],
        "initial_range": {"start": int(values["initial_start"]), "end": int(values["initial_end"])},
    }
    for name in ("argument", "result"):
        text = rust_string(values[name])
        length = int(values[name + "_length"])
        is_null = values[name + "_null"] == "true"
        if len(text.encode("utf-16-le")) // 2 != length or (is_null and text):
            raise ValueError("Link BSTR disagrees with its null/UTF-16 metadata")
        result[name] = {"is_null": is_null, "text": text, "utf16_length": length}
    if result["operation"] == "Get" and not result["argument"]["is_null"]:
        raise ValueError("GetURL has no input BSTR")
    if result["operation"] == "Set" and not result["result"]["is_null"]:
        raise ValueError("SetURL has no output BSTR")
    result["before"] = link_snapshot(values, "before")
    result["after"] = link_snapshot(values, "after")
    result["hresult"] = values["hresult"]
    return result


def validate_link_steps(fixture: dict, rows: list[dict]) -> None:
    if [row["step"] for row in rows] != list(range(len(rows))):
        raise ValueError("Link steps must be unique and consecutive from zero")
    roles = [row["role"] for row in rows]
    if roles.count("Target") != 1:
        raise ValueError("Link fixture needs exactly one target command")
    target = roles.index("Target")
    expected = ["Setup"] * target + ["Target"]
    if rows[target]["operation"] == "Set":
        expected.append("Readback")
    if roles != expected:
        raise ValueError("Link setup/target/readback order is incomplete")
    for index, row in enumerate(rows):
        initial = row["initial_range"]
        if not 0 <= initial["start"] <= initial["end"] <= fixture["native_story_length"]:
            raise ValueError("Link initial range exceeds its exact fixture")
        if row["role"] == "Setup":
            if row["operation"] != "Set" or row["argument"]["is_null"] or int(row["hresult"], 16) >= 0x80000000:
                raise ValueError("Link setup must be a successful SetURL")
        if index == 0:
            if row["before"]["range"] != initial or row["before"]["story_length"] != fixture["native_story_length"]:
                raise ValueError("first Link command disagrees with its initial fixture")
            if not row["before"]["story_text"].startswith(fixture["text"]):
                raise ValueError("initial Link story does not contain its exact fixture text")
        else:
            previous = rows[index - 1]
            for key in ("story_text", "story_text_utf16_length", "story_length", "selection"):
                if previous["after"][key] != row["before"][key]:
                    raise ValueError("Link story/selection changed between consecutive commands")
        if row["role"] == "Readback":
            previous = rows[index - 1]
            if row["operation"] != "Get" or initial != previous["initial_range"] or row["before"] != previous["after"]:
                raise ValueError("Link readback must observe the same target range")


def extract(log: str) -> dict:
    engine = None
    fixtures = []
    home_end = []
    vertical = []
    horizontal = []
    options = []
    link = []
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
                    "LINK_": "link",
                }[match[1]],
                "name": match[2],
                "text": text,
                "supplied_utf16_length": int(match[4]),
                "native_story_length": int(match[5]),
            }
            if fixture["native_story_length"] < fixture["supplied_utf16_length"]:
                raise ValueError("native story is shorter than its supplied text")
            if fixture["family"] in ("horizontal", "options", "link"):
                if match[6] is None or int(match[6]) > 0xFFFFFFFF:
                    raise ValueError("horizontal/options/Link fixture needs its native u32 window style")
                fixture["window_ex_style"] = int(match[6])
            elif match[6] is not None:
                raise ValueError("unexpected window style outside horizontal/options/Link fixture")
            if fixture["family"] == "link" and "link" in completed:
                raise ValueError("Link fixture follows its completion marker")
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
        match = LINK.fullmatch(row)
        if match:
            if "link" in completed:
                raise ValueError("Link command follows its completion marker")
            if not any(fixture["family"] == "link" and fixture["name"] == match["fixture"] for fixture in fixtures):
                raise ValueError("Link command precedes its exact fixture")
            link.append(link_command(match))
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
        match = re.fullmatch(r"LINK_PROBE_COMPLETE commands=(\d+) fixtures=(\d+)", row)
        if match:
            if "link" in completed:
                raise ValueError("duplicate Link completion")
            completed["link"] = (int(match[1]), int(match[2]))
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
    if link or "link" in completed:
        counts = (len(link), sum(fixture["family"] == "link" for fixture in fixtures))
        if counts != completed.get("link") or not link:
            raise ValueError("Link observations do not match the completion counts")
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
    link_rows = {}
    for row in link:
        identity = ("link", row["fixture"])
        if identity not in by_fixture:
            raise ValueError("Link command has no exact text fixture")
        used_fixtures.add(identity)
        link_rows.setdefault(identity, []).append(row)
    for identity, rows in link_rows.items():
        validate_link_steps(by_fixture[identity], rows)
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
    if link:
        result["link"] = link
        result["counts"]["link"] = len(link)
        result["counts"]["link_fixtures"] = len(link_rows)
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
