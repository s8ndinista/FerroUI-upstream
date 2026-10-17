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
- **The text of FerroUI sits 1 px higher than WinUI's at 200 % (0.5 dip), not at 100 %; a 28 px title is 1 px higher at both.** The cause is in WinUI's TextBlock (section 6): with layout rounding the height of a text block is rounded up to whole device pixels, and WinUI puts the added height above the text (`m_layoutRoundingHeightAdjustment`), where FerroUI, as the reference, leaves it below. The baselines are then snapped to the nearest pixel on both sides. The fix is implemented behind a development override (`FERROUI_TEXT_BASELINE_ROUNDING=above`) so that the comparison measures both before it becomes the default; the default is unchanged.
- **Most of the gap in weight that was left at 200 % is the comparison, not the window.** The comparison host's `-ferroui.png` is a render target bitmap, which has no display and renders with Skia's default contrast (0.5); the window renders with the platform's (1.0). Measured on the window itself (`-ferroui-window.png`), dark text on light backgrounds has 0.97 to 1.01 of native's ink and native's solid share at 200 % (section 9.1).

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

### 6.1 What FerroUI does

FerroUI places the baseline of a line at its top plus the ascent of the font (`text_line_impl.rs`: `baseline = -ascent + half_line_gap`; Segoe UI Variable has no line gap), a text block draws its first line at the top of its content (`TextBlock::render_core`: the top padding), and Skia rounds the baseline of a run to the nearest device pixel (`baseline_snap`, `BaselinePixelAlignment::Aligned`: with subpixel positioning only in x the y of the glyph origin is `floor(y + 0.5)` in device space). With layout rounding (on by default) the desired height of the text block is its line height rounded up to whole device pixels (`Layoutable::measure`, `LayoutHelper::round_layout_value_up`), and that added height is below the text.

### 6.2 What WinUI does

From the sources of WinUI 3 (microsoft-ui-xaml, `dxaml/xcp`):

- A plain TextBlock (no character spacing, no `TextTrimming.Clip`, no inlines) lays out with an `IDWriteTextLayout` (`CTextBlock::ResolveTextMode`, `TextMode::DWriteLayout`). Its line metrics are DirectWrite's font metrics of the nominal face, unrounded: the baseline `FontSize * ascent / unitsPerEm` and the line advance `FontSize * (ascent + descent + lineGap) / unitsPerEm` (`CCompositeFontFamily::DetermineDefaultLineMetrics`, `CTextBlock::GetLineHeight`), set with `DWRITE_LINE_SPACING_METHOD_UNIFORM` (`ConfigureDWriteTextLayout`). For Segoe UI Variable at 14 px: baseline 15.107 dip, line 18.621 dip, the same as FerroUI's.
- **With layout rounding the measured height is rounded up and the difference is put above the text.** `CTextBlock::MeasureOverride`: `desiredSize.height = XcpCeiling(pageNodeSize.height * plateauScale) / plateauScale; m_layoutRoundingHeightAdjustment = desiredSize.height - pageNodeSize.height;` (the comment there: the line is not aware of layout rounding, so the text is moved by the difference), and `CTextBlock::GetContentRenderingOffset` adds `m_layoutRoundingHeightAdjustment` to the top padding. The slow path does the same (`PageNode::GetContentRenderingOffset`).
- The glyph run is drawn with DirectWrite's pixel snapping disabled (`IsPixelSnappingDisabled` returns true: XAML snaps itself) and its realization transform snapped vertically to the nearest device pixel at the full rasterization scale (`D2DTextDrawingContext::GetGlyphRunTransformAndBounds`: `SetDy(XcpRound(Dy))`, `XcpRound(x) = floor(x + 0.5)`), the same rounding as Skia's. Horizontally the subpixel position is kept, as in Skia.

So WinUI's baseline, from the top of a text block on a whole device pixel, is `round(s * (ascent + ceil(s * H) / s - H))` device pixels (`s` the scale, `H` the line height plus the vertical padding), and FerroUI's is `round(s * ascent)`:

| Size, scale | `s * H`, rounded up | added above (px) | WinUI's baseline | FerroUI's | Measured offset of FerroUI |
|---|---|---|---|---|---|
| 14 px, 1 | 18.62, 19 | 0.38 | round(15.49) = 15 | round(15.11) = 15 | 0 |
| 14 px, 2 | 37.24, 38 | 0.76 | round(30.97) = 31 | round(30.21) = 30 | -1 px |
| 28 px, 1 | 37.24, 38 | 0.76 | round(30.97) = 31 | round(30.21) = 30 | -1 px |
| 28 px, 2 | 74.48, 75 | 0.52 | round(60.95) = 61 | round(60.43) = 60 | -1 px |

All four measured cases (section 1, `dy` of the script) follow. The rounding of the glyph runs is the same on both sides; the cause is where the height that layout rounding adds goes, which is a decision of WinUI's TextBlock that the reference does not make.

### 6.3 The fix, behind a development override

`FERROUI_TEXT_BASELINE_ROUNDING=above` (or `winui`) makes `TextBlock::render_core` move its text down by what layout rounding adds to its content (`TextBaselineRounding::Above.offset`: the height of the text and the vertical padding rounded up to whole device pixels at the layout scale, minus that height), as WinUI does; `top` (the default, also when the variable is unset) keeps the reference's placement. It applies only with layout rounding, as in WinUI, and to every text block, so also to the text of buttons, labels and items (their content presenters create text blocks); the text boxes (`TextPresenter`) are not affected (WinUI renders them through RichEdit, which stacks its lines at whole pixels, section 1). The comparison host can switch it per text setting of a calibration run (section 10).

Before it becomes the default it should be measured: the calibration page (`textcal`, section 10) at 100 % and 200 % with the text settings `top` and `above`, where every row should then have a `dy` of 0. The formula above predicts these offsets of FerroUI's rows (device pixels) with `top`, for Segoe UI Variable (the semibold rows as the regular ones of their size):

| Scale | 12 | 13 | 14 | 15 | 16 | 18 | 20 | 24 | 28 | 32 |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 0 | -1 | 0 | 0 | -1 | 0 | 0 | 0 | -1 | 0 |
| 1.25 | 0 | 0 | -1 | 0 | 0 | 0 | -1 | 0 | 0 | -1 |
| 1.5 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 2 | 0 | 0 | -1 | 0 | 0 | 0 | -1 | 0 | -1 | -1 |

A row that measures otherwise points at a second cause. Making it the default changes the place of the text of every text block by up to one device pixel on every backend (a deviation from the reference for `docs/porting/DEVIATIONS.md` and new expected pictures of the render tests), and two places would follow it then: the underline of the access key of `AccessText` (drawn from the text layout without the offset) and the hit testing of `SelectableTextBlock` (from the top padding), which are both off by the offset (less than one device pixel) while the override is on.

## 7. How the platform values reach the backend

- `ferroui_base::platform::surfaces::PlatformTextRenderingParameters` (subpixel antialiasing on, `SubpixelGeometry`, composited, contrast, gamma) and `IPlatformRenderSurface::text_rendering_parameters`, which returns `None` by default: a surface reports what the platform knows of the display it is on.
- The Skia backend (`text_rendering_settings.rs`): `TextRenderingSource` keeps the surfaces of a top-level that report parameters and asks them every frame; `TextRenderingSettings::for_scene` applies the rule of section 5.1 to the parameters, the render scaling and the transparency level of the scene (`RenderTargetSceneInfo`) and gives the surface properties (pixel geometry, contrast, gamma) and the default mode. The GPU render targets (OpenGL, Vulkan, Metal) create the Skia surface of each frame with those properties (`ISkiaGpuRenderTarget::begin_rendering_session` takes them), the framebuffer render target recreates its surface when they change, the drawing context resolves unspecified text with the default mode (`effective_text_options`) and hands the settings to the layers it creates (`SurfaceRenderTargetCreateInfo::text_rendering`, `ISkiaGpu::try_create_surface`).
- Windows (`win32_text_rendering.rs`): the platform handle of a window, the first surface of the window, reports the system's parameters, composited when the window uses a composition mode; the swap chain top-level of a WinUI host adds a surface that reports them as composited. The values are read once (`SystemParametersInfoW`, `EnumDisplayDevicesW` and `RegOpenKeyExW` for the tuning key, `DWriteCreateFactory` and `IDWriteFactory::CreateRenderingParams` for the tuned values) and read again after `WM_SETTINGCHANGE`; a change shows from the next frame that is rendered.
- X11 (`x_resources.rs`): the surface handle of a window reports `Xft.rgba`, which the resource database updates when the desktop changes it.
- Nothing else is a global of the backend (apart from the development override of section 10, empty unless its variables are set); a platform that reports nothing gets greyscale text with Skia's contrast and gamma.

## 8. Measuring: `scripts/text-quality.py`

```sh
python3 scripts/text-quality.py NATIVE.png FERROUI.png [--scale 2] [--crop X,Y,W,H]
python3 scripts/text-quality.py <run>/compare [--pages text,buttons] [--settings light,dark] [--window] [--rows]
python3 scripts/text-quality.py <calibration run>/compare --variants [--rows]
```

`--window` measures the panel cut out of the picture of the window instead of the render target bitmap (section 9.1), `--rows` adds a line per row of text, and `--variants` reads a calibration run (section 10).

The second form reads a compare directory of the WinUI theme gallery (`<page>-<setting>-winui.png` and `<page>-<setting>-ferroui.png`) and prints a row per page and a mean. The scale defaults to the width divided by 720 (the width of the gallery's captures at 100 %). Python 3 and Pillow only; a 200 % page takes about 12 seconds.

| Column | Meaning | Native WinUI | Goal for FerroUI |
|---|---|---|---|
| fringe n/f | mean colour (max − min of R, G, B) of the ink pixels on a neutral background with neutral text | 0 | about 0 (greyscale text) |
| ink ratio | FerroUI's ink mass over native's (coverage against the local background and native's text colour) | 1 | 0.95 to 1.05 |
| solid n/f | share of ink pixels more than 85 % covered: how solid the stems are | measured | within a few points of native |
| dy px | median vertical offset of FerroUI's text rows from native's, in device pixels (negative: FerroUI higher) | 0 | 0 (section 6: about -1 px at 200 % and for large text until the baseline override is the default) |

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
solidity of native text, and dark text on light backgrounds measured darker than before but still
4 to 9 % lighter than DirectWrite's.

**That remaining gap is mostly the comparison.** The picture the script measures,
`<page>-<setting>-ferroui.png`, is drawn into a render target bitmap (`capture::render`), and a
render target bitmap knows no display: it renders greyscale text with Skia's default contrast,
0.5, and sRGB (section 5.2). The window, the swap chain panel of the comparison host, renders with
the parameters of the platform, contrast 1.0 and sRGB. Side by side the host also writes the panel
cut out of the picture of the window (`<page>-<setting>-ferroui-window.png`), which is what the
window shows. Measured on the same run (`aae609da`, 200 %), native / FerroUI, the bitmap against the
window (`scripts/text-quality.py --window`):

| Pair | Ink ratio, bitmap | Ink ratio, window | Solid, bitmap | Solid, window |
|---|---|---|---|---|
| buttons-light | 0.950 | 0.994 | 34 % / 32 % | 34 % / 34 % |
| lists-light | 0.933 | 0.994 | 54 % / 51 % | 54 % / 55 % |
| menus-light | 0.962 | 1.010 | 49 % / 47 % | 49 % / 50 % |
| text-light | 0.909 | 0.970 | 52 % / 47 % | 52 % / 52 % |
| buttons-dark | 1.027 | 1.025 | 39 % / 39 % | 39 % / 39 % |
| lists-dark | 1.023 | 1.024 | 61 % / 61 % | 61 % / 61 % |
| menus-dark | 1.034 | 1.035 | 54 % / 54 % | 54 % / 53 % |
| text-dark | 1.004 | 1.006 | 58 % / 58 % | 58 % / 58 % |

At 200 % the window's dark text on light backgrounds has 0.97 to 1.01 of native's ink and native's
solid share; the dark pages do not change (the contrast fades for light text). The 1.000 of the
runner at 100 % is a bitmap at contrast 0.5 as well; the windows there are one over the other, so
there is no picture of the window, and the window at contrast 1.0 is expected to be heavier than
native at 100 % (section 4.2 estimated 1.06). Both are measured with one build by a calibration run
(section 10) whose text settings set the contrast explicitly, which applies to the bitmap as well.
The baseline offset of section 6 is the same in the bitmap and in the window (`dy` -0.98 to -0.99).

## 10. Calibration aids (development only)

Each experiment on Windows costs a build and a run in the virtual machine or on CI. These aids let
one build be measured with several text settings. They are not a supported API: their names and
values may change or go, and nothing in FerroUI sets them. With none of them set nothing changes.

### 10.1 The development overrides

Read once from the environment when they are first needed, and replaceable at run time by a
calibration host (`ferroui_skia::set_text_rendering_override`,
`ferroui_base::media::text_formatting::set_text_baseline_rounding`):

| Variable | Values | What it replaces | Where |
|---|---|---|---|
| `FERROUI_TEXT_CONTRAST` | 0 to 1 | the text contrast of every Skia surface (the platform's, or Skia's 0.5) | `TextRenderingSource::settings` (the render targets of windows and of render target bitmaps) and the offscreen render targets of the backend (`text_rendering_override.rs`) |
| `FERROUI_TEXT_GAMMA` | 0 or `srgb` (sRGB), else up to 4 exclusive | the text gamma of every Skia surface | the same |
| `FERROUI_TEXT_EDGING` | `alias`, `antialias`, `subpixel` | the mode of text that asks for none (the default of the surface, section 5.1) | the same; the drawing context resolves unspecified text with it |
| `FERROUI_TEXT_HINTING` | `none`, `light`, `strong` | the hinting of text whose text options leave it unspecified (`strong` otherwise) | the same |
| `FERROUI_TEXT_BASELINE_SNAP` | `on`, `off` | the baseline pixel alignment of text that leaves it unspecified (`on` otherwise) | the same |
| `FERROUI_TEXT_BASELINE_ROUNDING` | `top`, `above` | where a text block puts the height layout rounding adds (section 6.3) | `TextBlock::render_core` (`text_baseline_rounding.rs`) |

A value that does not parse is reported on the error stream and left out. The first five apply to
the Skia backend only (the Vello backend draws its own glyphs); the baseline rounding is a matter
of the text block and applies on every backend. Explicit text options of an element
(`TextOptions`, `RenderOptions.TextRenderingMode`) still win over the edging, hinting and snap
overrides; the contrast and gamma apply to all text of a surface.

### 10.2 The text calibration page

`textcal` of the WinUI theme gallery (`Pages/TextCalibrationPage.xaml` and its native twin
`TextCalibrationPage.winui.xml`): plain TextBlocks of the default font, one line per size and
weight (12, 13, 14, 14 semibold, 15, 16, 18, 20, 20 semibold, 24, 28, 28 semibold, 32 px), each at
the top of a grid row of a whole number of device-independent pixels, so that every row is measured
on its own. It is one page of the gallery: the theme variants give light and dark.

### 10.3 A calibration run of the comparison host

`FERROUI_WINUI_COMPARE_TEXT_VARIANTS` lists text settings separated by `;`, each
`[name:]key=value,key=value` or `[name:]default`, with the keys `contrast`, `gamma`, `edging`,
`hinting`, `baseline_snap` (the Skia overrides) and `baseline` (the text block override). The host
compares every page and setting once per text setting in one process: before the pages of a text
setting it sets the overrides (those of the environment with the values of the setting), and it
writes the pictures and `compare.tsv` of each text setting into a directory of its name, with
`text-variants.tsv` listing them. Because the overrides apply to render target bitmaps too, the
`-ferroui.png` of a text setting that sets the contrast and the gamma has the text rendering the
window has with those values. Then

```sh
python3 scripts/text-quality.py <compare directory> --variants --rows [--pages textcal] [--settings light,dark]
```

prints the pairs of every text setting, one table of the means per text setting (ink ratio light
and dark, solid shares, `dy`), and with `--rows` a table per page and setting with a line per row of
text (on `textcal` named by size and weight) and a column per text setting (ink ratio and `dy`).
`--window` measures `-ferroui-window.png` instead (side by side only), `--rows` works on a plain
compare directory and on two pictures as well.

A list that answers both open questions, the weight at 100 % and 200 % and the baseline:

```
default;c050:contrast=0.5,gamma=0;c075:contrast=0.75,gamma=0;c100:contrast=1,gamma=0;c100g14:contrast=1,gamma=1.4;c100g18:contrast=1,gamma=1.8;c100g22:contrast=1,gamma=2.2;above:contrast=1,gamma=0,baseline=above
```

In the Windows virtual machine (`scripts/windows/vm-smoke.ps1`, which then runs the comparison only,
with `FERROUI_WINUI_COMPARE_DENSITIES=normal`, and writes the table into
`text-calibration.txt` where the machine has Python with Pillow):

```
powershell -NoProfile -ExecutionPolicy Bypass -File <S>:\ferroui-vm-windows\src-<commit>\scripts\windows\vm-smoke.ps1 -TargetDir C:\ferroui-target -Jobs 3 -InteractiveUser <user> -SkipTests -WinUIThemeGallery -WinUIThemeGalleryPages textcal,text,buttons -WinUIThemeGalleryTextVariants "<list>" -Report <S>:\ferroui-vm-windows\vm-smoke-report.txt
```

On CI (100 % on the Windows runner): start the workflow by hand with the inputs
`winui_compare_pages` (`textcal,text,buttons`) and `winui_text_variants` (the list), for example
`gh workflow run ci.yml --ref <branch> -f winui_compare_pages=textcal,text,buttons -f winui_text_variants='<list>'`;
the artifact `winui-theme-gallery-windows` then holds `compare/<setting>/` and
`compare/text-calibration.txt`.

### 10.4 Verification of the aids

Verified on macOS: the unit tests of the overrides (parsing, clamping, an empty override changes
nothing, the resolution of unspecified hinting and baseline alignment, the baseline formula against
the four measured cases), the tests of the Skia text rendering, of `TextBlock` and of the gallery
(the new page loads, has its native twin and passes the checks of the capture mode), and the
override end to end in the capture mode: with `FERROUI_TEXT_BASELINE_ROUNDING=above` a row whose
line height is not a whole number of device pixels moves down by one pixel, the others do not move
(the fallback font of the gallery on macOS has line heights of whole pixels at 14 px and the other
sizes of the page, so there the page itself does not move). `scripts/text-quality.py --variants`,
`--rows` and `--window` were run on the pictures of section 9.1.

Not verified here: the comparison host (`compare/run.rs`, Windows only, not compiled on macOS),
`vm-smoke.ps1 -WinUIThemeGalleryTextVariants` and the inputs of the CI workflow.
