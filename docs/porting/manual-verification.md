# Verification that needs real hardware or a person

Every in-scope part of the port is built and was exercised by automated runs: continuous integration (macOS, Windows x64 and ARM64, Linux under Xvfb and headless sway, the Linux framebuffer on virtual devices, iOS and Android build checks, the browser), virtual machines (Windows 11 ARM64, Ubuntu 24.04 ARM64), the iOS simulator and the Android emulator. What those runs cannot show is listed here, per platform, as checks a person runs once on real hardware. Each item names how to start what is checked; the platform documents describe the automated runs.

The ControlCatalog is the application for most checks. On the desktop: `cargo run -p control-catalog-desktop --release` (options per platform below).

## macOS

- [ ] A screen reader: VoiceOver (Cmd+F5) reads the navigation, the Buttons page and a text box; the rotor lists headings and form controls; activating a button with VoiceOver raises its click.
- [ ] Input methods: Japanese or Chinese input in a TextBox of the catalog (composition shown at the caret, the candidate window placed at the caret, committed text inserted once).
- [ ] The storage pickers: open, save and folder pickers from the catalog's dialogs page.
- [ ] OpenGL: a build with `RUSTFLAGS="--cfg ferro_skia_apple_ganesh"` and `FERROUI_SMOKE_RENDERING=opengl`: live resize, a second window, popups (`apple-opengl.md`).

## Windows

- [ ] A screen reader: Narrator (Ctrl+Win+Enter) or NVDA reads the catalog's navigation and controls; the UI Automation tree in Accessibility Insights matches the controls (`win32-platform.md`, section 13).
- [ ] Input methods: the Japanese IME in a TextBox (composition, candidate window at the caret); the touch keyboard and the input pane on a touch device.
- [ ] The look of the transparency levels: acrylic and Mica on a real desktop (`FERROUI_SMOKE_TRANSPARENCY=acrylic` and `mica`), compared with a native WinUI window on the same machine.
- [ ] Vulkan on a real driver: a build with `--features ferroui-win32/vulkan` on a machine with a GPU driver; the example `win32_window --vulkan-probe` draws its frames, and the catalog with the Vulkan mode renders (`vulkan.md`, section 8).
- [ ] High DPI: a monitor at 150 % and moving a window between monitors of different scaling.

## WinUI 3 hosting

- [ ] `samples/WinUIEmbedSample` and `samples/ControlGallery.WinUI` on a Windows machine with the Windows App SDK 1.6 runtime: keyboard focus moving between the WinUI controls and the hosted content, typing and the IME in a hosted TextBox, drag and drop between the two sides, the clipboard, cursors and hover, pen input, and Narrator reading the hosted content (`winui.md`, section 9).

## Linux, X11

- [ ] A screen reader: Orca reads the catalog over AT-SPI (`atspi.md`).
- [ ] Input methods: IBus and Fcitx with a Chinese or Japanese engine in a TextBox.
- [ ] A desktop session with a GPU driver: GLX, EGL and Vulkan modes (`FERROUI_SMOKE_RENDERING=glx|egl`, the Vulkan mode by name) on hardware, not Mesa's software rasterisers.

## Linux, Wayland

- [ ] A desktop compositor with input: GNOME (Mutter) and KDE (KWin) sessions (the catalog uses Wayland when a compositor can be reached; `FERROUI_CATALOG_WAYLAND=0` keeps X11): pointer, keyboard, touch, the text input protocol with an input method, popups, drag and drop, fractional scaling.
- [ ] The dmabuf swap chain on a hardware GPU (`FERROUI_CATALOG_WAYLAND=dmabuf`): buffers with several planes and non-linear modifiers (`wayland-platform.md`, section 12.3).

## Linux framebuffer

- [ ] A board with a panel (for example a Raspberry Pi with a DSI or HDMI display): `--fbdev` and `--drm` (`--card`, `--orientation`), a touch screen through libinput with its calibration, vsync of the page flip (`linux-framebuffer-platform.md`).

## iOS

- [ ] A device build: a signed bundle (`scripts/ios/bundle.sh` with a development team) on an iPhone: touch, the software keyboard and dictation, the clipboard, the document pickers, rotation, the safe areas.
- [ ] VoiceOver reads the catalog and activates controls.
- [ ] OpenGL ES on a device (`FERROUI_IOS_RENDERING_MODE=OpenGl` with `--cfg ferro_skia_apple_ganesh`).

## Android

- [ ] A device: the catalog APK (`scripts/android/apk.sh catalog`) on a phone: touch, the software keyboard with an input method, the clipboard, storage pickers, rotation, back navigation, insets with a display cutout.
- [ ] TalkBack reads the catalog and activates controls.
- [ ] The Vulkan mode on a device GPU (`apk.sh smoke --features ferroui-android/vulkan`, `mode=vulkan`).

## Browser

- [ ] Firefox and Safari in addition to Chrome: the catalog site (`browser-platform.md`), text input with an input method, the clipboard, file pickers; the build with threads where the browser allows shared memory.

## Headless VNC

- [ ] Real viewers: TigerVNC, RealVNC and noVNC connect to the catalog started with `--vnc` (port 5901), with the password option, Tight encoding and resizing (`headless-vnc.md`). macOS Screen Sharing is refused by design of the ported library (it speaks protocol 3.889).
