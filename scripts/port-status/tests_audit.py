#!/usr/bin/env python3
"""Measures how completely the upstream test suites are ported, per upstream test project.

    python3 scripts/port-status/tests_audit.py                    # write docs/porting/TESTS.md and tests-status.json
    python3 scripts/port-status/tests_audit.py --check            # exit 1 when either is out of date; writes nothing
    python3 scripts/port-status/tests_audit.py --extract          # re-read the upstream test projects first
    python3 scripts/port-status/tests_audit.py --explain Avalonia.Base.UnitTests [glob]
                                                                  # every test of a project and what matched it

Inputs

    docs/porting/data/upstream-tests.json   the upstream test inventory: every test method of every upstream test
                                            project at the tracked commit (and of the test projects of the two
                                            submodules, XamlX and the D-Bus library, at the commits the tracked
                                            tree names). Written by `--extract`, which reads the upstream checkout
                                            ($UPSTREAM, else ../Avalonia next to the repository) through `git
                                            archive`: nothing is checked out. Written as well when the file is
                                            missing or names another commit than docs/porting/TRACKING.md.
    docs/porting/data/test-aliases.toml     the aliases and waivers of scripts/port-status/test_gaps.py, read as
                                            that script reads them.
    the Rust tree                           `src`, `tests` and `external` of the repository, only read.

Outputs

    docs/porting/TESTS.md                   per upstream test project: tests, found, waived, missing; the
                                            projects that do not apply, with the reason; per project a collapsible
                                            list of the missing tests by upstream file.
    docs/porting/data/tests-status.json     the same, machine readable.

`--check` needs neither the upstream checkout nor git: it recomputes both outputs from the committed inventory and
the Rust tree and compares them with the files.

The rules of the match and their limits are written at the top of TESTS.md (RULES below); the projects that do not
apply are in PROJECTS. The parsing of the C# test methods and of the alias file is the one of test_gaps.py, which
reports one project in detail; this script differs from it in which Rust functions count (see RULES).

Standard library only (Python 3.11 or later).
"""

from __future__ import annotations

import argparse
import fnmatch
import io
import json
import os
import re
import subprocess
import sys
import tarfile
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import test_gaps as tg  # noqa: E402  (the C# parser, the Rust blanking and the alias file)

REPO = os.path.dirname(os.path.dirname(HERE))
DATA = os.path.join(REPO, "docs", "porting", "data")
INVENTORY = os.path.join(DATA, "upstream-tests.json")
STATUS = os.path.join(DATA, "tests-status.json")
DOCUMENT = os.path.join(REPO, "docs", "porting", "TESTS.md")
ALIASES = os.path.join(DATA, "test-aliases.toml")
RUST_ROOTS = ["src", "tests", "external"]

# The submodules whose test projects are read, with the directory of their tests.
SUBMODULES = ["external/XamlX", "external/Avalonia.DBus"]

# Directories of shared test sources that have no project file of their own but are projects for the count:
# `tests/Avalonia.RenderTests` is compiled by `Avalonia.Skia.RenderTests` (and the WPF comparison),
# `tests/Avalonia.Headless.UnitTests` by the four NUnit and xUnit projects of the headless platform.
SOURCE_DIRECTORIES = ["tests/Avalonia.RenderTests", "tests/Avalonia.Headless.UnitTests"]

# --------------------------------------------------------------------------------------------------------------
# The upstream test projects: whether each applies to the port, where its tests are, and why not when not.
# Every project of the inventory must be listed (the script stops otherwise), so a project upstream adds is
# decided on, not silently counted or dropped.
# --------------------------------------------------------------------------------------------------------------

NA = "not applicable"
PROJECTS: dict[str, dict] = {
    # -- applicable --------------------------------------------------------------------------------------------
    "tests/Avalonia.Base.UnitTests": {"rust": "next to the code, mostly `src/FerroUI.Base`"},
    "tests/Avalonia.Controls.UnitTests": {"rust": "next to the code, mostly `src/FerroUI.Controls`"},
    "tests/Avalonia.Markup.UnitTests": {"rust": "`src/Markup/FerroUI.Markup`"},
    "tests/Avalonia.Markup.Xaml.UnitTests": {"rust": "`tests/FerroUI.Markup.Xaml.UnitTests`, `src/Markup`"},
    "tests/Avalonia.Skia.UnitTests": {"rust": "`src/Skia/FerroUI.Skia`"},
    "tests/Avalonia.Headless.UnitTests": {"rust": "`src/Headless/FerroUI.Headless/unit_tests`"},
    "tests/Avalonia.Themes.UnitTests": {"rust": "`src/FerroUI.Themes.Fluent`, `src/FerroUI.Themes.Simple`"},
    "tests/Avalonia.Build.Tasks.UnitTest": {"rust": "`src/FerroUI.Build.Tasks`"},
    "tests/Avalonia.DesignerSupport.Tests": {"rust": "`src/FerroUI.DesignerSupport`"},
    "tests/Avalonia.RenderTests": {"rust": "`tests/FerroUI.RenderTests`",
                                   "note": "shared sources of `Avalonia.Skia.RenderTests`, counted here once"},
    "tests/Avalonia.Skia.RenderTests": {"rust": "`tests/FerroUI.RenderTests`",
                                        "note": "compiles the sources of `tests/Avalonia.RenderTests`; no tests of its own"},
    "tests/Avalonia.LeakTests": {"rust": "`tests/FerroUI.LeakTests`"},
    "tests/Avalonia.Benchmarks": {"rust": "`tests/FerroUI.Benchmarks`",
                                  "note": "a benchmark counts as found when the port has a function of its name"},
    "tests/Avalonia.IntegrationTests.Win32": {"rust": "`tests/FerroUI.IntegrationTests.Win32`"},
    "tests/Avalonia.IntegrationTests.Appium": {
        "rust": "`tests/FerroUI.IntegrationTests.Appium`",
        "note": "the tests drive `samples/IntegrationTestApp` from another process through the accessibility API of "
                "the platform, with the port's own automation layer in place of Appium (UI Automation on Windows, "
                "the accessibility API of macOS): `integration-tests.md`",
    },
    "tests/Avalonia.Wayland.UnitTests": {"rust": "`src/FerroUI.Wayland`"},
    "tests/Avalonia.UnitTests": {"rust": "`test_support` modules of the crates",
                                 "note": "the shared test library (`TestRoot`, `TestServices`, ...); no tests of its own"},
    "external/XamlX/tests/XamlParserTests": {"rust": "`src/Markup/FerroUI.Markup.Xaml.Loader/runtime/interpreter/tests`, "
                                                     "`external/XamlX/src/XamlX/tests`",
                                             "note": "the XAML compiler library is ported as the crate `xamlx`"},
    "external/XamlX/tests/TestClasses": {"rust": "none", "note": "types used by the other XamlX tests; no tests"},
    # -- not applicable ----------------------------------------------------------------------------------------
    "tests/Avalonia.Analyzers.Tests": {
        "status": NA, "reason": "Roslyn analyzers and code fixes for C# sources; the port has no .NET analyzers"},
    "tests/Avalonia.Generators.Tests": {
        "status": NA, "reason": "Roslyn source generators (named element accessors, `InitializeComponent`, generated "
                                "properties) whose C# output is compared with snapshots; the port generates no C#: the "
                                "XAML compiler emits the named elements (its tests are in `src/Markup`) and "
                                "`ferro_property!` declares properties"},
    "tests/Avalonia.Headless.NUnit.PerAssembly.UnitTests": {
        "status": NA, "reason": "runs the sources of `Avalonia.Headless.UnitTests` under the NUnit adapter; its own "
                                "file only sets the isolation level. The tests are counted once, under that project "
                                "(the port runs each per test and per assembly)"},
    "tests/Avalonia.Headless.NUnit.PerTest.UnitTests": {"status": NA, "reason": "as the NUnit per-assembly project"},
    "tests/Avalonia.Headless.XUnit.PerAssembly.UnitTests": {"status": NA, "reason": "as the NUnit per-assembly project, for xUnit"},
    "tests/Avalonia.Headless.XUnit.PerTest.UnitTests": {"status": NA, "reason": "as the NUnit per-assembly project, for xUnit"},
    "tests/Avalonia.RenderTests.WpfCompare": {
        "status": NA, "reason": "renders the cross tests with WPF to compare; WPF is a Windows .NET toolkit"},
    "tests/Avalonia.UnitTests.WpfCompare": {
        "status": NA, "reason": "runs unit tests against WPF to compare behaviour; WPF is a Windows .NET toolkit"},
    "tests/Avalonia.DesignerSupport.TestApp": {
        "status": NA, "reason": "the application started by `Avalonia.DesignerSupport.Tests`; no tests"},
    "tests/BuildTests/BuildTests": {"status": NA, "reason": "builds .NET project formats (MSBuild heads, trimming, "
                                                            "NativeAOT, WPF hybrid); the port builds with Cargo"},
    "tests/BuildTests/BuildTests.Android": {"status": NA, "reason": "as `BuildTests`"},
    "tests/BuildTests/BuildTests.Browser": {"status": NA, "reason": "as `BuildTests`"},
    "tests/BuildTests/BuildTests.Desktop": {"status": NA, "reason": "as `BuildTests`"},
    "tests/BuildTests/BuildTests.NativeAot": {"status": NA, "reason": "as `BuildTests`"},
    "tests/BuildTests/BuildTests.WpfHybrid": {"status": NA, "reason": "as `BuildTests`"},
    "tests/BuildTests/BuildTests.iOS": {"status": NA, "reason": "as `BuildTests`"},
    "tests/TestFiles/BuildTasks/PInvoke": {
        "status": NA, "reason": "an input assembly of the build task test (IL rewriting of P/Invoke); no tests"},
    "external/XamlX/tests/CecilTests": {
        "status": NA, "reason": "the IL backend of XamlX over Mono.Cecil (`XamlX.IL.Cecil`), which the port does not "
                                "have: its XAML compiler emits Rust"},
    "external/XamlX/tests/CecilNetstandardTests": {"status": NA, "reason": "as `CecilTests`"},
    "external/XamlX/tests/TypeSystemTest": {
        "status": NA, "reason": "runs one set of checks on the two type systems of XamlX over compiled .NET "
                                "assemblies, System.Reflection and Mono.Cecil (generic instantiation, nested types of "
                                "IL metadata); the port has neither: its type systems are over Rust metadata (the "
                                "fake type system of the `xamlx` tests, the run-time and build-time ones of `src/Markup`)"},
    "external/XamlX/tests/GuilabsParserTests": {
        "status": NA, "reason": "the alternative parser `XamlX.Parser.GuiLabs`, not ported"},
    "external/Avalonia.DBus/tests/Avalonia.DBus.Interop.Tests": {
        "status": NA, "reason": "the D-Bus library of upstream is not ported: the port talks to D-Bus through the "
                                "`zbus` crate (`x11-platform.md`, the D-Bus choice)"},
    "external/Avalonia.DBus/tests/Avalonia.DBus.SourceGen.Tests": {"status": NA, "reason": "as the D-Bus interop tests"},
    "external/Avalonia.DBus/tests/Avalonia.DBus.Testing": {"status": NA, "reason": "as the D-Bus interop tests"},
    "external/Avalonia.DBus/tests/Avalonia.DBus.Tests": {"status": NA, "reason": "as the D-Bus interop tests"},
    "external/Avalonia.DBus/tests/Avalonia.DBus.WireMarshalling.Tests": {"status": NA, "reason": "as the D-Bus interop tests"},
    "external/Avalonia.DBus/tests/NDesk.DBus.Tests": {"status": NA, "reason": "as the D-Bus interop tests"},
    "external/Avalonia.DBus/tests/external/ndesk-dbus-0.6.0/src": {"status": NA, "reason": "a D-Bus library vendored for the tests above"},
    "external/Avalonia.DBus/tests/external/ndesk-dbus-glib-0.4.1/src": {"status": NA, "reason": "as the vendored D-Bus library"},
}

# --------------------------------------------------------------------------------------------------------------
# The text at the top of TESTS.md: the rules, their limits and the calibration by hand.
# --------------------------------------------------------------------------------------------------------------

RULES = """\
## How the count is made

**Upstream.** Every method of a `.cs` file of an upstream test project that carries a test attribute: an attribute
whose name ends in `Fact`, `Theory`, `Test`, `TestCase` or `Benchmark`. At the tracked commit that is `[Fact]`,
`[Theory]`, `[Benchmark]`, the `[PlatformFact]` / `[PlatformTheory]` of the Appium tests, the `[CrossFact]` /
`[CrossTheory]` of the cross render tests, the `[ReleaseFact]` of the leak tests, the `[Win32Fact]` /
`[Win32Theory]` of the Skia tests, the headless `[AvaloniaFact]` / `[AvaloniaTheory]` (xUnit) and `[AvaloniaTest]`
(NUnit), and the NUnit `[Test]` / `[TestCase]`. A theory is one test however many rows it has. A method declared in
two classes of one file is one test per class (`Class.Method` in the lists); overloads in one class are one test.
Comments and strings are blanked first; code in an inactive `#if` branch is counted (a waiver says so where it
matters). A file belongs to the deepest directory above it with a project file; `tests/Avalonia.RenderTests` and
`tests/Avalonia.Headless.UnitTests`, which have none, are projects of their own (their sources are compiled by
`Avalonia.Skia.RenderTests` and by the four NUnit and xUnit projects of the headless platform). Sources a project
links from another directory are counted once, where they are. The inventory is
`docs/porting/data/upstream-tests.json`.

**Port.** The functions of test code under `src`, `tests` and `external`:

- every function of a file that is test code as a whole: a file named `*_tests*.rs`, `tests.rs` or `test_*.rs`, a
  file below a directory named `tests`, and a module declared under `#[cfg(test)]` (`#[cfg(test)] mod unit_tests;`,
  also with `#[path]`) with the files below its directory;
- in other files, the functions with a `#[test]` attribute, the functions inside an inline `#[cfg(test)] mod`, and
  the names handed to a macro whose `macro_rules!` body has a `#[test]` (`ferro_fact!(name)`, the binding suites).

Comments and string literals are blanked first. A function of test code without a `#[test]` of its own still counts
(the port writes an upstream theory as a function with upstream's name that the row tests call, and a benchmark is
a plain function).

**Match.** By name: the C# method name and the Rust function name, both lower-cased with the underscores removed, so
`Should_Do_It`, `ShouldDoIt` and `should_do_it` are one name; in the upstream name `Avalonia` reads as `Ferro`, a
word-initial `Avn` as `Frn` and `avares` as `ferres` (rule 2 of the porting guide). One Rust function is the port of
at most one upstream test, over all projects at once, assigned in this order:

1. *In its file* ("in its file" in the table): a function of the name in a Rust file or inline module named after
   the upstream file or the test class (`Styling/SelectorTests_Child.cs` and `selector_tests_child.rs` or
   `mod selector_child`; the words `tests` and `test` and the punctuation are ignored), the same name first, then
   one name the start of the other (`input_tests.rs` for `InputElement_Focus.cs`).
2. *Elsewhere*: a function of the name anywhere, a `#[test]` before a plain function, but only for an upstream name
   of three words or more. Shorter names (`Parse`, `Root`, `Apply`, `WindowState`) found outside their file were
   names of helpers or of tests of other subjects as often as not (next section).
3. *By alias*: an `[[alias]]` of `docs/porting/data/test-aliases.toml` (the file of
   `scripts/port-status/test_gaps.py`), when every Rust function it names exists. A `[[waive]]` there takes a test
   out of the count with a reason.

Coverage is `found / (tests - waived)`, rounded down.

**What this does not show.**

- *Same name, other test.* A function of the right name counts in its file even when it tests something else, and
  outside its file when the name has three words or more. The calibration below found no such case in its sample.
- *Ported under another name.* Counts as missing until it is renamed or aliased; several upstream tests merged into
  one Rust test need an alias each.
- *Behaviour.* A found test may be ignored (`#[ignore]`), have fewer rows than upstream's theory, or assert less.
  Whether the suites pass is what `cargo test` shows, not this document.
- *Commit.* Upstream test projects added after the tracked commit are not read until the port moves to a newer
  commit.

`scripts/port-status/test_gaps.py` reports one project in detail (also the waived tests and the aliases) with the
same upstream rules. It counts more Rust functions as test code (every function of a file that has a `#[test]`
anywhere, production code included) and matches by number of names per project, not function by function, so its
count of present tests is higher by the false matches the next section lists.
"""

CALIBRATION = """\
## Calibration by hand (2026-10-16)

The rules were checked by hand before the numbers were taken, on `Avalonia.Base.UnitTests`,
`Avalonia.Controls.UnitTests`, `Avalonia.Markup.UnitTests`, `Avalonia.Benchmarks` and the XamlX tests.

**Found tests that are not the port of the test.** Under the looser rule of `test_gaps.py` four tests were found
through a function of another subject, all with a name of one or two words: `Style_ClassSelector.Apply` of the
benchmarks (a helper `apply` of the animation tests; the README of `tests/FerroUI.Benchmarks` lists it as not
ported), `PixelSizeTests.Parse` and `VisualExtensions_GetTransformedBounds.Root` of the base tests (helpers of the
XamlX tests, while their own ports in `pixel_size.rs` and `visual_extensions_get_transformed_bounds_tests.rs` went to
other tests of the name), and `WindowTests.WindowState` of the Appium tests (a helper of `platform_manager_tests.rs`).
One more was a test of the same name of another file: `AvaloniaObjectTests_Direct.SetValue_Should_Not_Cause_StackOverflow_And_Have_Correct_Values`
has no port in `ferro_object_tests_direct.rs`; the two Rust functions of the name are the ports of the test of
`AvaloniaObjectTests_Binding.cs` and of the one of the markup `BindingTests.cs`. The assignment of one function to
one test and the three-word rule are what this found; both tests now count as missing. Then a sample was opened in
the port: 20 tests found in their file and 20 found elsewhere in `Avalonia.Base.UnitTests`, 15 found in their file
and 20 found elsewhere in `Avalonia.Controls.UnitTests`, and all 22 found elsewhere in `Avalonia.Markup.UnitTests`.
Every one of the 97 is the port of the upstream test of its name (the tests found elsewhere are in files that hold
several upstream files: `input/input_tests.rs`, `styling/style_tests.rs`, the `binding_*_tests.rs` of the base
crate, `application_lifetimes/classic_desktop_style_application_lifetime_tests.rs`).

**Missing tests that exist under another name.** `XamlParserTests`: of 35 tests missing by name, 24 were ported as
merged tests (one Rust test for the rows of several upstream tests: the dynamic setters, the boolean extension, the
simple compile and populate, the non-boxed conversions) or under a changed name; they have aliases now. Three
(`UtilityTests.cs`) are inside `#if GUILABS` and are waived. Eight are missing. `Avalonia.Benchmarks`: the 21
missing benchmarks are the files and benchmarks that `tests/FerroUI.Benchmarks/README.md` lists as not ported.
`Avalonia.DesignerSupport.Tests`: the one missing test is not in the port under another name. `TypeSystemTest`
(5) checks the two .NET type systems of XamlX (System.Reflection and Cecil over compiled assemblies), which the port
does not have; it is listed as not applicable.

**Estimated accuracy.** Of the sample of found tests, 97 of 97 are true. Of the 58 tests missing by name in the
projects checked (XamlParserTests, the benchmarks, designer support, base), 24 were in the port under other names
(now aliased) and 3 do not apply (waived); the other 31 are true gaps. The remaining risk is
in tests merged or renamed in projects that were not checked one by one; their "missing" lists are short (every
applicable project but the Appium tests and the benchmarks has at most eight).
"""


# --------------------------------------------------------------------------------------------------------------
# Upstream inventory
# --------------------------------------------------------------------------------------------------------------


def git(upstream: str, *args: str) -> bytes:
    result = subprocess.run(["git", "-C", upstream, *args], capture_output=True)
    if result.returncode != 0:
        sys.exit(f"git -C {upstream} {' '.join(args)}: {result.stderr.decode(errors='replace').strip()}")
    return result.stdout


def archive(upstream: str, rev: str, prefix: str) -> tuple[dict[str, str], list[str]]:
    """The `.cs` texts and the project files below `prefix` at `rev`."""
    texts: dict[str, str] = {}
    projects: list[str] = []
    data = git(upstream, "archive", "--format=tar", rev, prefix)
    with tarfile.open(fileobj=io.BytesIO(data)) as tar:
        for member in tar:
            if not member.isfile():
                continue
            if member.name.endswith(".cs"):
                texts[member.name] = tar.extractfile(member).read().decode("utf-8-sig", "replace")
            elif member.name.endswith(".csproj"):
                projects.append(os.path.dirname(member.name))
    return texts, projects


def extract(upstream: str, commit: str) -> dict:
    """Reads every upstream test project and returns the inventory."""
    texts, project_dirs = archive(upstream, commit, "tests")
    project_dirs += SOURCE_DIRECTORIES
    submodules = {}
    tree = git(upstream, "ls-tree", commit, "external/").decode()
    for line in tree.splitlines():
        meta, name = line.split("\t", 1)
        mode, kind, sha = meta.split()
        if kind == "commit" and name in SUBMODULES:
            submodules[name] = sha
            sub_texts, sub_projects = archive(os.path.join(upstream, name), sha, "tests")
            texts.update({f"{name}/{k}": v for k, v in sub_texts.items()})
            project_dirs += [f"{name}/{p}" for p in sub_projects]
    project_dirs = sorted(set(project_dirs), key=lambda p: (-p.count("/"), p))

    projects: dict[str, dict] = {}
    for directory in sorted(project_dirs):
        projects[directory] = {"csFiles": 0, "files": {}}
    for path in sorted(texts):
        owner = next((d for d in project_dirs if path.startswith(d + "/")), None)
        if owner is None:
            pairs = tg.upstream_tests(texts[path])
            if pairs:
                sys.exit(f"{path} has tests but belongs to no project")
            continue
        project = projects[owner]
        project["csFiles"] += 1
        pairs = tg.upstream_tests(texts[path])
        if pairs:
            project["files"][path[len(owner) + 1:]] = [f"{c}.{m}" for c, m in pairs]
    return {
        "generator": "scripts/port-status/tests_audit.py --extract",
        "upstreamCommit": commit,
        "submodules": submodules,
        "projects": projects,
    }


# --------------------------------------------------------------------------------------------------------------
# Port side
# --------------------------------------------------------------------------------------------------------------

CFG_TEST_MOD_DECL = re.compile(
    r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]"
    r"((?:\s*#\s*\[[^\]]*\])*)"
    r"\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_]\w*)\s*([;{])"
)
PATH_ATTRIBUTE = re.compile(r'#\s*\[\s*path\s*=\s*"([^"]+)"\s*\]')
FN_TEST = tg.RUST_TEST  # #[test] (#[...])* fn name


class Function:
    __slots__ = ("name", "path", "kind", "contexts")

    def __init__(self, name: str, path: str, kind: str, contexts: frozenset):
        self.name = name
        self.path = path
        self.kind = kind  # "test" (a #[test] or a test macro writes it) or "helper"
        self.contexts = contexts  # normalised names of its file and of the inline modules around it


def normalise_context(name: str) -> str:
    """A file, module or class name as compared by the file rule: lower case, no underscores, `Avalonia` read as
    `Ferro`, and the words `tests` / `test` dropped (`SelectorTests_Child` and `selector_tests_child` and
    `selector_child` are one name)."""
    name = re.sub(r"[^a-z0-9]", "", name.lower()).replace("avalonia", "ferro")
    return name.replace("tests", "").replace("test", "")


def contexts_match(upstream: frozenset, port: frozenset, exact: bool) -> bool:
    """The same name, or (`exact` false) one name the start of the other, of five characters or more
    (`input_tests.rs` holds `InputElement_Focus.cs`)."""
    for u in upstream:
        for p in port:
            if u == p:
                return True
            shorter = min(len(u), len(p))
            if not exact and shorter >= 5 and (u.startswith(p) or p.startswith(u)):
                return True
    return False


def module_file(file_path: str, name: str, path_attribute: str | None) -> list[str]:
    """The candidate files of `mod name;` declared in `file_path`."""
    directory = os.path.dirname(file_path)
    if path_attribute:
        return [os.path.normpath(os.path.join(directory, path_attribute))]
    stem = os.path.splitext(os.path.basename(file_path))[0]
    if stem not in ("mod", "lib", "main"):
        directory = os.path.join(directory, stem)
    return [os.path.join(directory, name + ".rs"), os.path.join(directory, name, "mod.rs")]


INLINE_MOD = re.compile(r"\bmod\s+([A-Za-z_]\w*)\s*\{")


def file_contexts(relative: str) -> set[str]:
    stem = os.path.splitext(os.path.basename(relative))[0]
    names = {stem}
    if stem in ("mod", "tests", "lib", "main"):
        names.add(os.path.basename(os.path.dirname(relative)))
    return {n for n in map(normalise_context, names) if n}


def scan_port(repo: str, roots: list[str]) -> dict[str, list[Function]]:
    files: dict[str, tuple[str, str]] = {}  # relative path -> (text, blanked)
    for root in roots:
        base = os.path.join(repo, root)
        for directory, directories, names in os.walk(base):
            directories[:] = sorted(d for d in directories if d not in ("target", ".git", "node_modules"))
            for name in sorted(names):
                if name.endswith(".rs"):
                    full = os.path.join(directory, name)
                    with open(full, encoding="utf-8", errors="replace") as handle:
                        text = handle.read()
                    files[os.path.relpath(full, repo).replace(os.sep, "/")] = (text, "")

    # Pass 1: the macros that write tests, and the files and directories of `#[cfg(test)] mod x;`.
    test_macros: set[str] = set()
    test_files: set[str] = set()
    test_dirs: set[str] = set()
    for relative in list(files):
        text, _ = files[relative]
        blanked = tg.blank_rust(text) if ("test" in text or tg.is_test_file(relative)) else ""
        files[relative] = (text, blanked)
        if not blanked:
            continue
        for m in tg.MACRO_RULES.finditer(blanked):
            end = tg.balanced_end(blanked, m.end() - 1)
            if tg.RUST_TEST_ATTRIBUTE.search(blanked, m.end(), end):
                test_macros.add(m.group(1))
        for m in CFG_TEST_MOD_DECL.finditer(blanked):
            if m.group(3) != ";":
                continue
            attributes = PATH_ATTRIBUTE.search(text[m.start(1):m.end(1)])
            for candidate in module_file(relative, m.group(2), attributes.group(1) if attributes else None):
                candidate = candidate.replace(os.sep, "/")
                if candidate in files:
                    test_files.add(candidate)
                    stem = candidate[:-3] if not candidate.endswith("/mod.rs") else candidate[:-len("/mod.rs")]
                    test_dirs.add(stem + "/")
                    break

    def whole_file(relative: str) -> bool:
        return tg.is_test_file(relative) or relative in test_files or any(relative.startswith(d) for d in test_dirs)

    macro_call = None
    if test_macros:
        macro_call = re.compile(r"\b(" + "|".join(sorted(map(re.escape, test_macros))) + r")\s*!\s*[{(\[]")

    found: dict[str, list[Function]] = defaultdict(list)
    for relative in sorted(files):
        text, blanked = files[relative]
        if not blanked:
            continue
        whole = whole_file(relative)
        own = file_contexts(relative)
        spans = [(m.end() - 1, tg.balanced_end(blanked, m.end() - 1), normalise_context(m.group(1)))
                 for m in INLINE_MOD.finditer(blanked)]

        def contexts(position: int) -> frozenset:
            return frozenset(own | {n for start, end, n in spans if start < position < end and n})

        direct = {m.group(1) for m in FN_TEST.finditer(blanked)}
        # The names a test macro writes: the `fn` names inside its calls, else the identifiers handed to it.
        written: list[tuple[str, int]] = []
        if macro_call:
            for call in macro_call.finditer(blanked):
                if blanked[:call.start()].rstrip().endswith("macro_rules"):
                    continue
                end = tg.balanced_end(blanked, call.end() - 1)
                declared_in_call = [(f.group(1), f.start(1)) for f in tg.RUST_FN.finditer(blanked, call.end(), end)]
                written += declared_in_call or [(i.group(0), i.start()) for i in tg.IDENTIFIER.finditer(blanked, call.end(), end)]
        tests = direct | {n for n, _ in written}
        positions: dict[int, str] = {}
        if whole:
            for f in tg.RUST_FN.finditer(blanked):
                positions[f.start(1)] = f.group(1)
        else:
            for m in CFG_TEST_MOD_DECL.finditer(blanked):
                if m.group(3) == "{":
                    end = tg.balanced_end(blanked, m.end() - 1)
                    for f in tg.RUST_FN.finditer(blanked, m.end(), end):
                        positions[f.start(1)] = f.group(1)
            for f in FN_TEST.finditer(blanked):
                positions[f.start(1)] = f.group(1)
        declared = set(positions.values())
        seen: set[str] = set()
        for name, position in written:
            if name not in declared and name not in seen:
                seen.add(name)
                positions.setdefault(position, name)
        for position in sorted(positions):
            function = positions[position]
            kind = "test" if function in tests else "helper"
            found[normalise_rust(function)].append(Function(function, relative, kind, contexts(position)))
    return found


def normalise_rust(name: str) -> str:
    return tg.normalise_rust(name)


# --------------------------------------------------------------------------------------------------------------
# Matching
# --------------------------------------------------------------------------------------------------------------

WORD = re.compile(r"[A-Z]+(?![a-z])|[A-Z]?[a-z]+|[0-9]+")


def words(name: str) -> int:
    return len(WORD.findall(name))


# A test found outside the file of its name needs a name of at least this many words: shorter names (`Parse`,
# `Root`, `WindowState`, `SetValues`) are names of helpers and of tests of other subjects as often as not.
ELSEWHERE_MIN_WORDS = 3


def short_name(project_path: str) -> str:
    return project_path.rsplit("/", 1)[1] if not project_path.endswith("/src") else project_path.split("/")[-2]


def match_all(inventory: dict, applicable: list[str], port: dict[str, list[Function]]) -> dict[str, dict]:
    """Assigns Rust functions to upstream tests over all applicable projects at once; one function is the port of
    at most one upstream test. Pass 1: a function in the file (or inline module) named after the upstream file or
    class, the same name first, then one name the start of the other. Pass 2: a `#[test]` function anywhere, for a name of ELSEWHERE_MIN_WORDS words or more. Pass 3: any
    function of test code, for such a name. Then the aliases and waivers of test-aliases.toml."""
    records = []
    for path in applicable:
        for file in sorted(inventory["projects"][path]["files"]):
            entries = inventory["projects"][path]["files"][file]
            several = len({e.split(".", 1)[0] for e in entries}) > 1
            stem = os.path.splitext(os.path.basename(file))[0]
            for entry in entries:
                owner, method = entry.split(".", 1)
                upstream_contexts = frozenset(n for n in (normalise_context(stem), normalise_context(owner)) if n)
                records.append({"project": path, "file": file, "entry": entry, "method": method,
                                "shown": entry if several else method, "key": tg.normalise_upstream(method),
                                "contexts": upstream_contexts, "how": None, "function": None})
    used: set[int] = set()

    def assign(record, accept, how):
        for function in port.get(record["key"], ()):
            if id(function) not in used and accept(record, function):
                used.add(id(function))
                record["how"] = how
                record["function"] = function
                return

    for record in records:
        assign(record, lambda r, f: contexts_match(r["contexts"], f.contexts, True), "file")
    for record in records:
        if record["how"] is None:
            assign(record, lambda r, f: contexts_match(r["contexts"], f.contexts, False), "file")
    for kind in ("test", "helper"):
        for record in records:
            if record["how"] is None and words(record["method"]) >= ELSEWHERE_MIN_WORDS:
                assign(record, lambda r, f, k=kind: f.kind == k, "elsewhere")

    results: dict[str, dict] = {}
    rules = {path: tg.load_rules(ALIASES, short_name(path)) for path in applicable}
    for path in applicable:
        results[path] = {"tests": 0, "found": 0, "inFile": 0, "elsewhere": 0, "aliased": 0, "waived": 0, "missing": 0,
                         "missingTests": {}, "waivedTests": {}, "explain": []}
    for record in records:
        result = results[record["project"]]
        aliases, waivers = rules[record["project"]]
        file, shown, method, entry = record["file"], record["shown"], record["method"], record["entry"]
        result["tests"] += 1
        if record["how"]:
            function = record["function"]
            result["found"] += 1
            result["inFile" if record["how"] == "file" else "elsewhere"] += 1
            result["explain"].append((file, shown, f"{record['how']}: {function.name} ({function.kind}) {function.path}"))
            continue
        alias = tg.find_rule(aliases, file, method, entry)
        if alias is not None and alias["rust"] and all(normalise_rust(r) in port for r in alias["rust"]):
            result["found"] += 1
            result["aliased"] += 1
            result["explain"].append((file, shown, "alias: " + ", ".join(alias["rust"])))
            continue
        waiver = tg.find_rule(waivers, file, method, entry)
        if waiver is not None:
            result["waived"] += 1
            result["waivedTests"].setdefault(file, []).append((shown, waiver["reason"]))
            result["explain"].append((file, shown, "waived: " + waiver["reason"]))
            continue
        result["missing"] += 1
        result["missingTests"].setdefault(file, []).append(shown)
        candidates = port.get(record["key"], [])
        detail = "; not assigned: " + ", ".join(f"{f.name} ({f.kind}) {f.path}" for f in candidates[:3]) if candidates else ""
        result["explain"].append((file, shown, "MISSING" + detail))
    return results


# --------------------------------------------------------------------------------------------------------------
# Output
# --------------------------------------------------------------------------------------------------------------


KEYS = ("tests", "found", "inFile", "elsewhere", "aliased", "waived", "missing")


def percent(found: int, total: int) -> str:
    # Rounded down, so that 100.0% means nothing is missing.
    return "n/a" if total == 0 else f"{(1000 * found // total) / 10:.1f}%"


def plural(count: int, noun: str) -> str:
    return f"{count} {noun}" + ("" if count == 1 else "s")


def anchor(text: str) -> str:
    return re.sub(r"[^a-z0-9 -]", "", text.lower()).replace(" ", "-")


def render(inventory: dict, results: dict[str, dict]) -> tuple[str, str]:
    commit = inventory["upstreamCommit"]
    subs = inventory.get("submodules", {})
    applicable = [p for p in inventory["projects"] if PROJECTS[p].get("status") != NA]
    not_applicable = [p for p in inventory["projects"] if PROJECTS[p].get("status") == NA]
    order = sorted(applicable, key=lambda p: (-results[p]["tests"], p))

    total = defaultdict(int)
    for p in applicable:
        for k in KEYS:
            total[k] += results[p][k]

    w = []
    w.append("# Upstream tests in the port")
    w.append("")
    w.append("<!-- Generated by scripts/port-status/tests_audit.py. Do not edit: change the script, "
             "docs/porting/data/test-aliases.toml or the Rust tree and regenerate. -->")
    w.append("")
    sub_text = ", ".join(f"`{k}` at `{v}`" for k, v in sorted(subs.items()))
    w.append(f"Upstream commit `{commit}`" + (f" ({sub_text})" if sub_text else "") + ".")
    w.append("")
    w.append("How completely the upstream test suites are ported, per upstream test project: the upstream test methods, "
             "how many of them the port has by name, how many are waived with a reason, and the names of the missing ones. "
             "The member counts are in [TRACKING.md](TRACKING.md); what is left overall is in [REMAINING.md](REMAINING.md).")
    w.append("")
    w.append(f"**{total['found']} of {total['tests'] - total['waived']} upstream tests that apply are found "
             f"({percent(total['found'], total['tests'] - total['waived'])})**: {total['tests']} tests in "
             f"{len(applicable)} applicable projects, {total['waived']} waived, {total['missing']} missing. "
             f"{sum(results[p]['tests'] for p in not_applicable)} more tests are in {len(not_applicable)} projects that do "
             "not apply to the port.")
    w.append("")
    w.append("```sh")
    w.append("python3 scripts/port-status/tests_audit.py            # regenerate this document and data/tests-status.json")
    w.append("python3 scripts/port-status/tests_audit.py --check    # exit 1 when they are out of date (also run by run.sh --check)")
    w.append("python3 scripts/port-status/tests_audit.py --extract  # re-read the upstream test projects (UPSTREAM=/path/to/checkout)")
    w.append("python3 scripts/port-status/tests_audit.py --explain Avalonia.Base.UnitTests 'Styling/*'   # what matched each test")
    w.append("```")
    w.append("")
    w.append("## Applicable projects")
    w.append("")
    w.append("| Upstream test project | Tests | Found | in its file | elsewhere | by alias | Waived | Missing | Coverage | Port |")
    w.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|---|")
    for p in order:
        r = results[p]
        name = f"[`{p}`](#{anchor(short_name(p))})" if r["missing"] else f"`{p}`"
        w.append(f"| {name} | {r['tests']} | {r['found']} | {r['inFile']} | {r['elsewhere']} | {r['aliased']} | {r['waived']} "
                 f"| {r['missing']} | {percent(r['found'], r['tests'] - r['waived'])} | {PROJECTS[p]['rust']} |")
    w.append(f"| Total | {total['tests']} | {total['found']} | {total['inFile']} | {total['elsewhere']} | {total['aliased']} "
             f"| {total['waived']} | {total['missing']} | {percent(total['found'], total['tests'] - total['waived'])} | |")
    w.append("")
    notes = [p for p in order if PROJECTS[p].get("note")]
    if notes:
        w.append("Notes:")
        w.append("")
        for p in notes:
            w.append(f"- `{short_name(p)}`: {PROJECTS[p]['note']}.")
        w.append("")
    w.append("## Projects that do not apply")
    w.append("")
    w.append("| Upstream test project | C# files | Tests | Reason |")
    w.append("|---|---:|---:|---|")
    for p in sorted(not_applicable):
        w.append(f"| `{p}` | {inventory['projects'][p]['csFiles']} | {results[p]['tests']} | {PROJECTS[p]['reason']} |")
    w.append("")
    w.append(RULES)
    w.append(CALIBRATION)
    w.append("## Missing tests by project")
    w.append("")
    w.append("Per project, the upstream files with tests the port does not have by name (`missing/tests of the file`), and "
             "the missing names. Waived tests are in the reports of `test_gaps.py` (`docs/porting/data/test-gaps-*.txt`).")
    for p in order:
        r = results[p]
        if not r["missing"]:
            continue
        files = r["missingTests"]
        counts = {f: len(e) for f, e in inventory["projects"][p]["files"].items()}
        w.append("")
        w.append(f"### {short_name(p)}")
        w.append("")
        w.append(f"{r['missing']} missing of {r['tests']} (`{p}`).")
        w.append("")
        w.append("<details>")
        w.append(f"<summary>{plural(r['missing'], 'missing test')} in {plural(len(files), 'file')}</summary>")
        w.append("")
        for f in sorted(files, key=lambda f: (-len(files[f]), f)):
            w.append(f"- `{f}` ({len(files[f])}/{counts[f]}): " + ", ".join(f"`{n}`" for n in files[f]))
        w.append("")
        w.append("</details>")
    document = "\n".join(w).rstrip() + "\n"

    status = {
        "generator": "scripts/port-status/tests_audit.py",
        "upstreamCommit": commit,
        "submodules": subs,
        "totals": {k: total[k] for k in KEYS},
        "projects": [],
    }
    for p in sorted(inventory["projects"]):
        r = results[p]
        entry = {"path": p, "name": short_name(p), "applicable": PROJECTS[p].get("status") != NA,
                 "csFiles": inventory["projects"][p]["csFiles"], "tests": r["tests"]}
        if entry["applicable"]:
            entry.update({k: r[k] for k in KEYS if k != "tests"})
            entry["port"] = PROJECTS[p]["rust"]
            entry["missingTests"] = [{"file": f, "test": n} for f in sorted(r["missingTests"]) for n in r["missingTests"][f]]
        else:
            entry["reason"] = PROJECTS[p]["reason"]
        status["projects"].append(entry)
    return document, json.dumps(status, indent=1, ensure_ascii=False) + "\n"


# --------------------------------------------------------------------------------------------------------------


def load_inventory() -> dict | None:
    if not os.path.exists(INVENTORY):
        return None
    with open(INVENTORY, encoding="utf-8") as handle:
        return json.load(handle)


def write_if_changed(path: str, text: str) -> bool:
    try:
        with open(path, encoding="utf-8") as handle:
            if handle.read() == text:
                return False
    except OSError:
        pass
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(text)
    return True


def compute(inventory: dict, port: dict) -> dict[str, dict]:
    unknown = [p for p in inventory["projects"] if p not in PROJECTS]
    if unknown:
        sys.exit("upstream test projects not decided on in PROJECTS of tests_audit.py: " + ", ".join(unknown))
    applicable = [p for p in inventory["projects"] if PROJECTS[p].get("status") != NA]
    results = match_all(inventory, applicable, port)
    for p, project in inventory["projects"].items():
        if p not in applicable:
            results[p] = {"tests": sum(len(e) for e in project["files"].values())}
    return results


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="exit 1 when the outputs are out of date; writes nothing")
    parser.add_argument("--extract", action="store_true", help="re-read the upstream test projects")
    parser.add_argument("--upstream", default=os.environ.get("UPSTREAM", os.path.join(REPO, "..", "Avalonia")))
    parser.add_argument("--explain", nargs="+", metavar=("PROJECT", "GLOB"),
                        help="print each test of a project (files matching GLOB) and what matched it")
    arguments = parser.parse_args()

    tracked = tg.tracked_commit()
    inventory = load_inventory()
    if arguments.check:
        if inventory is None:
            print(f"error: {os.path.relpath(INVENTORY, REPO)} is missing", file=sys.stderr)
            return 2
    elif arguments.extract or inventory is None or inventory.get("upstreamCommit") != tracked:
        if not os.path.isdir(arguments.upstream):
            sys.exit(f"upstream checkout not found at {arguments.upstream} (set UPSTREAM or --upstream)")
        inventory = extract(os.path.abspath(arguments.upstream), tracked)
        if write_if_changed(INVENTORY, json.dumps(inventory, indent=1, ensure_ascii=False) + "\n"):
            print(f"tests-audit: wrote {os.path.relpath(INVENTORY, REPO)}", file=sys.stderr)

    port = scan_port(REPO, RUST_ROOTS)
    results = compute(inventory, port)

    if arguments.explain:
        wanted = arguments.explain[0]
        pattern = arguments.explain[1] if len(arguments.explain) > 1 else "*"
        paths = [p for p in inventory["projects"] if short_name(p) == wanted or p == wanted]
        if not paths or "explain" not in results[paths[0]]:
            sys.exit(f"no applicable project {wanted}")
        for file, name, how in results[paths[0]]["explain"]:
            if fnmatch.fnmatchcase(file, pattern):
                print(f"{file}\t{name}\t{how}")
        return 0

    document, status = render(inventory, results)
    if arguments.check:
        stale = []
        for path, text in ((DOCUMENT, document), (STATUS, status)):
            try:
                with open(path, encoding="utf-8") as handle:
                    if handle.read() != text:
                        stale.append(path)
            except OSError:
                stale.append(path)
        for path in stale:
            print("out of date:", os.path.relpath(path, REPO))
        return 1 if stale else 0
    changed = write_if_changed(DOCUMENT, document) + write_if_changed(STATUS, status)
    totals = json.loads(status)["totals"]
    print(f"tests-audit: {totals['found']} of {totals['tests'] - totals['waived']} applicable upstream tests found, "
          f"{totals['missing']} missing, {totals['waived']} waived; {changed} of 2 documents updated")
    return 0


if __name__ == "__main__":
    sys.exit(main())
