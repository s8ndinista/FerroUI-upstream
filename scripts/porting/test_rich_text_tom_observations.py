#!/usr/bin/env python3
"""Extraction integrity tests; synthetic rows are not native conformance evidence."""

from __future__ import annotations

import importlib.util
import copy
import json
from pathlib import Path
import unittest


SCRIPT = Path(__file__).with_name("rich-text-tom-observations.py")
SPEC = importlib.util.spec_from_file_location("rich_text_tom_observations", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
PARSER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PARSER)


def debug_string(value: str) -> str:
    escapes = {'"': '\\"', "\\": "\\\\", "\n": "\\n", "\r": "\\r", "\t": "\\t", "\0": "\\0"}
    return '"' + "".join(
        escapes.get(char, f"\\u{{{ord(char):x}}}" if ord(char) < 0x20 or char == "\u200d" else char)
        for char in value
    ) + '"'


def native_state(value: dict) -> str:
    return "SelectionState { start: %s, end: %s, flags: %s }" % (
        value["start"], value["end"], value["flags"]
    )


def options_row(row: dict) -> str:
    return (
        f'OPTIONS fixture={row["fixture"]} requested_flags={row["requested_flags"]} '
        f'before={native_state(row["before"])} before_type={row["before"]["selection_type"]} '
        f'after_set={native_state(row["after_set"])} after_set_type={row["after_set"]["selection_type"]} '
        f'set_hresult={row["set_hresult"]} typed={debug_string(row["typed"])} '
        f'after={native_state(row["after"])} after_type={row["after"]["selection_type"]} '
        f'type_hresult={row["type_hresult"]} native_story_length_after={row["native_story_length_after"]} '
        f'story_text={debug_string(row["story_text"])} story_text_utf16_length={row["story_text_utf16_length"]}'
    )


def link_snapshot_row(snapshot: dict) -> str:
    return (
        'LinkSnapshot { range: RangeState { start: %s, end: %s }, '
        'range_text: %s, range_text_utf16_length: %s, story_text: %s, '
        'story_text_utf16_length: %s, story_length: %s, selection: %s }'
    ) % (
        snapshot["range"]["start"], snapshot["range"]["end"],
        debug_string(snapshot["range_text"]), snapshot["range_text_utf16_length"],
        debug_string(snapshot["story_text"]), snapshot["story_text_utf16_length"],
        snapshot["story_length"], native_state(snapshot["selection"]),
    )


def link_row(row: dict) -> str:
    return (
        f'LINK fixture={row["fixture"]} step={row["step"]} role={row["role"]} '
        f'operation={row["operation"]} initial_start={row["initial_range"]["start"]} '
        f'initial_end={row["initial_range"]["end"]} '
        f'argument_null={str(row["argument"]["is_null"]).lower()} '
        f'argument={debug_string(row["argument"]["text"])} argument_utf16_length={row["argument"]["utf16_length"]} '
        f'before={link_snapshot_row(row["before"])} after={link_snapshot_row(row["after"])} '
        f'hresult={row["hresult"]} result_null={str(row["result"]["is_null"]).lower()} '
        f'result={debug_string(row["result"]["text"])} result_utf16_length={row["result"]["utf16_length"]}'
    )


def native_number_row(value: dict) -> str:
    return 'NativeNumber { hresult: %s, value: %s }' % (value["hresult"], value["value"])


def range_state_row(value: dict) -> str:
    return 'RangeState { start: %s, end: %s }' % (value["start"], value["end"])


def gravity_range_row(value: dict) -> str:
    result = 'GravityRangeState { range: ' + range_state_row(value["range"]) + ', '
    for name in ("gravity", "flags", "bold", "italic", "alignment"):
        result += name + ': ' + native_number_row(value[name]) + ', '
    return result + f'text: {debug_string(value["text"])}, text_utf16_length: {value["text_utf16_length"]} }}'


def gravity_snapshot_row(value: dict) -> str:
    duplicate = 'None' if value["duplicate"] is None else 'Some(' + gravity_range_row(value["duplicate"]) + ')'
    return (
        'GravitySnapshot { primary: ' + gravity_range_row(value["primary"]) + ', duplicate: ' + duplicate
        + f', story_text: {debug_string(value["story_text"])}, story_text_utf16_length: {value["story_text_utf16_length"]}'
        + f', story_length: {value["story_length"]}, selection: {native_state(value["selection"])} }}'
    )


def gravity_row(row: dict) -> str:
    output = 'None' if row["output"] is None else 'Some(%s)' % row["output"]
    return (
        f'GRAVITY fixture={row["fixture"]} step={row["step"]} role={row["role"]} target={row["target"]} '
        f'operation={row["operation"]} argument1={row["argument1"]} argument2={row["argument2"]} value={row["value"]} '
        f'text={debug_string(row["text"])} text_utf16_length={row["text_utf16_length"]} '
        f'action_before={range_state_row(row["action_before"])} action_after={range_state_row(row["action_after"])} '
        f'before={gravity_snapshot_row(row["before"])} after={gravity_snapshot_row(row["after"])} '
        f'hresult={row["hresult"]} output={output}'
    )


def record_log(record: dict) -> str:
    """Reconstitute the probe grammar from immutable previously captured rows."""
    engine = record["engine"]
    version = ", ".join(map(str, engine["windows_version"]))
    lines = [f'ENGINE windows=({version}) dll={json.dumps(engine["dll_path"])} file_version={engine["dll_file_version"]}']
    for fixture in record["fixtures"]:
        prefix = "" if fixture["family"] == "home_end" else fixture["family"].upper() + "_"
        line = (
            f'{prefix}FIXTURE name={fixture["name"]} text={debug_string(fixture["text"])} '
            f'supplied_utf16_length={fixture["supplied_utf16_length"]} native_story_length={fixture["native_story_length"]}'
        )
        if "window_ex_style" in fixture:
            line += f' window_ex_style={fixture["window_ex_style"]}'
        if "expected_commands" in fixture:
            line += f' expected_commands={fixture["expected_commands"]}'
        lines.append(line)
    for family in ("home_end", "vertical", "horizontal"):
        for row in record.get(family, []):
            identity = f'{family.upper()} fixture={row["fixture"]} '
            if family == "home_end":
                identity += f'supplied_utf16_length={row["supplied_utf16_length"]} native_story_length={row["native_story_length"]} '
            identity += f'unit={row["unit"]} '
            if family != "home_end":
                identity += f'count={row["count"]} '
            identity += f'extend={str(row["extend"]).lower()} operation={row["operation"]} '
            states = f'before={native_state(row["before"])} '
            if family == "horizontal":
                states += f'before_type={row["before"]["selection_type"]} '
            states += f'after={native_state(row["after"])} '
            if family == "horizontal":
                states += f'after_type={row["after"]["selection_type"]} '
            lines.append(
                identity + states + f'anchor={row["after"]["anchor"]} active={row["after"]["active"]} '
                f'delta={row["delta"]} active_displacement={row["active_displacement"]} hresult={row["hresult"]}'
            )
            if "repeat" in row:
                repeat = row["repeat"]
                lines.append(
                    f'HOME_END_REPEAT fixture={row["fixture"]} unit={row["unit"]} '
                    f'extend={str(row["extend"]).lower()} operation={row["operation"]} '
                    f'after={native_state(repeat["after"])} delta={repeat["delta"]} hresult={repeat["hresult"]}'
                )
    for row in record.get("options", []):
        lines.append(options_row(row))
    for row in record.get("link", []):
        lines.append(link_row(row))
    for row in record.get("gravity", []):
        lines.append(gravity_row(row))
    for family in ("home_end", "vertical", "horizontal", "options", "link", "gravity"):
        count = record["counts"].get(family, 0)
        if not count:
            continue
        if family == "home_end":
            lines.append(f'PROBE_COMPLETE commands={count} repeated_noops={record["counts"]["home_end_repeats"]}')
        elif family == "link":
            lines.append(f'LINK_PROBE_COMPLETE commands={count} fixtures={record["counts"]["link_fixtures"]}')
        elif family == "gravity":
            lines.append(f'GRAVITY_PROBE_COMPLETE commands={count} fixtures={record["counts"]["gravity_fixtures"]}')
        else:
            lines.append(f'{family.upper()}_PROBE_COMPLETE commands={count}')
    return "\n".join("SYSTEM_MSFTEDIT_" + line for line in lines)


SYNTHETIC_OPTIONS = (
    'SYSTEM_MSFTEDIT_ENGINE windows=(10, 0, 26100) dll="Msftedit.dll" file_version=1.2.3.4\n'
    'SYSTEM_MSFTEDIT_OPTIONS_FIXTURE name=example text="abcdef" supplied_utf16_length=6 native_story_length=7 window_ex_style=0\n'
    'SYSTEM_MSFTEDIT_OPTIONS fixture=example requested_flags=17 '
    'before=SelectionState { start: 1, end: 4, flags: 16 } before_type=2 '
    'after_set=SelectionState { start: 1, end: 4, flags: 17 } after_set_type=2 '
    'set_hresult=0x00000000 typed="😀" '
    'after=SelectionState { start: 3, end: 3, flags: 17 } after_type=1 '
    'type_hresult=0x00000000 native_story_length_after=6 story_text="a😀ef\\r" story_text_utf16_length=6\n'
    'SYSTEM_MSFTEDIT_OPTIONS_PROBE_COMPLETE commands=1'
)


def synthetic_link_record() -> dict:
    """Invented parser cases; no return value or hidden-field layout is evidence."""
    def snapshot(start: int, end: int, text: str, story: str) -> dict:
        return {
            "range": {"start": start, "end": end},
            "range_text": text,
            "range_text_utf16_length": len(text.encode("utf-16-le")) // 2,
            "story_text": story,
            "story_text_utf16_length": len(story.encode("utf-16-le")) // 2,
            "story_length": len(story.encode("utf-16-le")) // 2,
            "selection": {"start": 0, "end": 0, "flags": 17, "anchor": 0, "active": 0},
        }

    null_bstr = {"is_null": True, "text": "", "utf16_length": 0}
    url = '"https://例子.example/😀"'
    url_bstr = {"is_null": False, "text": url, "utf16_length": len(url.encode("utf-16-le")) // 2}
    initial = snapshot(0, 1, "a", "ab\r")
    hidden = snapshot(3, 4, "a", "\x13X\x14ab\r")
    expanded = snapshot(0, 4, "\x13X\x14a", hidden["story_text"])
    rows = [
        {
            "fixture": "example_link", "step": 0, "role": "Target", "operation": "Set",
            "initial_range": {"start": 0, "end": 1}, "argument": url_bstr,
            "result": null_bstr, "before": initial, "after": hidden, "hresult": "0x00000000",
        },
        {
            "fixture": "example_link", "step": 1, "role": "Readback", "operation": "Get",
            "initial_range": {"start": 0, "end": 1}, "argument": null_bstr,
            "result": url_bstr, "before": hidden, "after": expanded, "hresult": "0x00000000",
        },
    ]
    return {
        "engine": {"windows_version": [10, 0, 26100], "dll_path": "Msftedit.dll", "dll_file_version": "1.2.3.4"},
        "fixtures": [{
            "family": "link", "name": "example_link", "text": "ab", "supplied_utf16_length": 2,
            "native_story_length": 3, "window_ex_style": 0,
        }],
        "home_end": [], "vertical": [], "link": [copy.deepcopy(row) for row in rows],
        "counts": {"home_end": 0, "home_end_repeats": 0, "vertical": 0, "link": 2, "link_fixtures": 1},
    }


def synthetic_gravity_record() -> dict:
    """Invented parser states, explicitly not native gravity/format expectations."""
    def number(value: int, hresult: str = "0x00000000") -> dict:
        return {"hresult": hresult, "value": value}

    def text_length(text: str) -> int:
        return len(text.encode("utf-16-le")) // 2

    def tracked(start: int, end: int, gravity: int, story: str) -> dict:
        text = story.encode("utf-16-le")[2 * start:2 * end].decode("utf-16-le")
        return {
            "range": {"start": start, "end": end}, "gravity": number(gravity),
            "flags": number(-(2 ** 31), "0x80004001"), "bold": number(0),
            "italic": number(0), "alignment": number(0), "text": text,
            "text_utf16_length": text_length(text),
        }

    current = {
        "primary": tracked(0, 0, 0, "a😀b\r"), "duplicate": None,
        "story_text": "a😀b\r", "story_text_utf16_length": 5, "story_length": 5,
        "selection": {"start": 0, "end": 0, "flags": 17, "anchor": 0, "active": 0},
    }
    rows = []

    def add(operation: str, after: dict, *, target: str = "Primary", a: int = 0, b: int = 0,
            value: int = 0, text: str = "", role: str = "Target", hresult: str = "0x00000000",
            output: int | None = None) -> None:
        nonlocal current
        action_before = {"start": a, "end": b} if target == "External" else current[target.lower()]["range"]
        action_after = (
            {"start": a, "end": a + text_length(text)} if operation == "SetText" else action_before
        ) if target == "External" else after[target.lower()]["range"]
        rows.append(copy.deepcopy({
            "fixture": "example_gravity", "step": len(rows), "role": role, "target": target,
            "operation": operation, "argument1": a, "argument2": b, "value": value,
            "text": text, "text_utf16_length": text_length(text), "before": current,
            "after": after, "action_before": action_before, "action_after": action_after,
            "hresult": hresult, "output": output,
        }))
        current = copy.deepcopy(after)

    after = copy.deepcopy(current)
    after["primary"]["bold"] = number(-1)
    add("SetBold", after, target="External", a=0, b=1, value=-1, role="Setup")
    after = copy.deepcopy(current)
    after["primary"] = tracked(1, 3, 0, current["story_text"])
    after["primary"]["bold"] = number(-9_999_999)
    add("SetRange", after, a=3, b=1)
    after = copy.deepcopy(current)
    after["primary"]["gravity"] = number(4)
    add("SetGravity", after, value=4)
    after = copy.deepcopy(current)
    after["duplicate"] = copy.deepcopy(after["primary"])
    add("Duplicate", after)
    after = copy.deepcopy(current)
    after.update(story_text="aX😀b\r", story_text_utf16_length=6, story_length=6)
    for name in ("primary", "duplicate"):
        after[name] = tracked(1, 4, 4, after["story_text"])
        after[name]["bold"] = number(-9_999_999)
    add("SetText", after, target="External", a=1, b=1, text="X")
    after = copy.deepcopy(current)
    after["duplicate"]["gravity"] = number(2)
    add("SetGravity", after, target="Duplicate", value=2)
    after = copy.deepcopy(current)
    after["primary"] = tracked(0, 0, 4, after["story_text"])
    add("SetRange", after, a=0, b=0)
    add("GetGravity", current, output=4)
    add("SetGravity", current, value=-(2 ** 31), hresult="0x80070057")
    add("Collapse", current, value=-1, hresult="0x00000001")
    add("GetGravity", current, output=4)
    return {
        "engine": {"windows_version": [10, 0, 26100], "dll_path": "Msftedit.dll", "dll_file_version": "1.2.3.4"},
        "fixtures": [{
            "family": "gravity", "name": "example_gravity", "text": "a😀b", "supplied_utf16_length": 4,
            "native_story_length": 5, "window_ex_style": 0, "expected_commands": len(rows),
        }],
        "home_end": [], "vertical": [], "gravity": rows,
        "counts": {"home_end": 0, "home_end_repeats": 0, "vertical": 0, "gravity": len(rows), "gravity_fixtures": 1},
    }


class NativeObservationExtractionTests(unittest.TestCase):
    def test_every_versioned_record_roundtrips_without_changing_observations(self) -> None:
        paths = sorted((SCRIPT.parents[2] / "docs/porting/winui-rich-text").glob("native-msftedit-*.json"))
        self.assertGreaterEqual(len(paths), 4)
        for path in paths:
            with self.subTest(record=path.name):
                record = json.loads(path.read_text(encoding="utf-8"))
                expected = {key: value for key, value in record.items() if key not in ("schema_version", "scope", "provenance")}
                self.assertEqual(PARSER.extract(record_log(record)), expected)

    def test_options_preserve_each_phase_direction_type_and_changed_story(self) -> None:
        record = PARSER.extract(SYNTHETIC_OPTIONS)
        row = record["options"][0]
        self.assertEqual(record["counts"]["options"], 1)
        self.assertEqual((row["before"]["anchor"], row["before"]["active"]), (1, 4))
        self.assertEqual((row["after_set"]["anchor"], row["after_set"]["active"]), (4, 1))
        self.assertEqual(row["after"]["selection_type"], 1)
        self.assertEqual(row["after"]["active"], 3)
        self.assertEqual(row["typed"], "😀")
        self.assertEqual(row["story_text"], "a😀ef\r")
        self.assertEqual(row["native_story_length_after"], 6)
        self.assertEqual(PARSER.extract(record_log(record)), record)

    def test_options_allow_story_growth_beyond_original_fixture(self) -> None:
        log = SYNTHETIC_OPTIONS.replace(
            "start: 3, end: 3, flags: 17", "start: 9, end: 9, flags: 17"
        ).replace("native_story_length_after=6", "native_story_length_after=10").replace(
            'story_text="a😀ef\\r" story_text_utf16_length=6',
            'story_text="abcdefXYZ\\r" story_text_utf16_length=10',
        )
        self.assertEqual(PARSER.extract(log)["options"][0]["after"]["end"], 9)

    def test_options_reject_incomplete_duplicate_and_mismatched_counts(self) -> None:
        for log in (
            SYNTHETIC_OPTIONS.rsplit("\n", 1)[0],
            SYNTHETIC_OPTIONS.replace("commands=1", "commands=2"),
            SYNTHETIC_OPTIONS + "\nSYSTEM_MSFTEDIT_OPTIONS_PROBE_COMPLETE commands=1",
        ):
            with self.subTest(log=log[-90:]), self.assertRaises(ValueError):
                PARSER.extract(log)

    def test_options_reject_inconsistent_states_lengths_flags_and_fixture_identity(self) -> None:
        edits = (
            ("requested_flags=17", "requested_flags=32"),
            ("fixture=example", "fixture=missing"),
            ("start: 1, end: 4, flags: 17", "start: 1, end: 8, flags: 17"),
            ("start: 3, end: 3, flags: 17", "start: 7, end: 7, flags: 17"),
            ("start: 3, end: 3, flags: 17", "start: 4, end: 3, flags: 17"),
            ("story_text_utf16_length=6", "story_text_utf16_length=5"),
            ("native_story_length_after=6", "native_story_length_after=5"),
            ("native_story_length=7", "native_story_length=5"),
            ("window_ex_style=0", "window_ex_style=4294967296"),
            (" window_ex_style=0", ""),
        )
        for old, new in edits:
            with self.subTest(old=old, new=new), self.assertRaises(ValueError):
                PARSER.extract(SYNTHETIC_OPTIONS.replace(old, new))

    def test_an_earlier_complete_probe_does_not_hide_a_later_incomplete_family(self) -> None:
        fixture = (
            '\nSYSTEM_MSFTEDIT_HORIZONTAL_FIXTURE name=incomplete text="a" '
            'supplied_utf16_length=1 native_story_length=2 window_ex_style=0'
        )
        with self.assertRaisesRegex(ValueError, "no completed probe"):
            PARSER.extract(SYNTHETIC_OPTIONS + fixture)
        orphan = fixture.replace("HORIZONTAL_FIXTURE", "OPTIONS_FIXTURE")
        with self.assertRaisesRegex(ValueError, "no observed command"):
            PARSER.extract(SYNTHETIC_OPTIONS + orphan)

    def test_rust_debug_escaping_preserves_unicode_and_literal_backslashes(self) -> None:
        value = 'A😀e\u0301👩\u200d👦\\u{200d}\0\r\n\t"'
        self.assertEqual(PARSER.rust_string(debug_string(value)), value)
        for invalid in ('"\\u{d800}"', '"\\u{110000}"', '"\\u{}"', '"\\x20"', '"\\u200d"', '"\n"'):
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                PARSER.rust_string(invalid)

    def test_link_preserves_exact_bstr_utf16_hidden_story_and_getter_range_expansion(self) -> None:
        expected = synthetic_link_record()
        record = PARSER.extract(record_log(expected))
        self.assertEqual(record, expected)
        setter, getter = record["link"]
        self.assertEqual(setter["before"]["story_length"], 3)
        self.assertEqual(setter["after"]["story_length"], 6)
        self.assertEqual(getter["before"]["range"], {"start": 3, "end": 4})
        self.assertEqual(getter["after"]["range"], {"start": 0, "end": 4})
        self.assertIn("😀", getter["result"]["text"])

    def test_link_preserves_null_and_allocated_empty_bstr_separately(self) -> None:
        for is_null in (False, True):
            record = synthetic_link_record()
            record["link"][1]["result"] = {"is_null": is_null, "text": "", "utf16_length": 0}
            with self.subTest(is_null=is_null):
                self.assertEqual(PARSER.extract(record_log(record)), record)

    def test_link_retains_target_hresult_failures_and_observes_readback(self) -> None:
        record = synthetic_link_record()
        setter, getter = record["link"]
        setter["hresult"] = "0x80070057"
        setter["after"] = copy.deepcopy(setter["before"])
        getter["before"] = copy.deepcopy(setter["after"])
        getter["after"] = copy.deepcopy(getter["before"])
        getter["hresult"] = "0x00000001"
        getter["result"] = {"is_null": True, "text": "", "utf16_length": 0}
        self.assertEqual(PARSER.extract(record_log(record)), record)

    def test_link_setup_has_explicit_successful_history_and_one_target(self) -> None:
        record = synthetic_link_record()
        record["link"][0]["role"] = "Setup"
        record["link"][1]["role"] = "Target"
        self.assertEqual(PARSER.extract(record_log(record)), record)
        record["link"][0]["hresult"] = "0x80070057"
        with self.assertRaisesRegex(ValueError, "successful SetURL"):
            PARSER.extract(record_log(record))

    def test_link_rejects_incomplete_duplicate_reordered_and_mismatched_steps(self) -> None:
        baseline = synthetic_link_record()
        invalid = []
        for steps in (
            [baseline["link"][0]],
            [baseline["link"][1], baseline["link"][0]],
            [baseline["link"][0], baseline["link"][0]],
        ):
            record = copy.deepcopy(baseline)
            record["link"] = steps
            record["counts"]["link"] = len(steps)
            invalid.append(record_log(record))
        log = record_log(baseline)
        invalid.extend((
            log.rsplit("\n", 1)[0],
            log.replace("commands=2 fixtures=1", "commands=3 fixtures=1"),
            log.replace("commands=2 fixtures=1", "commands=2 fixtures=2"),
            log + "\nSYSTEM_MSFTEDIT_LINK_PROBE_COMPLETE commands=2 fixtures=1",
            log + "\nSYSTEM_MSFTEDIT_" + link_row(baseline["link"][0]),
            log.replace("step=1 role=Readback", "step=2 role=Readback"),
        ))
        for value in invalid:
            with self.subTest(log=value[-100:]), self.assertRaises(ValueError):
                PARSER.extract(value)

    def test_link_rejects_contradictory_range_story_bstr_and_phase_metadata(self) -> None:
        baseline = synthetic_link_record()
        for row_index, path, value in (
            (0, ("initial_range", "end"), 4),
            (0, ("before", "range", "start"), 1),
            (0, ("after", "story_length"), 3),
            (0, ("after", "range_text_utf16_length"), 2),
            (0, ("after", "selection", "end"), 7),
            (0, ("argument", "is_null"), True),
            (0, ("argument", "utf16_length"), 1),
            (0, ("result", "is_null"), False),
            (1, ("argument", "is_null"), False),
            (1, ("initial_range", "end"), 2),
            (1, ("before", "range", "start"), 2),
            (1, ("before", "story_text"), "\x13Y\x14ab\r"),
        ):
            record = copy.deepcopy(baseline)
            target = record["link"][row_index]
            for name in path[:-1]:
                target = target[name]
            target[path[-1]] = value
            with self.subTest(row=row_index, path=path), self.assertRaises(ValueError):
                PARSER.extract(record_log(record))

    def test_link_rejects_undeclared_or_late_fixtures(self) -> None:
        lines = record_log(synthetic_link_record()).splitlines()
        for reordered in (
            [lines[0], lines[2], lines[1], *lines[3:]],
            [*lines, lines[1].replace("name=example_link", "name=late")],
        ):
            with self.assertRaises(ValueError):
                PARSER.extract("\n".join(reordered))

    def test_gravity_roundtrips_raw_getters_formats_direction_and_live_duplicates(self) -> None:
        expected = synthetic_gravity_record()
        actual = PARSER.extract(record_log(expected))
        self.assertEqual(actual, expected)
        rows = actual["gravity"]
        self.assertEqual((rows[1]["argument1"], rows[1]["argument2"]), (3, 1))
        self.assertEqual(rows[1]["after"]["primary"]["bold"]["value"], -9_999_999)
        self.assertEqual(rows[1]["after"]["primary"]["flags"], {"hresult": "0x80004001", "value": -(2 ** 31)})
        self.assertEqual(rows[4]["after"]["primary"]["text"], "X😀")
        self.assertEqual(rows[5]["after"]["duplicate"]["gravity"]["value"], 2)
        self.assertEqual(rows[5]["after"]["primary"]["gravity"]["value"], 4)
        self.assertEqual(rows[8]["hresult"], "0x80070057")
        self.assertEqual(rows[9]["hresult"], "0x00000001")

    def test_gravity_keeps_extreme_invalid_inputs_and_failed_getter_outputs(self) -> None:
        for value in (-(2 ** 31), -9_999_999, -1, 5, 2 ** 31 - 1):
            record = synthetic_gravity_record()
            record["gravity"][8]["value"] = value
            record["gravity"][7]["hresult"] = "0x80004001"
            record["gravity"][7]["output"] = -(2 ** 31)
            with self.subTest(value=value):
                self.assertEqual(PARSER.extract(record_log(record)), record)
        record = synthetic_gravity_record()
        record["gravity"][6]["argument1"] = 2 ** 31 - 1
        record["gravity"][6]["argument2"] = -(2 ** 31)
        self.assertEqual(PARSER.extract(record_log(record)), record)

    def test_gravity_rejects_incomplete_duplicate_or_mismatched_completion(self) -> None:
        record = synthetic_gravity_record()
        log = record_log(record)
        invalid = [
            log.rsplit("\n", 1)[0],
            log.replace("expected_commands=11", "expected_commands=10"),
            log.replace("commands=11 fixtures=1", "commands=10 fixtures=1"),
            log.replace("commands=11 fixtures=1", "commands=11 fixtures=2"),
            log + "\nSYSTEM_MSFTEDIT_GRAVITY_PROBE_COMPLETE commands=11 fixtures=1",
            log + "\nSYSTEM_MSFTEDIT_" + gravity_row(record["gravity"][-1]),
            log.replace("step=9 role=Target", "step=10 role=Target"),
        ]
        for rows in (record["gravity"][:-1], record["gravity"] + [record["gravity"][-1]],
                     [record["gravity"][1], record["gravity"][0], *record["gravity"][2:]]):
            changed = copy.deepcopy(record)
            changed["gravity"] = rows
            changed["counts"]["gravity"] = len(rows)
            changed["fixtures"][0]["expected_commands"] = len(rows)
            invalid.append(record_log(changed))
        for value in invalid:
            with self.subTest(log=value[-90:]), self.assertRaises(ValueError):
                PARSER.extract(value)

    def test_gravity_rejects_unknown_or_late_fixture_declarations(self) -> None:
        lines = record_log(synthetic_gravity_record()).splitlines()
        invalid = (
            [lines[0], lines[2], lines[1], *lines[3:]],
            [*lines, lines[1].replace("name=example_gravity", "name=late")],
            [lines[0], lines[1].replace(" expected_commands=11", ""), *lines[2:]],
            [lines[0], lines[1].replace(" window_ex_style=0", ""), *lines[2:]],
        )
        for reordered in invalid:
            with self.assertRaises(ValueError):
                PARSER.extract("\n".join(reordered))
        with self.assertRaises(ValueError):
            PARSER.extract(SYNTHETIC_OPTIONS.replace("window_ex_style=0", "window_ex_style=0 expected_commands=1"))

    def test_gravity_rejects_contradictory_ranges_utf16_and_native_widths(self) -> None:
        baseline = synthetic_gravity_record()
        for row_index, path, value in (
            (1, ("argument1",), 2 ** 31),
            (2, ("value",), -(2 ** 31) - 1),
            (7, ("output",), 2 ** 31),
            (0, ("before", "primary", "gravity", "value"), 2 ** 31),
            (0, ("before", "primary", "range", "end"), 6),
            (0, ("before", "primary", "text_utf16_length"), 1),
            (0, ("before", "story_text_utf16_length"), 4),
            (0, ("before", "story_length"), 4),
            (0, ("before", "selection", "end"), 6),
            (0, ("before", "selection", "flags"), 2 ** 31),
            (4, ("text_utf16_length",), 2),
            (4, ("action_before", "end"), 2),
            (4, ("action_after", "end"), 7),
            (5, ("action_before", "start"), 0),
            (5, ("after", "duplicate", "range", "start"), 5),
        ):
            record = copy.deepcopy(baseline)
            target = record["gravity"][row_index]
            for name in path[:-1]:
                target = target[name]
            target[path[-1]] = value
            with self.subTest(row=row_index, path=path), self.assertRaises(ValueError):
                PARSER.extract(record_log(record))

    def test_gravity_rejects_wrong_argument_shapes_and_unrecorded_outputs(self) -> None:
        for row_index, key, value in (
            (7, "output", None), (8, "output", 4), (8, "argument1", 1),
            (8, "text", "X"), (6, "value", 1), (9, "value", 1),
            (0, "target", "Primary"), (0, "role", "Target"), (0, "value", 2),
            (0, "hresult", "0x80070057"), (7, "role", "Setup"),
            (1, "operation", "GetUnknownGravity"),
        ):
            record = synthetic_gravity_record()
            record["gravity"][row_index][key] = value
            if key == "text":
                record["gravity"][row_index]["text_utf16_length"] = len(value.encode("utf-16-le")) // 2
            with self.subTest(row=row_index, key=key), self.assertRaises(ValueError):
                PARSER.extract(record_log(record))

    def test_gravity_rejects_inconsistent_duplicate_lifetimes_and_targets(self) -> None:
        for row_index, phase, key, value in (
            (3, "before", "duplicate", synthetic_gravity_record()["gravity"][3]["after"]["duplicate"]),
            (3, "after", "duplicate", None),
            (4, "after", "duplicate", None),
            (5, "before", "duplicate", None),
        ):
            record = synthetic_gravity_record()
            record["gravity"][row_index][phase][key] = value
            with self.subTest(row=row_index, phase=phase), self.assertRaises(ValueError):
                PARSER.extract(record_log(record))
        record = synthetic_gravity_record()
        record["gravity"][3]["hresult"] = "0x80004001"
        with self.assertRaises(ValueError):
            PARSER.extract(record_log(record))

    def test_gravity_rejects_missing_initial_configuration_or_state_continuity(self) -> None:
        for row_index, path, value in (
            (1, ("operation",), "SetStart"),
            (2, ("operation",), "Collapse"),
            (4, ("before", "primary", "bold", "value"), -1),
            (4, ("before", "story_text"), "x😀b\r"),
            (4, ("before", "selection", "flags"), 16),
        ):
            record = synthetic_gravity_record()
            target = record["gravity"][row_index]
            for name in path[:-1]:
                target = target[name]
            target[path[-1]] = value
            with self.subTest(row=row_index, path=path), self.assertRaises(ValueError):
                PARSER.extract(record_log(record))
        record = synthetic_gravity_record()
        record["fixtures"][0]["text"] = "x😀b"
        with self.assertRaises(ValueError):
            PARSER.extract(record_log(record))

    def test_gravity_and_link_families_coexist_without_changing_old_schema(self) -> None:
        gravity = synthetic_gravity_record()
        link = synthetic_link_record()
        record = copy.deepcopy(gravity)
        record["fixtures"].extend(link["fixtures"])
        record["link"] = link["link"]
        record["counts"].update(link=2, link_fixtures=1)
        self.assertEqual(PARSER.extract(record_log(record)), record)
        old = PARSER.extract(SYNTHETIC_OPTIONS)
        self.assertNotIn("gravity", old)
        self.assertNotIn("gravity", old["counts"])


if __name__ == "__main__":
    unittest.main()
