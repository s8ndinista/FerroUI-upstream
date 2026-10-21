# Native Gallery rendering checkpoint

These are actual frames from `winui-gallery-capture`, using the existing FerroUI
headless window platform, **Skia drawing, HarfBuzz text shaping and the WinUI theme**.
The capture host rendered at **1280 × 900**, scale 1, on Linux x86_64 without a
display server. The theme selector in each frame identifies the requested theme.
The source Gallery is pinned to `14061eeac21c38ae99f302edf3d9136d4f652c4c`.

Five representative frames were visually reviewed and retained here:

| Frame | Content exercised | Page state |
| --- | --- | --- |
| [NumberBox, light](NumberBox-light.png) | Native WinUI NumberBox inputs, inline spin buttons, expression input and value output. | Partial |
| [AutoSuggestBox, light](AutoSuggestBox-light.png) | Both native source examples; the second submitted `Button` and decoded the original embedded control PNG through the native asset loader. | Partial |
| [RadioButton, dark](RadioButton-dark.png) | Native option group, color groups and the original selected background/border composition. | Partial |
| [InfoBar, dark](InfoBar-dark.png) | All three native source compositions, severity/message/action controls and close/icon/open options. | Partial |
| [WebView2, light](WebView2-pending-light.png) | Navigation and source inspection for a page whose native platform control remains pending. No WebView was instantiated. | Pending |

The Linux host does not provide Segoe Fluent Icons / Segoe MDL2 Assets. The
AutoSuggestBox Find symbol and InfoBar status/close glyphs consequently show missing
font boxes in these frames. The symbol/glyph API values are preserved; cross-platform
glyph presentation remains an explicit limitation. The font is not redistributed.
See the existing [WinUI theme notice](../../../../src/FerroUI.Themes.WinUI/NOTICE.md).

## Reproduce

From the repository root, the representative light frames were produced with:

```sh
cargo run -p winui-gallery --features capture --bin winui-gallery-capture --offline -- \
  --pages=NumberBox,AutoSuggestBox,RadioButton,WebView2 --query=Button \
  --output-dir=/tmp/ferroui-gallery-captures
```

The successfully built executable was then run directly for the complete light
sweep and representative dark sweep:

```sh
/tmp/ferroui-gallery-target/debug/winui-gallery-capture \
  --all --query=Button --output-dir=/tmp/ferroui-gallery-captures-all
/tmp/ferroui-gallery-target/debug/winui-gallery-capture \
  --pages=NumberBox,AutoSuggestBox,RadioButton,InfoBar,WebView2 --query=Button --dark \
  --output-dir=/tmp/ferroui-gallery-captures-dark
```

For these commands, `CARGO_TARGET_DIR=/tmp/ferroui-gallery-target` selected the build
directory. The source binary has no dependency on that location; the same arguments
can follow `cargo run ... --` in another environment. `--offline` requires cached
dependencies and may be omitted when initially fetching them. The constrained build
used one Cargo job, disabled incremental compilation/development debug information,
and the public Skia binary archive documented in
[the sample validation ledger](../../../../samples/WinUIGallery/VALIDATION.md).

**All 126 catalog pages completed the light rendering sweep.** Every output decoded
as a nonblank 1280 × 900 PNG; all 126 frame hashes were distinct. All five selected
dark pages completed as well. Before rendering a page, the host logs its stable ID
and rejects any native adaptation construction error. `--query=Button` also fails
if the selected source bitmap cannot be decoded by the real renderer.

The remaining sweep images are temporary smoke-test output and are not checked in.
These frames establish native construction/layout/rendering at this checkpoint;
they are **not native Windows pixel-parity baselines**, complete page acceptance,
screen-reader verification or a promotion of any pending/partial page.

## Capture hashes

| File | SHA-256 |
| --- | --- |
| `NumberBox-light.png` | `c83f75bf391c8615f1a01ca273d0974f29b81b32d665ed7b29968ae81545b0c0` |
| `AutoSuggestBox-light.png` | `6d9a74c9dffabd54e5558672c84d5914f6a38021ae721608ceb1e0da4ed38056` |
| `RadioButton-dark.png` | `80ce8e13ab197536311262ac2c71727bdc15e19976177c7fdf8b41aea9e8e639` |
| `InfoBar-dark.png` | `5c3685c0852008681233967db1e9c2cc0ec8403637877dd8a45f374046232da0` |
| `WebView2-pending-light.png` | `3ed745bad107abe566bb6879c076cb6491df49a2e65bca4a158dd07586df4ec9` |
