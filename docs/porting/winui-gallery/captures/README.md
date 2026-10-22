# Native Gallery rendering checkpoints

These are actual frames from `winui-gallery-capture`, using the existing FerroUI
headless window platform, **Skia drawing, HarfBuzz text shaping and the WinUI theme**.
The capture host rendered at **1280 × 900**, scale 1, on Linux x86_64 without a
display server. The theme selector in each frame identifies the requested theme.
The upstream Gallery is pinned to `14061eeac21c38ae99f302edf3d9136d4f652c4c`.

The checked-in frames below retain their original checkpoints. Subsequent CI on
the FerroUI 0.4.0 integration at `83623379` produced **126 light and eight dark
frames**, all decoded and verified as nonblank 1280 × 900 PNGs. Its artifact, source
revision and hashes are recorded separately in the
[integration validation record](../data/integration-validation.json). These earlier
PNGs and their manifest have not been relabeled or replaced by that CI run.

## Retained checkpoint: native feedback controls and CompactSizing

The Gallery sources for this checkpoint are published at
[`3c7ceb81`](https://github.com/wieslawsoltes/FerroUI/commit/3c7ceb81dcc4948f374c05c4757c6cb72ed5de5d).
Native controls are recorded at `9501f091`, opt-in XAML migration at `f50450d0`, and
the compiled WinUI theme at `a9ede2a5`. The
[capture manifest](capture-manifest.json) records the exact source hashes, binary and
log hashes, every frame ID/hash from the **126-page light sweep** and **nine-page dark
sweep**, and the eight reviewed PNGs retained below.

| Page | Retained light frame | Retained dark frame | Visible content and evidence boundary |
| --- | --- | --- | --- |
| InfoBadge | [Light](InfoBadge-light.png) | [Dark](InfoBadge-dark.png) | Icon/value/dot compositions, severity selector, Refresh composition and dynamic-value input. Some icon glyphs are unavailable on this host. |
| ProgressRing | [Light](ProgressRing-light.png) | [Dark](ProgressRing-dark.png) | Indeterminate and determinate native controls with activation, background and value options. A static frame does not verify motion. |
| RatingControl | [Light](RatingControl-light.png) | [Dark](RatingControl-dark.png) | Caption and placeholder examples with clear/read-only options. The rating stars appear as missing-glyph boxes. |
| CompactSizing | [Light](CompactSizing-light.png) | [Dark](CompactSizing-dark.png) | Standard/Compact selector and the initial **Standard** form. These frames do not exercise switching to compact sizing or retaining entered values. |

All eight frames were visually reviewed for construction, layout and rendering.
**All four pages remain partial.** The Linux host does not provide the required
Segoe Fluent Icons / Segoe MDL2 Assets glyphs: rating stars and some icons display
missing-font boxes. The API/glyph values are retained; complete cross-platform font
presentation remains pending. The font is not redistributed. See the existing
[WinUI theme notice](../../../../src/FerroUI.Themes.WinUI/NOTICE.md).

### Exact commands

The following environment and commands produced this checkpoint, from the repository
root. Paths identify the recorded host's toolchain/cache and output directories;
another environment can use its own paths. `--offline` requires cached dependencies.

```sh
export RUSTUP_HOME=/tmp/ferroui-rustup
export CARGO_HOME=/tmp/ferroui-cargo
export PATH="/tmp/ferroui-cargo/bin:$PATH"
export CARGO_BUILD_JOBS=1
export CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export SKIA_BINARIES_URL=file:///tmp/ferroui-skia.tar.gz

cargo run --locked --offline -p winui-gallery --features capture --bin winui-gallery-capture -- \
  --pages=InfoBadge,ProgressRing,RatingControl,CompactSizing \
  --output-dir=/tmp/ferroui-gallery-current-light

target/debug/winui-gallery-capture \
  --all --query=Button --output-dir=/tmp/ferroui-gallery-current-all

target/debug/winui-gallery-capture \
  --pages=NumberBox,RadioButton,AutoSuggestBox,InfoBar,InfoBadge,ProgressRing,RatingControl,CompactSizing,WebView2 \
  --dark --query=Button --output-dir=/tmp/ferroui-gallery-current-dark
```

The local Skia archive is the public dependency artifact documented in
[the sample validation ledger](../../../../samples/WinUIGallery/VALIDATION.md), with
SHA-256 `9ebe4c448cc9f789bad241cbd11739cdf759e27db5719c23a7635e096d5807b3`.
The captured executable's SHA-256 is
`f59fb8575e5342777aa0089f7b4402620c44cf6937b4bb52d834a274a758f4e6`.

| Run | Measured result |
| --- | --- |
| Selected light run used for retained images | Four decoded, nonblank 1280 × 900 PNGs; four distinct hashes. |
| Full light sweep | All 126 catalog pages completed; 126 decoded, nonblank 1280 × 900 PNGs; 126 distinct hashes. |
| Selected dark sweep | All nine requested pages completed; nine decoded, nonblank 1280 × 900 PNGs; nine distinct hashes. |

Validation decoded every output and checked that at least one RGB channel had pixel
variation. Logged IDs matched the output files. The capture host rejects native
adaptation construction errors; `--query=Button` also rejects failure to decode the
selected original AutoSuggestBox detail PNG through the real renderer.

The retained light images come from the separate four-page run, **not** the full light
sweep. Their digests are recorded separately: three differ from the same-page sweep
images. These captures are observations of those runs, not deterministic image hashes
that a rebuild must reproduce.

| Capture/build log | SHA-256 |
| --- | --- |
| `/tmp/ferroui-capture-build.log` | `a3bc806a0cf7f1a0f8e7943fe01b0b95251b16513feca60273b80793ad38c8c1` |
| `/tmp/ferroui-capture-all.log` | `acb9ec40532bdecd10b017f05660ffb15cc1660a2872e710964cd98f2628e760` |
| `/tmp/ferroui-capture-dark.log` | `b81d2a803c792c0377596c800df67f3f5772df864344877b47657d94602aea01` |

The other sweep images, executable and logs are temporary artifacts; their hashes and
frame IDs are preserved in the manifest. The checked-in theme reference pointer is
not a production input: the manifest separately hashes the actual generated theme
files referenced by the capture executable's dependency file.

### Current retained-image hashes

The PNGs were copied byte for byte from the original capture output. No resizing,
retouching or image conversion was applied.

| File | SHA-256 |
| --- | --- |
| `InfoBadge-light.png` | `9ae63ab11fe04eccf3e5ee281288319db7ae83f7d3b2f89f878e28d1fd29b441` |
| `InfoBadge-dark.png` | `d515f3d3f207d5d77fad94498a92f60c97f681f8e48f92f759db9b4060e77e90` |
| `ProgressRing-light.png` | `14445302cfba279365dd53ab286c857b102f017c4bdd46b93270260ddd2b8812` |
| `ProgressRing-dark.png` | `5f5d3ecd3c6ed67af9ec9c197cc77fd2f53186ab16216cfe031cd89b9ad71338` |
| `RatingControl-light.png` | `646758eb13723257e08beb2aba77e798094eb2759b295f42b7d28caee5047e3a` |
| `RatingControl-dark.png` | `ffd42ba03be96b9a72a404d57b8317c03d6ae6f1a3f0a6e5b02207d16ef1d934` |
| `CompactSizing-light.png` | `5e19ba3f771e2d77152414b30a79ea8af65893b8150ca3fbe21eb5bdd7ccc5ec` |
| `CompactSizing-dark.png` | `025405e151d17339c6f2e92b009d72a17082bb8c5617af2218e1bf92de9d52b0` |

## Historical checkpoint: `d008845`

The following five images are **unchanged historical captures from `d008845`**. They
are retained as evidence of that earlier checkpoint and do not show the current
feedback-control or CompactSizing implementation.

| Frame | Content exercised at the historical checkpoint | Page state |
| --- | --- | --- |
| [NumberBox, light](NumberBox-light.png) | Native WinUI NumberBox inputs, inline spin buttons, expression input and value output. | Partial |
| [AutoSuggestBox, light](AutoSuggestBox-light.png) | Both native source examples; the second submitted `Button` and decoded the original embedded control PNG through the native asset loader. | Partial |
| [RadioButton, dark](RadioButton-dark.png) | Native option group, color groups and the original selected background/border composition. | Partial |
| [InfoBar, dark](InfoBar-dark.png) | All three native source compositions, severity/message/action controls and close/icon/open options. | Partial |
| [WebView2, light](WebView2-pending-light.png) | Navigation and source inspection for a page whose native platform control remained pending. No WebView was instantiated. | Pending |

At that checkpoint all 126 light-theme pages and five selected dark-theme pages
completed rendering. Every light-sweep output decoded as a nonblank 1280 × 900 PNG,
with 126 distinct hashes. The AutoSuggestBox Find symbol and InfoBar status/close
glyphs showed missing font boxes on the same kind of Linux host.

### Historical commands

These commands belong to `d008845`; `CARGO_TARGET_DIR=/tmp/ferroui-gallery-target`
selected that checkpoint's build directory. The constrained build used one Cargo job,
disabled incremental compilation/development debug information, and the public Skia
archive listed above.

```sh
cargo run -p winui-gallery --features capture --bin winui-gallery-capture --offline -- \
  --pages=NumberBox,AutoSuggestBox,RadioButton,WebView2 --query=Button \
  --output-dir=/tmp/ferroui-gallery-captures

/tmp/ferroui-gallery-target/debug/winui-gallery-capture \
  --all --query=Button --output-dir=/tmp/ferroui-gallery-captures-all

/tmp/ferroui-gallery-target/debug/winui-gallery-capture \
  --pages=NumberBox,AutoSuggestBox,RadioButton,InfoBar,WebView2 --query=Button --dark \
  --output-dir=/tmp/ferroui-gallery-captures-dark
```

### Historical retained-image hashes

| File | SHA-256 |
| --- | --- |
| `NumberBox-light.png` | `c83f75bf391c8615f1a01ca273d0974f29b81b32d665ed7b29968ae81545b0c0` |
| `AutoSuggestBox-light.png` | `6d9a74c9dffabd54e5558672c84d5914f6a38021ae721608ceb1e0da4ed38056` |
| `RadioButton-dark.png` | `80ce8e13ab197536311262ac2c71727bdc15e19976177c7fdf8b41aea9e8e639` |
| `InfoBar-dark.png` | `5c3685c0852008681233967db1e9c2cc0ec8403637877dd8a45f374046232da0` |
| `WebView2-pending-light.png` | `3ed745bad107abe566bb6879c076cb6491df49a2e65bca4a158dd07586df4ec9` |

## Acceptance boundary

Both checkpoints provide **construction/rendering evidence only**. They do not
establish complete page acceptance, upstream Windows pixel parity, platform UIA or
screen-reader interoperability. Rendering a pending page's source browser does not
instantiate or implement its missing platform control. No page/API status is promoted
by these screenshots; source-example behavior tests and the explicit parity ledgers
remain separate evidence.
