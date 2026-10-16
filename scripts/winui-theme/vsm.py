#!/usr/bin/env python3
"""Drafts the nested styles of a control theme from the visual states of a WinUI 3 style
(docs/porting/winui-theme.md, section 4.4). A help for porting a control template by hand: it
prints markup to review and paste, it writes nothing.

A visual state of WinUI sets values while it is the current state of its group: discrete
`ObjectAnimationUsingKeyFrames` (one `DiscreteObjectKeyFrame` at `KeyTime="0"`) and the
`Setter Target="Element.Property"` of `VisualState.Setters`. In FerroUI the state is a
pseudo-class of the control and the values are setters of a nested style that matches it:

    VisualState PointerOver: ContentPresenter.Background = {ThemeResource ButtonBackgroundPointerOver}
    ->
    <Style Selector="^:pointerover">
      <Style Selector="^ /template/ ContentPresenter#PART_ContentPresenter">
        <Setter Property="Background" Value="{DynamicResource ButtonBackgroundPointerOver}" />
      </Style>
    </Style>

The states are written in the order given, so that a later state wins where two match (a
pressed button is also under the pointer). Everything else a state does (real animations,
transitions, a value that is markup, a property of a type FerroUI lacks) is printed as a comment
that starts with `TODO(port)`; a committed document has none.

usage: vsm.py <source document> <style key> --states <name>=<selector>;... --targets <element>=<selector>;...
              [--properties <element>.<property>=<property>;...] [--reference <microsoft-ui-xaml>]

  <source document>  a path of microsoft-ui-xaml, e.g. controls/dev/CommonStyles/Button_themeresources.xaml
  <style key>        the x:Key of the style, or {TargetType} for an implicit one
  --states           the state names to translate, each with the selector suffix of its pseudo-classes
                     (`PointerOver=:pointerover;Pressed=:pressed;CheckedPointerOver=:checked:pointerover`);
                     a state with an empty selector (`Normal=`) sets the base values
  --targets          the named elements, each with its selector in the FerroUI template
                     (`ContentPresenter=/template/ ContentPresenter#PART_ContentPresenter`); the empty
                     selector is the control itself
  --properties       property renames (`CheckGlyph.Foreground=Fill`; an empty name drops the setter)
"""

import argparse
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import winui_sources as w  # noqa: E402


def pairs(text):
    result = {}
    for part in (text or "").split(";"):
        if not part.strip():
            continue
        name, _, value = part.partition("=")
        result[name.strip()] = value.strip()
    return result


def convert_value(value):
    match = re.fullmatch(r"\{ThemeResource\s+([^\s}]+)\s*\}", value.strip())
    if match:
        return "{DynamicResource " + match.group(1) + "}"
    return value


def find_style(root, key):
    for item in w.document_items(w.reference_root(), root):
        if item.kind == "Style" and item.key == key:
            return item.element
    raise SystemExit(f"error: no style {key} in {root}")


def state_values(state):
    """[(element, property, value, note)] of a visual state; note is set for what is not a plain
    value."""
    values = []
    for child in state.iter():
        name = w.local_name(child.tag) if isinstance(child.tag, str) else ""
        if name == "ObjectAnimationUsingKeyFrames":
            target = child.get("Storyboard.TargetName")
            prop = child.get("Storyboard.TargetProperty")
            frames = [f for f in child if isinstance(f.tag, str)]
            if len(frames) == 1 and frames[0].get("KeyTime", "0") in ("0", "0:0:0") and frames[0].get("Value") is not None:
                values.append((target, prop, frames[0].get("Value"), None))
            else:
                values.append((target, prop, None, "keyframes that are not one discrete value at time 0"))
        elif name == "Setter" and child.get("Target"):
            target, _, prop = child.get("Target").partition(".")
            values.append((target, prop, child.get("Value"), None))
        elif name in ("DoubleAnimation", "ColorAnimation", "PointAnimation"):
            target = (child.get("Storyboard.TargetName"), child.get("Storyboard.TargetProperty"))
            immediate = child.get("Duration", "") in ("0", "0:0:0") and child.get("BeginTime") is None
            if immediate and child.get("To") is not None:
                # An animation of no duration is a value.
                values.append((*target, child.get("To"), None))
            else:
                timing = f"duration {child.get('Duration')}" + (f", begin {child.get('BeginTime')}" if child.get("BeginTime") else "")
                values.append((*target, child.get("To"), f"{name} to {child.get('To')}, {timing}"))
        elif name in ("DoubleAnimationUsingKeyFrames", "ColorAnimationUsingKeyFrames"):
            target = (child.get("Storyboard.TargetName"), child.get("Storyboard.TargetProperty"))
            frames = [f for f in child if isinstance(f.tag, str)]
            if len(frames) == 1 and frames[0].get("KeyTime", "0") in ("0", "0:0:0") and child.get("BeginTime") is None:
                values.append((*target, frames[0].get("Value"), None))
            else:
                described = "; ".join(
                    f"{w.local_name(f.tag)} {f.get('Value')} at {f.get('KeyTime')}" + (f" spline {f.get('KeySpline')}" if f.get("KeySpline") else "")
                    for f in frames
                )
                values.append((*target, None, f"{name}: {described}" + (f", begin {child.get('BeginTime')}" if child.get("BeginTime") else "")))
    return values


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("document")
    parser.add_argument("key")
    parser.add_argument("--states", required=True)
    parser.add_argument("--targets", required=True)
    parser.add_argument("--properties", default="")
    parser.add_argument("--reference")
    arguments = parser.parse_args()
    if arguments.reference:
        os.environ["WINUI_XAML_ROOT"] = arguments.reference
    style = find_style(arguments.document, arguments.key)
    states = pairs(arguments.states)
    targets = pairs(arguments.targets)
    renames = pairs(arguments.properties)

    found = {}
    for state in style.iter():
        if isinstance(state.tag, str) and w.local_name(state.tag) == "VisualState":
            found[state.get(w.x("Name"))] = state

    for name, selector in states.items():
        state = found.get(name)
        if state is None:
            print(f"<!-- TODO(port): no visual state {name} -->")
            continue
        by_target = {}
        for element, prop, value, note in state_values(state):
            by_target.setdefault(element, []).append((prop, value, note))
        indent = "    "
        print(f'{indent}<!-- {name} -->')
        if selector:
            print(f'{indent}<Style Selector="^{selector}">')
            inner = indent + "  "
        else:
            inner = indent
        for element, setters in by_target.items():
            target = targets.get(element)
            if target is None:
                print(f"{inner}<!-- TODO(port): element {element} has no target -->")
                continue
            lines = []
            for prop, value, note in setters:
                renamed = renames.get(f"{element}.{prop}", prop)
                if renamed == "":
                    continue
                if note or value is None or (value.startswith("{") and not value.startswith("{ThemeResource") and not value.startswith("{StaticResource")):
                    lines.append(f"<!-- TODO(port): {element}.{prop} = {value} ({note or 'markup value'}) -->")
                else:
                    lines.append(f'<Setter Property="{renamed}" Value="{convert_value(value)}" />')
            if not lines:
                continue
            if target:
                print(f'{inner}<Style Selector="^ {target}">')
                for line in lines:
                    print(f"{inner}  {line}")
                print(f"{inner}</Style>")
            else:
                for line in lines:
                    print(f"{inner}{line}")
        if selector:
            print(f"{indent}</Style>")


if __name__ == "__main__":
    main()
