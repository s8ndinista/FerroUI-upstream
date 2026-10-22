#!/usr/bin/env python3
"""Extraction integrity tests; synthetic rows are not native conformance evidence."""

from __future__ import annotations

import importlib.util
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
    for family in ("home_end", "vertical", "horizontal", "options"):
        count = record["counts"].get(family, 0)
        if not count:
            continue
        if family == "home_end":
            lines.append(f'PROBE_COMPLETE commands={count} repeated_noops={record["counts"]["home_end_repeats"]}')
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


if __name__ == "__main__":
    unittest.main()
