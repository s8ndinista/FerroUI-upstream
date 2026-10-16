# Text rendering quality

How FerroUI renders text through Skia, how that compares with the native text of each platform, and the defaults FerroUI chooses so that its text is as sharp and as dark as the platform's own. Windows, with native WinUI 3 as the reference, came first: the measurements below are from the WinUI theme gallery compared with native WinUI 3 (`winui-theme-gallery.md`).

The Vello backend (`src/Vello/FerroUI.Vello`) draws glyphs itself and is not affected by anything here; its text rendering is the owner's to change.

## 1. Findings in short

- **The text lines are not shorter.** TextBlocks have the same line pitch as WinUI (46 px per 23 dip line at 200 %). Inside a TextBox native WinUI uses RichEdit, which stacks lines at whole pixels (18 px at 14 px where the font's metrics give 18.62), and FerroUI's ScrollViewer shows one more partial line because of the clip, not because of the line height. Segoe UI Variable and Segoe UI have the same metrics (2048 units per em, ascent 2210, descent 514, no line gap) in `hhea` and `OS/2`.
- **It is not a Skia defect; it is how FerroUI configured Skia.** FerroUI (as the reference does) asked Skia for subpixel (LCD, ClearType-like) text on every surface, assumed an RGB display (`PixelGeometry::RGBH`) and kept Skia's default contrast (0.5) and gamma (sRGB). WinUI 3 renders greyscale text with DirectWrite's contrast. That explains all three measured differences:
  - colour fringes: native 0, FerroUI 97 at 100 % and 64 at 200 % (mean colour of the ink pixels): FerroUI rendered ClearType where WinUI renders greyscale, also at 200 %, where subpixel antialiasing gains nothing;
  - lighter text: 11 to 12 % less ink than native on light themes, and fewer solid stem pixels (34 % against 48 % at 200 %): Skia's contrast of 0.5 with sRGB gamma thins dark-on-light text; light-on-dark text is not affected (section 3.3), which the measurements confirm (ink ratio 1.0 to 1.1 on dark themes);
  - the shapes, hinting and advances are the same: Skia's Windows port renders the glyphs with DirectWrite itself, in the same rendering mode WinUI uses (section 3.4).
- **WinUI's text is better than FerroUI's was**: sharper (no fringes) and darker. With the new defaults (section 5) FerroUI renders greyscale text with the contrast Chromium uses on Windows, which on the measured pictures brings the ink to 0.98 to 1.06 of native and the solid share to native's or slightly above (section 4.2).
- **The text of FerroUI sits 1 px higher than WinUI's at 200 % (0.5 dip), not at 100 %.** It is the same on every row at 200 % and absent at 100 %, also for a 28 px title, which rules out a fixed offset in dips. FerroUI places the baseline at the top of the line plus the ascent and Skia rounds it to the nearest device pixel; WinUI's baseline ends up between 0.14 and 0.39 dip lower before rounding (section 6). It is not changed here: section 6 explains why and the experiment that settles it.

## 2. What the native platforms do

### 2.1 Windows: DirectWrite, WinUI 3 and Win32 applications

- **WinUI 3 (and UWP XAML)** renders text through Direct2D and DirectWrite into composition surfaces, always with greyscale antialiasing (`D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE`), whatever the ClearType setting of the system: the surfaces are composed by the system compositor, may be transparent, transformed or animated, and subpixel colour cannot be composed correctly. The glyphs are rendered in DirectWrite's natural symmetric mode (`DWRITE_RENDERING_MODE_NATURAL_SYMMETRIC`, chosen from the font's `gasp` table): antialiased in both directions, grid fitted vertically (sharp horizontal stems and baseline), with natural (fractional) advance widths. Direct2D then blends the coverage with the gamma and the enhanced contrast of the rendering parameters of the monitor (`IDWriteRenderingParams`: `GetGamma`, `GetEnhancedContrast`, and `IDWriteRenderingParams1::GetGrayscaleEnhancedContrast` for greyscale), which makes dark-on-light text heavier than linear blending would.
- **Win32 applications** (GDI, WPF, Win32 DirectWrite applications, Chromium and Edge on an opaque window) render ClearType when the system has it on: `SPI_GETFONTSMOOTHING` and `SPI_GETFONTSMOOTHINGTYPE` (`FE_FONTSMOOTHINGCLEARTYPE`), with the subpixel order of `SPI_GETFONTSMOOTHINGORIENTATION` (RGB or BGR).
- **Chromium on Windows** (`ui/gfx/font_render_params_win.cc`, `ui/gfx/font_util_win.cc`, `skia/BUILD.gn`): subpixel text when ClearType is on, for the orientation of the system (RGB when it cannot be read), only on opaque layers, greyscale on transparent ones. Skia is built with `SK_GAMMA_SRGB` and `SK_GAMMA_CONTRAST=1.0` on Windows, chosen to match the weight of DirectWrite text; when the user tuned ClearType with the ClearType Text Tuner (the key `HKCU\SOFTWARE\Microsoft\Avalon.Graphics\<primary display>` exists) Chromium takes DirectWrite's `GetEnhancedContrast` and `GetGamma` instead, clamped to Skia's ranges (contrast 0 to 1, gamma 0 to below 4). Firefox does the same (contrast 1.0 unless tuned).

### 2.2 macOS and iOS: CoreText

Since macOS 10.14 the system renders greyscale text (subpixel antialiasing is off by default; `AppleFontSmoothing` only controls a slight dilation); iOS has never rendered subpixel text. Skia's CoreText port follows the system: a request for LCD text gives greyscale masks unless the system smooths with subpixels, and it sets the contrast of the mask gamma to 0 itself because CoreGraphics dilates the outlines instead (`SkTypeface_mac_ct.cpp`, `onFilterRec`). Chromium builds Skia for macOS with sRGB gamma and contrast 0.

### 2.3 Linux and Android: FreeType

Desktops write the font settings of the user to fontconfig and to the X resource `Xft.rgba` (`rgb`, `bgr`, `vrgb`, `vbgr` or `none`); GNOME defaults to greyscale, KDE to RGB subpixel. Android has never rendered subpixel text. Skia's FreeType port is built without `SK_GAMMA_APPLY_TO_A8` in skia-bindings (and in Chromium except on Android), so contrast and gamma do not apply to greyscale masks at all, only to LCD masks; Chromium uses gamma 1.2 and contrast 0.2 on Linux for those.

### 2.4 Browsers

A page cannot read the ClearType state, the subpixel order or the DirectWrite parameters of the system, its canvas is composited by the browser (and may be transparent), and `devicePixelRatio` is usually 2 or more. Browsers draw canvas text in greyscale. FerroUI's browser backend draws with its own Skia (emscripten) and fonts from FerroUI's font manager (FreeType), so section 2.3 applies to it.

## 3. The Skia text stack

### 3.1 The settings and what Skia does with them

| Setting | Where FerroUI sets it | What Skia does |
|---|---|---|
| Edging (`Font::set_edging`) | `glyph_run_impl.rs`, from `TextRenderingMode` | `Alias`: 1-bit masks. `AntiAlias`: 8-bit coverage (A8). `SubpixelAntiAlias`: LCD16 masks, three coverages per pixel. LCD falls back to A8 when the surface's pixel geometry is unknown or the text is larger than 48 device pixels (`SK_MAX_SIZE_FOR_LCDTEXT`). |
| Pixel geometry (`SurfaceProps`) | the surface of a render target | The order of the subpixels of LCD masks; `Unknown` disables LCD text (A8 from the LCD rendering). |
| Text contrast and gamma (`SurfaceProps::new_with_text_properties`) | the surface of a render target | The mask pre-blend (`SkMaskGamma`): the coverage `a` becomes `a + c·a·(1 − a)` (contrast `c`, scaled by the luminance of the guessed background, so that it fades to 0 for white text), then it is corrected so that a linear blend looks like a blend in the given gamma (0 means sRGB). Skia's defaults are contrast 0.5 and sRGB. Applied to LCD masks by every port, to A8 masks by the DirectWrite port (not by FreeType without `SK_GAMMA_APPLY_TO_A8`, and CoreText sets its contrast to 0). |
| Hinting (`Font::set_hinting`) | from `TextHintingMode` (`Strong`: `Full`, `Light`: `Slight`, `None`) | DirectWrite: `None` disables grid fitting, every other value is `Normal`; FreeType: `Full` is `Normal` for A8 and LCD targets for LCD, `Slight` is the light auto-hinter; CoreText: `None` avoids the dilation of smoothing, anything else allows it. |
| Subpixel positioning (`Font::set_subpixel`) | on unless aliased | Glyphs are positioned at quarter-pixel offsets horizontally: natural advances, as DirectWrite's natural modes. |
| Baseline snap (`Font::set_baseline_snap`) | on unless `BaselinePixelAlignment::Unaligned` | The baseline of a run is rounded to the nearest device pixel (with subpixel positioning only in x, y is snapped). |

### 3.2 Layers

Skia renders LCD text into a layer only when asked to keep it (`kPreserveLCDText`); FerroUI's own layers and visual caches follow the reference: a layer of a drawing context renders subpixel text only when its context does (`disable_text_lcd_rendering`), and offscreen render targets only when asked (`enable_text_antialiasing`).

### 3.3 Contrast and the weight of text

For black text on white (the common case of a light theme) the pre-blend gives these coverages for a half covered pixel: linear 0.50; Skia's default (contrast 0.5, sRGB) 0.35; contrast 1.0 with sRGB 0.46; contrast 1.0 with gamma 1.8 0.54. The default makes dark text thinner than DirectWrite's, which is what was measured. The contrast is scaled by the luminance of the background Skia guesses (the inverse of the text colour), so white text on black gets almost none and does not change: the dark themes measured as heavy as native before and after.

### 3.4 The DirectWrite port and the hinting decision

Skia's Windows port renders every glyph with DirectWrite (`SkScalerContext_win_dw.cpp`): the rendering mode comes from the `gasp` table of the font, as DirectWrite's own recommendation does. Segoe UI Variable has `gasp` version 1 with symmetric smoothing from 8 to 50 ppem (flags 15: grid fit, greyscale, symmetric grid fit, symmetric smoothing) and Segoe UI from 19 ppem, so both render in `DWRITE_RENDERING_MODE_NATURAL_SYMMETRIC` with grid fitting at the sizes of the gallery, which is WinUI's mode. With greyscale edging the port asks DirectWrite for greyscale antialiasing (`DWRITE_TEXT_ANTIALIAS_MODE_GRAYSCALE`, DirectWrite 2), which is WinUI's coverage; only the blend differs (section 3.3).

Because the port maps every hinting other than `None` to `Normal`, `Full`, `Normal` and `Slight` are the same on Windows and all match DirectWrite's natural symmetric mode; `None` disables grid fitting, which blurs horizontal stems and the baseline. The hinting is therefore kept (`TextHintingMode::Strong` and unspecified give `Full`): it is what matches WinUI on Windows, and on FreeType `Full` is collapsed to `Normal` for greyscale masks. One small difference remains: Skia keeps grid fitting on above 50 ppem where the `gasp` table of Segoe UI Variable turns it off; it affects only display sizes.

## 4. Why FerroUI looked as it did, measured

### 4.1 Before the change

`scripts/text-quality.py` (section 8) on the gallery comparisons that were measured for this work, native first, FerroUI second:

| Comparison | Page | Fringe | Ink ratio | Solid share | Text rows, dy |
|---|---|---|---|---|---|
| Windows CI runner, x64, 100 % | text, light | 0.0 / 89.5 | 0.901 | 32 % / 24 % | 0.00 px |
| | lists, light | 0.0 / 81.9 | 0.857 | 34 % / 27 % | 0.00 px |
| | text, dark | 0.0 / 74.1 | 1.098 | 42 % / 43 % | 0.00 px |
| Windows 11 ARM64 VM, 200 % | text, light | 0.0 / 52.4 | 0.877 | 52 % / 45 % | -0.98 px |
| | buttons, light | 0.0 / 38.1 | 0.923 | 34 % / 30 % | -0.99 px |
| | text, dark | 0.0 / 46.6 | 1.009 | 58 % / 55 % | -0.96 px |

### 4.2 What the new values give

The FerroUI coverage of the light text pages was mapped back through Skia's default pre-blend and forward through other values (an estimate: it starts from LCD coverage, not from DirectWrite's greyscale coverage). Contrast 1.0 with sRGB, Chromium's values, comes closest to native at both scales:

| Contrast, gamma | Ink ratio 100 % | Solid 100 % (native 32 %) | Ink ratio 200 % | Solid 200 % (native 52 %) |
|---|---|---|---|---|
| 0.5, sRGB (Skia's default, before) | 0.909 | 24 % | 0.886 | 45 % |
| 0.75, sRGB | 0.974 | 29 % | 0.926 | 49 % |
| **1.0, sRGB (Chromium, now)** | **1.058** | **38 %** | **0.980** | **57 %** |
| 0.5, 1.8 | 0.982 | 29 % | 0.929 | 49 % |
| 1.0, 1.8 | 1.121 | 40 % | 1.014 | 62 % |
| 1.0, 1.4 | 1.205 | 53 % | 1.061 | 68 % |

## 5. The defaults FerroUI chooses

### 5.1 The rule

Text whose `TextOptions` and `RenderOptions` leave the mode unspecified is rendered with **greyscale antialiasing**, except where all of these hold, where it is rendered with **subpixel antialiasing**:

1. the platform renders subpixel text on the display of the surface (ClearType on, `Xft.rgba` set to an order);
2. the subpixel order of the display is known;
3. the surface is not composed by a system compositor (Windows.UI.Composition, DirectComposition, the swap chain panel of a WinUI host);
4. the scene is opaque (the transparency level of the top-level is `None`);
5. the render scaling is below 1.5 (`GREYSCALE_TEXT_SCALING`).

Explicit settings are unchanged: `TextRenderingMode::SubpixelAntialias` (in `TextOptions` or the older `RenderOptions.TextRenderingMode`) renders subpixel text wherever the surface allows it, for the subpixel order of the display (or RGB, the reference's assumption, when the platform does not say); `Antialias` and `Alias` are kept; aliased edges (`EdgeMode::Aliased`) still give aliased text; a layer or a context that disables subpixel text still maps subpixel to greyscale. The contrast and gamma apply to every mode.

### 5.2 Per platform

| Platform, surface | Inputs | Default text | Contrast, gamma |
|---|---|---|---|
| Windows, window composed through Windows.UI.Composition or DirectComposition (the defaults of `Win32PlatformOptions.composition_mode`) | ClearType and its order (`SystemParametersInfo`); composited | greyscale (as WinUI 3) | DirectWrite's when tuned, else 1.0 and sRGB |
| Windows, window with a redirection surface, the low latency swap chain, WGL, Vulkan or software rendering | same, not composited | ClearType below 150 % on an opaque window when ClearType is on; greyscale otherwise | same |
| Windows, top-level in the swap chain panel of a WinUI host (`ferroui-winui`) | same, composited | greyscale (as WinUI 3) | same |
| macOS, iOS | none (CoreText follows the system) | greyscale | Skia's defaults (CoreText sets its own contrast to 0) |
| Linux X11 | `Xft.rgba` of the resource database | subpixel for the order of `Xft.rgba` below 150 % on an opaque window; greyscale when `none` or not set | Skia's defaults (no effect on greyscale FreeType masks) |
| Linux Wayland, Linux framebuffer (DRM, fbdev), Android | none (no access to the desktop's font settings; Android never renders subpixel text) | greyscale | Skia's defaults (no effect on greyscale FreeType masks) |
| Browser (emscripten Skia, FreeType) | none: a page cannot read the system's settings; the canvas is composited and may be transparent; `devicePixelRatio` is usually 2 or more | greyscale | Skia's defaults, contrast 0.5 and sRGB: fixed, and without effect on greyscale FreeType masks, so the text is FreeType's coverage blended linearly, as the canvas text of browsers on Linux and Android |
| Render target bitmaps, offscreen render targets, pictures | none: they show on no display | greyscale | Skia's defaults; RGB order for explicit subpixel text |

The Windows contrast of 1.0 is Chromium's, which section 4.2 confirms. When the user tuned ClearType for the primary display, DirectWrite's tuned enhanced contrast and gamma are used instead, as Chromium does; WinUI follows the same tuning through the rendering parameters of the monitor.

## 6. The baseline

FerroUI places the baseline of a line at its top plus the ascent of the font (`text_line_impl.rs`: `baseline = -ascent + half_line_gap`), and Skia rounds the baseline of a run to the nearest device pixel (`baseline_snap`, `BaselinePixelAlignment::Aligned`). For Segoe UI Variable at 14 px the ascent is 15.107 dip: 15 px at 100 % (15.107 rounds to 15) and 30 px at 200 % (30.215 rounds to 30). Native WinUI draws the same rows 1 px lower at 200 % and at the same place at 100 %; the 28 px title is 1 px lower at both scales. All four cases fit a native baseline that is between 0.14 and 0.39 dip lower than FerroUI's before rounding (for example the 0.38 dip that the line box of 18.62 dip gains when it is rounded up to 19 dip, put above the baseline). Layout rounding cannot explain it: the controls, their borders and the line pitch are at the same pixels.

The rounding itself matches (to the nearest pixel), so this is not FerroUI's rounding, and changing where text lines put their baseline changes every backend and the layout of every text, so it needs more than two font sizes of evidence. The experiment that settles it: a gallery page with single-line TextBlocks of Segoe UI Variable at 12, 13, 14, 15, 16, 18, 20, 24, 28 and 32 px, captured natively and with FerroUI at 100 %, 125 %, 150 % and 200 %; `scripts/text-quality.py` gives the offset of every row, and the native baseline is then known as a function of the size and the scaling (the candidates are: centred in the line box rounded up to whole device pixels, at the bottom of that box, or the ascent rounded up in dips).

## 7. How the platform values reach the backend

- `ferroui_base::platform::surfaces::PlatformTextRenderingParameters` (subpixel antialiasing on, `SubpixelGeometry`, composited, contrast, gamma) and `IPlatformRenderSurface::text_rendering_parameters`, which returns `None` by default: a surface reports what the platform knows of the display it is on.
- The Skia backend (`text_rendering_settings.rs`): `TextRenderingSource` keeps the surfaces of a top-level that report parameters and asks them every frame; `TextRenderingSettings::for_scene` applies the rule of section 5.1 to the parameters, the render scaling and the transparency level of the scene (`RenderTargetSceneInfo`) and gives the surface properties (pixel geometry, contrast, gamma) and the default mode. The GPU render targets (OpenGL, Vulkan, Metal) create the Skia surface of each frame with those properties (`ISkiaGpuRenderTarget::begin_rendering_session` takes them), the framebuffer render target recreates its surface when they change, the drawing context resolves unspecified text with the default mode (`effective_text_options`) and hands the settings to the layers it creates (`SurfaceRenderTargetCreateInfo::text_rendering`, `ISkiaGpu::try_create_surface`).
- Windows (`win32_text_rendering.rs`): the platform handle of a window, the first surface of the window, reports the system's parameters, composited when the window uses a composition mode; the swap chain top-level of a WinUI host adds a surface that reports them as composited. The values are read once (`SystemParametersInfoW`, `EnumDisplayDevicesW` and `RegOpenKeyExW` for the tuning key, `DWriteCreateFactory` and `IDWriteFactory::CreateRenderingParams` for the tuned values) and read again after `WM_SETTINGCHANGE`; a change shows from the next frame that is rendered.
- X11 (`x_resources.rs`): the surface handle of a window reports `Xft.rgba`, which the resource database updates when the desktop changes it.
- Nothing else is a global of the backend; a platform that reports nothing gets greyscale text with Skia's contrast and gamma.

## 8. Measuring: `scripts/text-quality.py`

```sh
python3 scripts/text-quality.py NATIVE.png FERROUI.png [--scale 2] [--crop X,Y,W,H]
python3 scripts/text-quality.py <run>/compare [--pages text,buttons] [--settings light,dark]
```

The second form reads a compare directory of the WinUI theme gallery (`<page>-<setting>-winui.png` and `<page>-<setting>-ferroui.png`) and prints a row per page and a mean. The scale defaults to the width divided by 720 (the width of the gallery's captures at 100 %). Python 3 and Pillow only; a 200 % page takes about 12 seconds.

| Column | Meaning | Native WinUI | Goal for FerroUI |
|---|---|---|---|
| fringe n/f | mean colour (max − min of R, G, B) of the ink pixels on a neutral background with neutral text | 0 | about 0 (greyscale text) |
| ink ratio | FerroUI's ink mass over native's (coverage against the local background and native's text colour) | 1 | 0.95 to 1.05 |
| solid n/f | share of ink pixels more than 85 % covered: how solid the stems are | measured | within a few points of native |
| dy px | median vertical offset of FerroUI's text rows from native's, in device pixels (negative: FerroUI higher) | 0 | 0 at 100 %; the open item of section 6 at 200 % |

Coverage is measured against the local background (the lightest pixel around, the darkest in a dark theme) and pixels in straight runs longer than 24 device pixels at 100 % (borders, rules) are left out, so the numbers are about text; they compare two renderings of the same page and are not absolute.

## 9. Verification

Verified on macOS:

- the Skia crate's tests in the default configuration (Graphite on Metal) and with `RUSTFLAGS="--cfg ferro_skia_apple_ganesh"` (Ganesh on OpenGL and Metal, which compiles the OpenGL render target and the framebuffer object surface that Windows (ANGLE) and the browser (WebGL) use): the rule, the clamping, the parameters read every frame, the surface properties of frames and of layers, the resolution of the mode, greyscale text on a render target bitmap by default;
- the render tests (370 pass, unchanged: CoreText renders LCD requests as greyscale on current macOS);
- `cargo check --workspace --all-targets`;
- the Windows backend: its tests of the parameters on the Mac host and `cargo check -p ferroui-win32 --no-default-features --all-targets` for x64 and ARM64 Windows;
- the X11 tests of `Xft.rgba`.

Not verified here:

- the Vulkan render target of the Skia backend (compiled on Linux and with the feature `vulkan` only; the change passes the surface properties as the OpenGL target does) and the browser build (no code of its own changed; its text becomes greyscale by default, as on every surface without parameters);
- on Windows: the values read from the system (`SystemParametersInfoW`, the tuning key, DirectWrite's parameters), the windows of the composition modes rendering greyscale text, ClearType on a window with a redirection surface, and the gallery comparison with the script, where the light pages should show a fringe of about 0, an ink ratio of about 1 and a solid share close to native's, and the dark pages should not change.

One Windows behaviour is not followed: when font smoothing is off altogether (`SPI_GETFONTSMOOTHING` false) GDI renders aliased text; FerroUI keeps greyscale antialiasing there, as WinUI 3 does.

### 9.1 Measured on Windows (2026-10-16)

The WinUI theme gallery compared with native WinUI 3 in the Windows 11 ARM64 virtual machine at a
render scaling of 2 (the composited swap chain panel of the comparison host, ClearType not tuned:
contrast 1, sRGB gamma), `scripts/text-quality.py --pages text,buttons,lists,menus`, before
(`e72b8415`) and after (`aae609da`) the change, native / FerroUI:

| Pair | Fringe before | Fringe after | Ink ratio before | Ink ratio after | Solid before | Solid after |
|---|---|---|---|---|---|---|
| buttons-light | 0.0 / 38.1 | 0.0 / 0.0 | 0.923 | 0.950 | 34 % / 30 % | 34 % / 32 % |
| buttons-dark | 0.0 / 36.3 | 0.0 / 0.0 | 1.020 | 1.027 | 39 % / 38 % | 39 % / 39 % |
| lists-light | 0.0 / 44.6 | 0.0 / 0.0 | 0.900 | 0.933 | 54 % / 48 % | 54 % / 51 % |
| lists-dark | 0.0 / 38.2 | 0.0 / 0.0 | 1.028 | 1.023 | 59 % / 58 % | 61 % / 61 % |
| menus-light | 0.0 / 32.2 | 0.0 / 0.0 | 0.923 | 0.962 | 49 % / 45 % | 49 % / 47 % |
| menus-dark | 0.0 / 28.2 | 0.0 / 0.0 | 1.028 | 1.034 | 54 % / 51 % | 54 % / 54 % |
| text-light | 0.0 / 52.4 | 0.0 / 0.0 | 0.877 | 0.909 | 52 % / 45 % | 52 % / 47 % |
| text-dark | 0.0 / 46.6 | 0.0 / 0.0 | 1.009 | 1.004 | 58 % / 55 % | 58 % / 58 % |
| mean | 0.0 / 39.6 | 0.0 / 0.0 | 0.964 | 0.980 | 50 % / 46 % | 50 % / 49 % |

At a render scaling of 1, on the Windows runner of CI (x64, the same comparison, run 38938301589
before and 38961004687 after), the mean of the same pages, native / FerroUI: colour fringe 0.0 /
71.6 before and 0.0 / 0.0 after, ink ratio 0.966 before and 1.000 after, solid share 35 % / 32 %
before and 35 % / 35 % after: at that scaling the text has the weight and solidity of DirectWrite's.

The colour fringes are gone on every page, light text on dark backgrounds has the weight and
solidity of native text, and dark text on light backgrounds is darker than before but still 4 to
9 % lighter than DirectWrite's (Skia's contrast is at its maximum of 1). What is left of that gap
is the gamma Skia corrects coverage with (sRGB here) against the gamma DirectWrite applies in its
greyscale path; matching it needs the measurement of section 6's capture with the gamma as a
variable, and is open, as the baseline offset of section 6 (still about 1 pixel at a scaling of
2, `dy` of the script).
