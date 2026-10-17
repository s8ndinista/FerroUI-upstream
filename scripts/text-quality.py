#!/usr/bin/env python3
"""Measures how FerroUI's text compares with native text in two screenshots.

Usage:
    text-quality.py NATIVE.png FERROUI.png [--scale S] [--crop X,Y,W,H] [--rows]
    text-quality.py COMPARE_DIR [--scale S] [--crop X,Y,W,H] [--pages text,buttons] [--settings light,dark]
                    [--window] [--rows]
    text-quality.py CALIBRATION_DIR --variants [--pages textcal] [--settings light,dark] [--window] [--rows]

The second form reads the comparison directory of the WinUI theme gallery
(`<page>-<setting>-winui.png` beside `<page>-<setting>-ferroui.png`, as the
Windows comparison writes them) and reports every pair it finds. With
--window the framework's side is `<page>-<setting>-ferroui-window.png`, the
panel cut out of the picture of the window (side by side only): what the
window shows, with the text rendering of the window, where `-ferroui.png` is
a render target bitmap with the text rendering of a surface without a display.

The third form reads the output of a calibration run of the comparison host
(`FERROUI_WINUI_COMPARE_TEXT_VARIANTS`: a directory per text setting, listed
in `text-variants.tsv`) and prints the pairs of every text setting and one
table of the means per text setting (docs/porting/text-rendering.md, section 10).

--rows adds a line per row of text (a band of ink of the native picture): its
top and height in device pixels, the ink ratio, the solid shares and the
vertical offset of that row alone. On the text calibration page of the gallery
(`textcal`) the rows are named by their size and weight; with --variants the
rows are printed as one table per page and setting, a column per text setting.

What is reported per pair (docs/porting/text-rendering.md, section 7):

- fringe: the mean colour of the ink pixels, max(R,G,B) - min(R,G,B), over
  ink on a neutral background whose text colour is neutral. Greyscale
  anti-aliasing gives about 0; subpixel (ClearType-like) text gives tens.
- ink: the ink mass of the picture, the sum of the coverage of its ink
  pixels, and the ratio FerroUI / native (1.00 is as dark as native; less
  is lighter, thinner text).
- solid: the share of the ink pixels that are covered more than 85 %: how
  solid the stems are.
- dy: the vertical offset of the text rows of FerroUI from those of native,
  in device pixels (positive: FerroUI is lower), the median over the rows
  of text, with the number of rows that were matched.

Coverage is measured against the local background (the lightest pixel
around, or the darkest one in a dark theme) and the colour of the text,
which is taken from the native picture so that both pictures are measured
against the same ink. Pixels in long straight runs (borders, rules, fills)
are left out so that the numbers are about text. The numbers are a
heuristic for comparing two renderings of the same page, not absolute
quantities.

Needs Python 3 and Pillow only.
"""

import argparse
import os
import re
import statistics
import sys

from PIL import Image, ImageFilter


def pixels(image):
    """The pixels of a picture as a flat list (Pillow 12 renamed `getdata`)."""
    if hasattr(image, "get_flattened_data"):
        return list(image.get_flattened_data())
    return list(image.getdata())


def load(path, crop):
    image = Image.open(path).convert("RGB")
    if crop:
        x, y, w, h = crop
        image = image.crop((x, y, x + w, y + h))
    return image


class Analysis:
    """The coverage of the ink of one picture."""

    def __init__(self, image, scale, dark, text_level=None):
        self.width, self.height = image.size
        self.scale = scale
        width, height = self.width, self.height

        lum = image.convert("L")
        if dark:
            lum = lum.point(lambda v: 255 - v)
        radius = max(2, round(4 * scale))
        size = 2 * radius + 1
        background = lum.filter(ImageFilter.MaxFilter(size))

        # The colour of the background and of the text around a pixel, per
        # channel: the background must be neutral and so must the text, so
        # that accent text and coloured fills do not count as fringes.
        if dark:
            bg_rgb = image.filter(ImageFilter.MinFilter(size))
            ink_rgb = image.filter(ImageFilter.MaxFilter(size))
        else:
            bg_rgb = image.filter(ImageFilter.MaxFilter(size))
            ink_rgb = image.filter(ImageFilter.MinFilter(size))

        self.lum = pixels(lum)
        self.background = pixels(background)
        self.rgb = pixels(image)
        self.bg_rgb = pixels(bg_rgb)
        self.ink_rgb = pixels(ink_rgb)

        if text_level is None:
            text_level = self.estimate_text_level()
        self.text_level = text_level

        coverage = [0.0] * (width * height)
        for i in range(width * height):
            bg = self.background[i]
            contrast = bg - text_level
            if contrast < 60:
                continue
            c = (bg - self.lum[i]) / contrast
            if c >= 0.1:
                coverage[i] = min(c, 1.0)
        self.remove_runs(coverage, round(24 * scale))
        self.coverage = coverage

    def estimate_text_level(self):
        """The luminance of fully covered text: the first percentile of the
        pixels that are clearly darker than their background."""
        values = sorted(l for l, bg in zip(self.lum, self.background) if bg - l > 40)
        if not values:
            return 0
        return values[len(values) // 100]

    def remove_runs(self, coverage, limit):
        """Leaves out the ink of straight runs longer than `limit` pixels."""
        width, height = self.width, self.height
        drop = bytearray(width * height)
        for y in range(height):
            row = y * width
            x = 0
            while x < width:
                if coverage[row + x] > 0:
                    start = x
                    while x < width and coverage[row + x] > 0:
                        x += 1
                    if x - start >= limit:
                        for i in range(row + start, row + x):
                            drop[i] = 1
                else:
                    x += 1
        for x in range(width):
            y = 0
            while y < height:
                if coverage[y * width + x] > 0:
                    start = y
                    while y < height and coverage[y * width + x] > 0:
                        y += 1
                    if y - start >= limit:
                        for yy in range(start, y):
                            drop[yy * width + x] = 1
                else:
                    y += 1
        for i in range(width * height):
            if drop[i]:
                coverage[i] = 0.0

    def stats(self):
        ink = 0.0
        count = 0
        solid = 0
        fringe_sum = 0
        fringe_count = 0
        for i, c in enumerate(self.coverage):
            if c <= 0:
                continue
            ink += c
            count += 1
            if c > 0.85:
                solid += 1
            bg = self.bg_rgb[i]
            text = self.ink_rgb[i]
            if max(bg) - min(bg) < 12 and max(text) - min(text) < 24:
                r, g, b = self.rgb[i]
                fringe_sum += max(r, g, b) - min(r, g, b)
                fringe_count += 1
        return {
            "ink": ink,
            "count": count,
            "solid": solid / count if count else 0.0,
            "fringe": fringe_sum / fringe_count if fringe_count else 0.0,
        }

    def row_profile(self):
        width = self.width
        return [sum(self.coverage[y * width:(y + 1) * width]) for y in range(self.height)]


def bands(profile, scale):
    """The rows of text: runs of rows with ink, separated by empty rows."""
    threshold = max(1.0, max(profile, default=0) * 0.02)
    result = []
    y = 0
    while y < len(profile):
        if profile[y] > threshold:
            start = y
            while y < len(profile) and profile[y] > threshold:
                y += 1
            if 5 * scale <= y - start <= 60 * scale:
                result.append((start, y))
        else:
            y += 1
    return result


def best_shift(native, ferroui, band, max_shift):
    """The shift of `ferroui` against `native` over the rows of a band that
    correlates best, refined to a fraction of a pixel."""
    start, end = band
    scores = {}
    for s in range(-max_shift, max_shift + 1):
        total = 0.0
        for y in range(start, end):
            yy = y + s
            if 0 <= yy < len(ferroui):
                total += native[y] * ferroui[yy]
        scores[s] = total
    best = max(scores, key=scores.get)
    if scores[best] <= 0:
        return None
    left, right = scores.get(best - 1), scores.get(best + 1)
    if left is not None and right is not None:
        denominator = left - 2 * scores[best] + right
        if denominator < 0:
            return best + 0.5 * (left - right) / denominator
    return float(best)


# The rows of the text calibration page of the gallery, in their order
# (samples/WinUIThemeGallery/Pages/TextCalibrationPage.xaml).
CALIBRATION_ROWS = [
    "12", "13", "14", "14 semibold", "15", "16", "18", "20", "20 semibold", "24", "28", "28 semibold", "32",
]


def band_stats(coverage, width, start, end):
    """The ink and the solid share of the rows start..end of a coverage."""
    ink = 0.0
    count = 0
    solid = 0
    for c in coverage[start * width:end * width]:
        if c > 0:
            ink += c
            count += 1
            if c > 0.85:
                solid += 1
    return ink, (solid / count if count else 0.0)


def row_results(native, ferroui, native_bands, shifts, scale):
    """Per band of the native picture: its top, height, ink ratio, solid
    shares and offset. The ink of a band is summed over the band widened by a
    margin (half the gap to the next band at most), so that a row of FerroUI
    that is a pixel off is measured whole."""
    rows = []
    height = native.height
    for index, (band, shift) in enumerate(zip(native_bands, shifts)):
        start, end = band
        margin = round(2 * scale)
        if index > 0:
            margin = min(margin, (start - native_bands[index - 1][1]) // 2)
        if index + 1 < len(native_bands):
            margin = min(margin, (native_bands[index + 1][0] - end) // 2)
        top, bottom = max(0, start - margin), min(height, end + margin)
        native_ink, native_solid = band_stats(native.coverage, native.width, top, bottom)
        ferroui_ink, ferroui_solid = band_stats(ferroui.coverage, ferroui.width, top, bottom)
        rows.append({
            "top": start,
            "height": end - start,
            "ink_ratio": ferroui_ink / native_ink if native_ink else 0.0,
            "solid": (native_solid, ferroui_solid),
            "dy": shift,
        })
    return rows


def compare(native_path, ferroui_path, scale, crop):
    native_image = load(native_path, crop)
    ferroui_image = load(ferroui_path, crop)
    if native_image.size != ferroui_image.size:
        size = (min(native_image.width, ferroui_image.width), min(native_image.height, ferroui_image.height))
        native_image = native_image.crop((0, 0) + size)
        ferroui_image = ferroui_image.crop((0, 0) + size)
    if scale is None:
        scale = max(1.0, round(native_image.width / 720 * 4) / 4)

    mean = sum(pixels(native_image.convert("L"))) / (native_image.width * native_image.height)
    dark = mean < 128

    native = Analysis(native_image, scale, dark)
    ferroui = Analysis(ferroui_image, scale, dark, native.text_level)
    native_stats = native.stats()
    ferroui_stats = ferroui.stats()

    native_profile = native.row_profile()
    ferroui_profile = ferroui.row_profile()
    shifts = []
    native_bands = bands(native_profile, scale)
    band_shifts = []
    for band in native_bands:
        shift = best_shift(native_profile, ferroui_profile, band, round(6 * scale))
        band_shifts.append(shift)
        if shift is not None:
            shifts.append(shift)

    return {
        "rows_detail": row_results(native, ferroui, native_bands, band_shifts, scale),
        "scale": scale,
        "dark": dark,
        "native": native_stats,
        "ferroui": ferroui_stats,
        "ink_ratio": ferroui_stats["ink"] / native_stats["ink"] if native_stats["ink"] else 0.0,
        "dy": statistics.median(shifts) if shifts else None,
        "rows": len(shifts),
    }


HEADER = (
    f"{'pair':<28} {'scale':>5} {'fringe n/f':>11} {'ink ratio':>9} {'solid n/f':>11} {'dy px':>6} {'rows':>4}"
)


def pair(native, ferroui):
    return f"{native:.0f}%/{ferroui:.0f}%"


def format_row(name, result):
    native, ferroui = result["native"], result["ferroui"]
    dy = "-" if result["dy"] is None else f"{result['dy']:+.2f}"
    return (
        f"{name:<28} {result['scale']:>5g} "
        f"{native['fringe']:>5.1f}/{ferroui['fringe']:<5.1f} "
        f"{result['ink_ratio']:>9.3f} "
        f"{pair(native['solid'] * 100, ferroui['solid'] * 100):>11} "
        f"{dy:>6} {result['rows']:>4}"
    )


def row_name(page, index, count):
    """The name of a row: its size and weight on the calibration page."""
    if page == "textcal" and count == len(CALIBRATION_ROWS):
        return CALIBRATION_ROWS[index]
    return str(index + 1)


def format_rows(page, result):
    rows = result["rows_detail"]
    lines = [f"    {'row':<12} {'top':>5} {'h':>3} {'ink ratio':>9} {'solid n/f':>11} {'dy px':>6}"]
    for index, row in enumerate(rows):
        dy = "-" if row["dy"] is None else f"{row['dy']:+.2f}"
        native_solid, ferroui_solid = row["solid"]
        lines.append(
            f"    {row_name(page, index, len(rows)):<12} {row['top']:>5} {row['height']:>3} {row['ink_ratio']:>9.3f} "
            f"{pair(native_solid * 100, ferroui_solid * 100):>11} {dy:>6}"
        )
    return "\n".join(lines)


def find_pairs(directory, pages, settings, window):
    """The pairs of a compare directory: (name, page, native, ferroui)."""
    suffix = "ferroui-window" if window else "ferroui"
    pairs = []
    for name in sorted(os.listdir(directory)):
        match = re.fullmatch(r"(.+?)-((?:light|dark)(?:-compact)?)-winui\.png", name)
        if not match:
            continue
        page, setting = match.groups()
        if pages and page not in pages or settings and setting not in settings:
            continue
        ferroui = os.path.join(directory, f"{page}-{setting}-{suffix}.png")
        if os.path.isfile(ferroui):
            pairs.append((f"{page}-{setting}", page, os.path.join(directory, name), ferroui))
    return pairs


def mean(values):
    values = [v for v in values if v is not None]
    return sum(values) / len(values) if values else 0.0


def print_mean(results):
    print(
        f"{'mean':<28} {'':>5} "
        f"{mean(r['native']['fringe'] for r in results):>5.1f}/"
        f"{mean(r['ferroui']['fringe'] for r in results):<5.1f} "
        f"{mean(r['ink_ratio'] for r in results):>9.3f} "
        f"{pair(mean(r['native']['solid'] for r in results) * 100, mean(r['ferroui']['solid'] for r in results) * 100):>11} "
        f"{mean(r['dy'] for r in results):>+6.2f}"
    )


def read_variants(directory):
    """The text settings of a calibration run: (name, text rendering,
    baseline) from text-variants.tsv, else the subdirectories."""
    listing = os.path.join(directory, "text-variants.tsv")
    if os.path.isfile(listing):
        variants = []
        with open(listing, encoding="utf-8") as file:
            header = file.readline().rstrip("\n").split("\t")
            for line in file:
                fields = dict(zip(header, line.rstrip("\n").split("\t")))
                if fields.get("name"):
                    variants.append((fields["name"], fields.get("text_rendering", ""), fields.get("baseline", "")))
        return variants
    return [(name, "", "") for name in sorted(os.listdir(directory)) if os.path.isdir(os.path.join(directory, name))]


def main_variants(args, crop):
    directory = args.inputs[0]
    pages = set(args.pages.split(",")) if args.pages else None
    settings = set(args.settings.split(",")) if args.settings else None
    variants = read_variants(directory)
    if not variants:
        sys.exit(f"no text settings in {directory} (text-variants.tsv or a directory per setting)")

    # results[variant][pair name] = result
    results = {}
    names = []
    for variant, rendering, baseline in variants:
        pairs = find_pairs(os.path.join(directory, variant), pages, settings, args.window)
        print(f"== {variant}: text rendering {rendering or '?'}, baseline {baseline or '?'}")
        if not pairs:
            print("   no pairs")
            continue
        print(HEADER)
        results[variant] = {}
        for name, page, native, ferroui in pairs:
            result = compare(native, ferroui, args.scale, crop)
            result["page"] = page
            results[variant][name] = result
            if name not in names:
                names.append(name)
            print(format_row(name, result), flush=True)
        if len(pairs) > 1:
            print_mean(list(results[variant].values()))
        print()

    measured = [variant for variant, _, _ in variants if variant in results]
    print("== the text settings, the means of their pairs (light: dark text on a light background)")
    print(f"{'setting':<24} {'ink light':>9} {'ink dark':>9} {'solid light n/f':>15} {'solid dark n/f':>15} {'dy px':>6} {'pairs':>5}")
    for variant in measured:
        values = list(results[variant].values())
        light = [r for r in values if not r["dark"]]
        dark = [r for r in values if r["dark"]]

        def solid(rs):
            if not rs:
                return "-"
            return pair(mean(r["native"]["solid"] for r in rs) * 100, mean(r["ferroui"]["solid"] for r in rs) * 100)

        ink_light = f"{mean(r['ink_ratio'] for r in light):.3f}" if light else "-"
        ink_dark = f"{mean(r['ink_ratio'] for r in dark):.3f}" if dark else "-"
        print(
            f"{variant:<24} {ink_light:>9} {ink_dark:>9} {solid(light):>15} {solid(dark):>15} "
            f"{mean(r['dy'] for r in values):>+6.2f} {len(values):>5}"
        )

    if args.rows:
        for name in names:
            print()
            print(f"== {name}: per row, ink ratio / dy px, a column per text setting")
            columns = [variant for variant in measured if name in results[variant]]
            first = results[columns[0]][name]
            count = len(first["rows_detail"])
            print(f"{'row':<12} {'top':>5} " + " ".join(f"{variant[:17]:>17}" for variant in columns))
            for index in range(count):
                cells = []
                for variant in columns:
                    rows = results[variant][name]["rows_detail"]
                    if index >= len(rows):
                        cells.append(f"{'-':>17}")
                        continue
                    row = rows[index]
                    dy = "-" if row["dy"] is None else f"{row['dy']:+.2f}"
                    cells.append(f"{row['ink_ratio']:>9.3f} {dy:>7}")
                top = first["rows_detail"][index]["top"]
                print(f"{row_name(first['page'], index, count):<12} {top:>5} " + " ".join(cells))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("inputs", nargs="+", help="NATIVE FERROUI pictures, or a compare directory")
    parser.add_argument("--scale", type=float, help="the render scaling of the pictures (default: width / 720)")
    parser.add_argument("--crop", help="X,Y,W,H: measure only this rectangle of both pictures")
    parser.add_argument("--pages", help="comma-separated pages of a compare directory")
    parser.add_argument("--settings", help="comma-separated settings of a compare directory (light, dark-compact...)")
    parser.add_argument("--window", action="store_true", help="compare with <page>-<setting>-ferroui-window.png")
    parser.add_argument("--rows", action="store_true", help="a line per row of text")
    parser.add_argument("--variants", action="store_true", help="the directory is the output of a calibration run")
    args = parser.parse_args()

    crop = tuple(int(v) for v in args.crop.split(",")) if args.crop else None

    if args.variants:
        if len(args.inputs) != 1 or not os.path.isdir(args.inputs[0]):
            parser.error("--variants takes one directory")
        main_variants(args, crop)
        return

    if len(args.inputs) == 2 and os.path.isfile(args.inputs[0]):
        pairs = [(os.path.basename(args.inputs[1]), "", args.inputs[0], args.inputs[1])]
    elif len(args.inputs) == 1 and os.path.isdir(args.inputs[0]):
        directory = args.inputs[0]
        pages = set(args.pages.split(",")) if args.pages else None
        settings = set(args.settings.split(",")) if args.settings else None
        pairs = find_pairs(directory, pages, settings, args.window)
        if not pairs:
            suffix = "-ferroui-window.png" if args.window else "-ferroui.png"
            sys.exit(f"no <page>-<setting>-winui.png / {suffix} pairs in {directory}")
    else:
        parser.error("give two pictures or one compare directory")

    print(HEADER)
    results = []
    for name, page, native, ferroui in pairs:
        result = compare(native, ferroui, args.scale, crop)
        results.append(result)
        print(format_row(name, result), flush=True)
        if args.rows:
            print(format_rows(page, result), flush=True)

    if len(results) > 1:
        print_mean(results)


if __name__ == "__main__":
    main()
