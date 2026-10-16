#!/usr/bin/env python3
"""Builds an HTML report of a run of the WinUI theme gallery.

    python3 scripts/winui-theme/report.py <output directory> [--out <report.html>] [--title <title>]

The directory is the output of one of the two runs of `samples/WinUIThemeGallery`
(docs/porting/winui-theme-gallery.md):

- the comparison host of Windows (`winui-theme-compare`): `compare.tsv` lists one comparison per
  line (the page, the setting, the share of the pixels that differ, the number of differing
  pixels, the size, the three pictures and the problems of the framework's page). The report has
  one row per comparison, sorted with the largest difference first: the native WinUI 3 page, the
  page of the framework in the theme under test and the picture of their difference side by
  side, with the share of the pixels that differ. A summary per page and per setting is above
  the rows.
- the capture mode of the gallery (`FERROUI_THEME_GALLERY_CAPTURE`): the pictures
  `<page>-<setting>.png`. The report has one row per page with its pictures in every setting.

The report is written into the directory (`report.html` by default) and names the pictures by
their file names, so the directory can be copied with it. Standard library only.
"""

import html
import os
import sys
from collections import OrderedDict, defaultdict

STYLE = """
body { font-family: Segoe UI, system-ui, sans-serif; margin: 24px; color: #1b1b1b; background: #fafafa; }
h1 { font-size: 24px; font-weight: 600; }
h2 { font-size: 18px; font-weight: 600; margin-top: 32px; }
table { border-collapse: collapse; margin: 8px 0 16px 0; }
th, td { border: 1px solid #d0d0d0; padding: 4px 8px; text-align: left; vertical-align: top; font-size: 13px; }
th { background: #eeeeee; }
td.number { text-align: right; font-variant-numeric: tabular-nums; }
.comparison { border: 1px solid #d0d0d0; background: #ffffff; margin: 16px 0; padding: 12px; }
.comparison h3 { font-size: 15px; margin: 0 0 8px 0; }
.pictures { display: flex; gap: 12px; flex-wrap: wrap; }
.pictures figure { margin: 0; }
.pictures figcaption { font-size: 12px; color: #555555; margin-bottom: 4px; }
.pictures img { max-width: 420px; border: 1px solid #c0c0c0; image-rendering: pixelated; }
.bad { color: #a4262c; font-weight: 600; }
.note { color: #555555; font-size: 13px; }
"""


def read_table(path):
    """The lines of `compare.tsv` as dictionaries by the names of its header."""
    with open(path, encoding="utf-8") as f:
        lines = [line.rstrip("\n").split("\t") for line in f if line.strip()]
    if not lines:
        return []
    header, rows = lines[0], lines[1:]
    result = []
    for row in rows:
        entry = dict(zip(header, row))
        try:
            entry["percent"] = float(entry.get("percent", "0"))
        except ValueError:
            entry["percent"] = 0.0
        try:
            entry["problems"] = int(entry.get("problems", "0") or "0")
        except ValueError:
            entry["problems"] = 0
        result.append(entry)
    return result


def picture(directory, name, caption):
    """A figure with the picture `name` of the directory, or a note that it is missing."""
    if not name or not os.path.isfile(os.path.join(directory, name)):
        return '<figure><figcaption>%s</figcaption><p class="note">(no picture)</p></figure>' % html.escape(caption)
    link = html.escape(name)
    return '<figure><figcaption>%s</figcaption><a href="%s"><img src="%s" alt="%s" loading="lazy"></a></figure>' % (
        html.escape(caption),
        link,
        link,
        html.escape(caption),
    )


def summary(rows, key):
    """The rows of a summary table: per value of `key`, the number of comparisons, the mean and
    the largest share of differing pixels, the worst first."""
    groups = defaultdict(list)
    for row in rows:
        groups[row.get(key, "")].append(row["percent"])
    lines = []
    for value, percents in groups.items():
        lines.append((value, len(percents), sum(percents) / len(percents), max(percents)))
    lines.sort(key=lambda line: line[2], reverse=True)
    return lines


def comparison_report(directory, rows, title):
    rows = sorted(rows, key=lambda row: (row["percent"], row.get("page", ""), row.get("setting", "")), reverse=True)
    parts = ["<h1>%s</h1>" % html.escape(title)]
    total = len(rows)
    mean = sum(row["percent"] for row in rows) / total if total else 0.0
    with_problems = sum(1 for row in rows if row["problems"])
    parts.append(
        '<p class="note">%d comparison(s); on average %.2f %% of the pixels differ; %d with problems of the framework\'s page. '
        "A pixel differs when a colour channel differs by more than the threshold of the run. "
        "Each row shows the native WinUI 3 page, the page of the framework in the theme under test and their difference "
        "(red: the pixels that differ, over the native page in grey).</p>" % (total, mean, with_problems)
    )

    for key, heading in (("page", "By page"), ("setting", "By setting")):
        parts.append("<h2>%s</h2>" % heading)
        parts.append('<table><tr><th>%s</th><th>Comparisons</th><th>Mean %%</th><th>Largest %%</th></tr>' % html.escape(key))
        for value, count, average, largest in summary(rows, key):
            parts.append(
                '<tr><td><a href="#%s">%s</a></td><td class="number">%d</td><td class="number">%.2f</td><td class="number">%.2f</td></tr>'
                % (html.escape(value), html.escape(value), count, average, largest)
                if key == "page"
                else '<tr><td>%s</td><td class="number">%d</td><td class="number">%.2f</td><td class="number">%.2f</td></tr>'
                % (html.escape(value), count, average, largest)
            )
        parts.append("</table>")

    parts.append("<h2>Comparisons, the largest difference first</h2>")
    anchored = set()
    for row in rows:
        page = row.get("page", "")
        anchor = ""
        if page not in anchored:
            anchored.add(page)
            anchor = ' id="%s"' % html.escape(page)
        problems = (
            ' <span class="bad">%d problem(s) of the framework\'s page (see the log of the run)</span>' % row["problems"]
            if row["problems"]
            else ""
        )
        parts.append('<div class="comparison"%s>' % anchor)
        parts.append(
            "<h3>%s, %s: %.2f %% of the pixels differ (%s of %s by %s)%s</h3>"
            % (
                html.escape(page),
                html.escape(row.get("setting", "")),
                row["percent"],
                html.escape(row.get("differing", "")),
                html.escape(row.get("width", "")),
                html.escape(row.get("height", "")),
                problems,
            )
        )
        parts.append('<div class="pictures">')
        parts.append(picture(directory, row.get("winui", ""), "WinUI 3"))
        parts.append(picture(directory, row.get("ferroui", ""), "FerroUI, WinUI theme"))
        parts.append(picture(directory, row.get("diff", ""), "Difference"))
        if row.get("ferroui_window"):
            parts.append(picture(directory, row.get("ferroui_window", ""), "FerroUI in the window"))
        parts.append("</div></div>")
    return parts


def capture_report(directory, title):
    """The report of a directory of the capture mode: the pictures `<page>-<setting>.png` by page."""
    pages = OrderedDict()
    for name in sorted(os.listdir(directory)):
        if not name.endswith(".png") or "-" not in name:
            continue
        stem = name[: -len(".png")]
        page, setting = stem.split("-", 1)
        pages.setdefault(page, []).append((setting, name))
    parts = ["<h1>%s</h1>" % html.escape(title)]
    parts.append(
        '<p class="note">%d page(s) of the capture mode of the gallery, each in every setting it was captured in.</p>'
        % len(pages)
    )
    for page, pictures in pages.items():
        parts.append('<div class="comparison" id="%s"><h3>%s</h3><div class="pictures">' % (html.escape(page), html.escape(page)))
        for setting, name in pictures:
            parts.append(picture(directory, name, setting))
        parts.append("</div></div>")
    return parts


def main():
    args = sys.argv[1:]
    if not args or args[0] in ("-h", "--help"):
        print(__doc__)
        return 2
    directory = args[0]
    out = None
    title = "WinUI theme gallery"
    i = 1
    while i < len(args):
        if args[i] == "--out" and i + 1 < len(args):
            out = args[i + 1]
            i += 2
        elif args[i] == "--title" and i + 1 < len(args):
            title = args[i + 1]
            i += 2
        else:
            print("unknown argument: %s" % args[i])
            print(__doc__)
            return 2
    if not os.path.isdir(directory):
        print("not a directory: %s" % directory)
        return 2
    out = out or os.path.join(directory, "report.html")

    table = os.path.join(directory, "compare.tsv")
    if os.path.isfile(table):
        rows = read_table(table)
        parts = comparison_report(directory, rows, title + ": WinUI 3 and FerroUI")
        described = "%d comparison(s)" % len(rows)
    else:
        parts = capture_report(directory, title + ": captures")
        described = "the captures"

    document = (
        '<!DOCTYPE html>\n<html lang="en"><head><meta charset="utf-8"><title>%s</title><style>%s</style></head><body>\n%s\n</body></html>\n'
        % (html.escape(title), STYLE, "\n".join(parts))
    )
    with open(out, "w", encoding="utf-8") as f:
        f.write(document)
    print("report of %s: %s" % (described, out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
