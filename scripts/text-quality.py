#!/usr/bin/env python3
"""Measures how FerroUI's text compares with native text in two screenshots.

Usage:
    text-quality.py NATIVE.png FERROUI.png [--scale S] [--crop X,Y,W,H]
    text-quality.py COMPARE_DIR [--scale S] [--crop X,Y,W,H] [--pages text,buttons] [--settings light,dark]

The second form reads the comparison directory of the WinUI theme gallery
(`<page>-<setting>-winui.png` beside `<page>-<setting>-ferroui.png`, as the
Windows comparison writes them) and reports every pair it finds.

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
    for band in bands(native_profile, scale):
        shift = best_shift(native_profile, ferroui_profile, band, round(6 * scale))
        if shift is not None:
            shifts.append(shift)

    return {
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


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("inputs", nargs="+", help="NATIVE FERROUI pictures, or a compare directory")
    parser.add_argument("--scale", type=float, help="the render scaling of the pictures (default: width / 720)")
    parser.add_argument("--crop", help="X,Y,W,H: measure only this rectangle of both pictures")
    parser.add_argument("--pages", help="comma-separated pages of a compare directory")
    parser.add_argument("--settings", help="comma-separated settings of a compare directory (light, dark-compact...)")
    args = parser.parse_args()

    crop = tuple(int(v) for v in args.crop.split(",")) if args.crop else None

    if len(args.inputs) == 2 and os.path.isfile(args.inputs[0]):
        pairs = [(os.path.basename(args.inputs[1]), args.inputs[0], args.inputs[1])]
    elif len(args.inputs) == 1 and os.path.isdir(args.inputs[0]):
        directory = args.inputs[0]
        pages = set(args.pages.split(",")) if args.pages else None
        settings = set(args.settings.split(",")) if args.settings else None
        pairs = []
        for name in sorted(os.listdir(directory)):
            match = re.fullmatch(r"(.+?)-((?:light|dark)(?:-compact)?)-winui\.png", name)
            if not match:
                continue
            page, setting = match.groups()
            if pages and page not in pages or settings and setting not in settings:
                continue
            ferroui = os.path.join(directory, f"{page}-{setting}-ferroui.png")
            if os.path.isfile(ferroui):
                pairs.append((f"{page}-{setting}", os.path.join(directory, name), ferroui))
        if not pairs:
            sys.exit(f"no <page>-<setting>-winui.png / -ferroui.png pairs in {directory}")
    else:
        parser.error("give two pictures or one compare directory")

    print(HEADER)
    results = []
    for name, native, ferroui in pairs:
        result = compare(native, ferroui, args.scale, crop)
        results.append(result)
        print(format_row(name, result), flush=True)

    if len(results) > 1:
        def mean(values):
            values = [v for v in values if v is not None]
            return sum(values) / len(values) if values else 0.0

        print(
            f"{'mean':<28} {'':>5} "
            f"{mean(r['native']['fringe'] for r in results):>5.1f}/"
            f"{mean(r['ferroui']['fringe'] for r in results):<5.1f} "
            f"{mean(r['ink_ratio'] for r in results):>9.3f} "
            f"{pair(mean(r['native']['solid'] for r in results) * 100, mean(r['ferroui']['solid'] for r in results) * 100):>11} "
            f"{mean(r['dy'] for r in results):>+6.2f}"
        )


if __name__ == "__main__":
    main()
