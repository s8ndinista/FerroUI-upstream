# The Linux framebuffer platform

The design of the platform that runs a FerroUI application on a Linux machine without a window system: one full-screen top-level drawn into a framebuffer device or onto a KMS output, with input read from the kernel's input devices. Upstream: `src/Linux/Avalonia.LinuxFramebuffer` at the tracked commit (`TRACKING.md`): 30 source files, 76 types, 529 members, and `src/Shared/RawEventGrouping.cs`, which the project compiles in.

Marks, as in `x11-platform.md`: **[V]** verified from sources (the upstream files, the kernel's headers as declared by the `libc` crate, the sources of a crate in the cargo registry), **[M]** measured here (a build or a test run on the development machine, a Mac), **[R]** recalled and not verified here, **[CI]** shown by the CI job, **[VM]** measured in the virtual machine (Ubuntu 24.04, ARM64). A real board with a panel and a touch screen is outside all of these and is not verified (section 10).

## 1. The crate

| Crate | Directory | Upstream | Enabled by |
|---|---|---|---|
| `ferroui-linuxframebuffer` | `src/Linux/FerroUI.LinuxFramebuffer` (replaces the placeholder of the first preview at the same path and name) | `Avalonia.LinuxFramebuffer` and `src/Shared/RawEventGrouping.cs` | `start_linux_fb_dev`, `start_linux_drm`, `start_linux_direct` of `LinuxFramebufferPlatformExtensions` (an extension trait of `AppBuilder`), which set the platform up, create the lifetime and run the loop, as upstream's `StartLinuxFbDev`, `StartLinuxDrm` and `StartLinuxDirect`; `use_linux_framebuffer` (an addition: upstream's internal `LinuxFramebufferPlatform.Initialize` made public, for a host that runs the lifetime itself) |

Dependencies: `ferroui-base`, `ferroui-controls`, `ferroui-opengl` (the EGL contracts of the DRM output, stage 2), `libc` (already a dependency of the workspace, `libc = "0.2"`, MIT or Apache-2.0, in `NOTICE.md` since the X11 crate) **[V]**. No new third-party crate.

Upstream's project references the Skia project and its initialisation calls `UseSkia()`. The port does not: the renderer is the caller's choice on the builder (`use_skia()`, or another renderer behind the same contracts) and the crate does not depend on a renderer; the example takes Skia, the text shaper and a theme behind the feature `example`, as the X11 crate does. This keeps `cargo check -p ferroui-linuxframebuffer --target x86_64-unknown-linux-gnu --all-targets` possible from a Mac without the native libraries of Skia (DEVIATIONS.md).

What compiles where. The modules that are functions of values (the kernel structures, the pixel formats, the key table, the state machines of the input devices, the parsing of the device descriptions, the timer arithmetic) compile on every Unix system, so that their tests run on the development machine. The modules that make system calls that only Linux has (`epoll`, `timerfd`, the framebuffer and input `ioctl`s, DRM) are `cfg(target_os = "linux")`. The crate is empty on Windows and on WebAssembly.

## 2. What upstream is, and what it is not

Read from the 30 files **[V]**:

- **One top-level.** `FramebufferToplevelImpl` is an `ITopLevelImpl` without a handle, a popup, a cursor or a frame; its client size is the pixel size of the output divided by the output's scaling; it can not be disposed (`Dispose` throws). `LinuxFramebufferLifetime` creates it on first use inside an `EmbeddableControlRoot` and is a controlled, single view and single top-level lifetime; `Shutdown` cancels the token the loop runs with.
- **Two outputs.** `FbdevOutput`: `open`, three `ioctl`s (`FBIOGET_VSCREENINFO`, `FBIOPUT_VSCREENINFO`, `FBIOGET_FSCREENINFO`), `mmap`, a software framebuffer surface that draws into a back buffer and copies it to the mapped memory after `FBIO_WAITFORVSYNC` (optionally on a thread of its own), or draws directly into the mapped memory. `DrmOutput`: a card of `/dev/dri`, its resources, a connected connector and a mode through `libdrm`, a GBM device and surface through `libgbm`, EGL on the GBM device, an OpenGL context shared with the renderer, and per frame `eglSwapBuffers`, `gbm_surface_lock_front_buffer`, a DRM framebuffer per buffer object, `drmModePageFlip` and a wait for the flip with `drmHandleEvent`.
- **Three input backends.** `LibInputBackend` (the default): a thread over `libinput` with the path backend, every `/dev/input/event*`; it maps touch events (down, up, motion, cancel by slot), absolute pointer motion, pointer buttons and the wheel. `EvDevBackend` (with `AVALONIA_USE_EVDEV=1`, devices named by environment variables): a thread over `epoll`, reading events through `libevdev`, with one device type, a single touch screen (`ABS_X`, `ABS_Y`, `BTN_TOUCH`) and a calibration matrix. `NullInputBackend`.
- **The dispatcher.** `EpollDispatcherImpl`: `epoll` over a wake-up pipe and a `timerfd`; timers, the signal, pending input from the manual raw event queue, and explicit background processing.

What upstream does not have, which matters for what a port can honestly claim **[V]**:

- **No keyboard.** Neither backend reads a key: `LibInputBackend` handles event types between `TOUCH_DOWN` and `TOUCH_CANCEL` and between `POINTER_MOTION` and `POINTER_AXIS`; the keyboard device that is registered never receives an event.
- **No relative pointer.** `LIBINPUT_EVENT_POINTER_MOTION` falls into the default branch ("is not mapped"): a mouse does not move the pointer; only absolute devices (tablets, the pointer of a virtual machine) do.
- **Unreachable and wrong branches in the pointer mapping.** The high-resolution wheel (`POINTER_SCROLL_WHEEL`, the value after `POINTER_AXIS`) is above the range the thread forwards, so only the legacy wheel event is handled; the two events describe the same step of a wheel, so forwarding both would scroll twice. The right button is mapped to "up" for state 1 and to "down" for state 2, and the middle button to "up" for state 2; `libinput` has two button states, 0 and 1, so the right button never goes down and the middle button never comes up.
- **No cursor image, no screens, no clipboard, no storage provider.**

The port ports what is there, and adds input that the task of this stage asks for and that a UI without a window system needs, as additions that are marked as such in the code and in DEVIATIONS.md (section 7): keys, relative pointers and multi-touch (protocol B) in the evdev backend; the button states corrected in the `libinput` backend.

## 3. Bindings

| Library | Upstream | The port | Why |
|---|---|---|---|
| The C library (`open`, `close`, `read`, `write`, `ioctl`, `mmap`, `munmap`, `poll`, `epoll_*`, `pipe2`, `timerfd_*`) | Platform invokes of `libc`, declared in `NativeUnsafeMethods.cs` and in `EpollDispatcherImpl.cs` | The `libc` crate: its declarations of the functions and of `epoll_event`, `itimerspec`, `pollfd`, `input_event`, `input_absinfo` **[V]** | The same C functions, with the structures laid out per architecture by the crate (upstream's `epoll_event` is the packed x86-64 one on every architecture; the kernel's is not packed on ARM64) |
| The framebuffer structures (`fb_var_screeninfo`, `fb_fix_screeninfo`, `fb_bitfield`, `fb_vblank`) and request numbers | Declared in `NativeUnsafeMethods.cs` | Declared in `native_unsafe_methods.rs` as `#[repr(C)]` structures, with tests of their sizes and field offsets against the kernel's (160, 80 on 64-bit and 68 on 32-bit, 12, 32 bytes) and of the request numbers against the `_IOR`/`_IOW` encoding | `libc` does not declare them; they are a stable kernel interface (`linux/fb.h`) |
| `libevdev.so.2` | Five functions: `libevdev_new_from_fd`, `_has_event_type`, `_next_event`, `_get_name`, `_get_abs_info` | Not used. The same five facts are read from the device itself: `EVIOCGNAME`, `EVIOCGBIT`, `EVIOCGABS` and `read` of `input_event` records (`input/ev_dev/ev_dev_device.rs`) | They are system calls on a file; a library for five of them is a run-time dependency a board image may not have. `libevdev`'s one behaviour beyond the calls is re-synchronising after `SYN_DROPPED`; here the events up to the next `SYN_REPORT` are passed over and the handler ends what was in progress (contacts end, held buttons are released). Recorded in DEVIATIONS.md |
| `libinput.so.10` | 23 functions (`LibInputNativeUnsafeMethods.cs`) | Opened at run time with `dlopen`, each function resolved with `dlsym` into a table (`input/lib_input/lib_input_native_unsafe_methods.rs`), as `x11_egl_helper.rs` does for `libEGL`. No link-time dependency | A machine without the library runs the fbdev output with the evdev backend; the check from the Mac and a minimal CI image need no development package. A missing library or symbol is an error that names it |
| `libdrm.so.2`, `libgbm.so.1` | 15 and 11 functions (`Output/Drm.cs`) | The same: `dlopen` and a table per library (`output/drm.rs`), structures declared `#[repr(C)]` with size tests | As above |
| `libEGL.so.1` | `eglGetProcAddress` by platform invoke, then the EGL interface of the OpenGL project | `EglInterface` of `ferroui-opengl` over a `dlopen`ed `libEGL`, as the X11 backend | The contract exists and is used the same way |

The `libc` crate is the one binding crate. Crates that wrap these libraries (`drm`, `gbm`, `input`, `evdev`) were not taken: each links its C library at build time or brings an object model of its own, and the code to port is upstream's sequence of C calls, which is short.

How the `unsafe` is contained: one module per library is the only place that calls it (`native_unsafe_methods.rs` for the C library and the framebuffer, `ev_dev_device.rs` for the input `ioctl`s, `drm.rs`, `lib_input_native_unsafe_methods.rs`), each call a safe function with its safety argument; the mapped memory of a framebuffer is owned by one type that unmaps it when dropped.

## 4. Threads and the event loop

| Thread | What runs on it |
|---|---|
| The UI thread (the thread that calls a start function) | `EpollDispatcherImpl::run_loop`; the top-level, the lifetime, the state machines of the input devices and the creation of raw input events |
| The render thread of the compositor (or the UI thread with `should_render_on_ui_thread`) | The render target of the output: the software framebuffer or the OpenGL session with the page flip |
| The render timer thread (`DefaultRenderTimer`) unless rendering on the UI thread (`UiThreadRenderTimer`) | Ticks at `fps` |
| The input thread of the backend | evdev: `epoll_wait` over the devices with a timeout of one second, `read`; `libinput`: `libinput_dispatch`, `poll` with a timeout of 10 ms |
| The blit thread, with `use_async_front_buffer_blit` | `FBIO_WAITFORVSYNC` and the copy of the back buffer |

**The dispatcher** is the port of `EpollDispatcherImpl.cs`, member by member; it is the loop of `X11PlatformThreading` without the connection, with one difference upstream has too: the wait has no timeout, a `timerfd` in the same `epoll` set ends it. One iteration: raise the timer when it is due; raise `signaled` when signalled; dispatch one input event of the manual raw event queue; raise `ready_for_background_processing` when asked; arm the timer (the seconds capped at 100, as upstream) or disarm it; `epoll_wait`; drain the pipe and the timer; clear the wake-up flag. The part other threads reach (the flags under a lock and the writing end of the pipe) is the `IDispatcherSignal` handle. On Unix systems other than Linux the type does not exist (unlike the X11 dispatcher, nothing can run there).

**Input crosses threads as values.** Upstream's input threads create `RawInputEventArgs` (which hold the input root and the device, objects of the UI thread) and the top-level posts each to the UI thread. In the port the input thread sends what it read (evdev: the `input_event` records of one read; `libinput`: a value per event with the fields the mapping needs, taken from the event before it is destroyed) and the handlers run on the UI thread, where they create the raw events and call the top-level's callback, which hands them to the raw event grouper. The order of events and the priority of the post (`Send`) are upstream's; the screen size is read on the UI thread instead of the input thread (DEVIATIONS.md).

**The output is shared with the thread that renders** (`IPlatformRenderSurface: Send + Sync`): `FbdevOutput` holds its device, the two information structures and the mapping behind a lock; the back buffer belongs to the render target, as the framebuffer of an X11 window does.

## 5. The fbdev output

`FbdevOutput` talks to its device through a trait, `IFbDevice` (get and put of the variable information, get of the fixed information, map, wait for the vertical sync), with two implementations: the kernel device (`open` of the file name of the options, of `FRAMEBUFFER`, or of `/dev/fb0`) and, for tests and for machines without a framebuffer device, a plain file (`FileFbDevice`: a file of `line_length * yres` bytes mapped the same way, with the information structures given by the caller). The trait is an addition; with the kernel device the sequence of calls is upstream's.

Pixel formats, as upstream **[V]**: 16 bits per pixel is `Rgb565`; 32 bits with blue at offset 16 is `Rgba8888`; everything else is treated as `Bgra8888`. A requested format (`FbDevOutputOptions::pixel_format`) is written with `FBIOPUT_VSCREENINFO` (again without the transparency field when the driver refuses it), and a device that then does not report 32 bits fails with upstream's message, also when 16 bits were asked for (upstream's check, kept). The renderer draws in the format of the lock; the conversion from the scene to that format is the renderer's (`LockedFramebuffer` of the Skia backend), not this crate's.

The vertical sync is `FBIO_WAITFORVSYNC`; its result is ignored as upstream ignores it (a driver without it returns an error and the frame is copied at once).

## 6. The DRM output (stage 2b, not built)

`DrmCard` opens the card (the given path, else the first `/dev/dri/card*` that has resources), `DrmResources` lists connectors, encoders and CRTCs, `DrmOutput` picks the connector (connected, optionally of the types of the options), the mode (the options' resolution, else the preferred mode), creates the GBM device and surface (`XRGB8888`, scan-out and rendering), the EGL display on it, the context of the options' versions, and sets the CRTC with the first buffer. A frame: `eglSwapBuffers`, lock the front buffer, a DRM framebuffer for the buffer object (kept in its user data and removed by its destroy callback), `drmModePageFlip` with an event, wait for the event with `poll` and `drmHandleEvent`, release the previous buffer. `PlatformGraphics` hands the renderer the one context (`uses_shared_context`), so Skia's Ganesh GL backend draws on it. The orientation of the options rotates the reported size and the session.

Mesa needs a driver for the card: on a virtual KMS device (`vkms`) that is `kms_swrast` with `llvmpipe` **[R]**, to be measured **[VM]**.

## 7. Input

`IInputBackend` (`initialize(info, on_input)`, `set_input_root`) and `IScreenInfoProvider` are upstream's.

**evdev** (`input/ev_dev/`). Upstream's types: `EvDevBackend` (the thread, `create_from_environment`), `EvDevDevice` (name, event types, the two absolute axes, `next_event`), `EvDevDeviceDescription` with `parse_from_env`, `EvDevTouchScreenDeviceDescription` with the calibration matrix, `EvDevDeviceHandler` and `EvDevSingleTouchScreen`. The environment variables are renamed: `FERROUI_USE_EVDEV`, `FERROUI_EVDEV_DEVICE_{name}`, with upstream's format `(path):type=(touchscreen):[calibration=m11,m12,m21,m22,m31,m32]`.

Additions (three device types, each a description and a handler, selected by `type=`):

| Type | Description | Handler | State machine |
|---|---|---|---|
| `keyboard` | `EvDevKeyboardDeviceDescription` | `EvDevKeyboard` | `EV_KEY` with value 1 (down), 2 (repeat, a further key down), 0 (up); the modifiers held; the key table (`ev_dev_key_table.rs`: the kernel's key code to `PhysicalKey`, then to `Key` and the key symbol of the US layout, with shift and caps lock); a text input event for a key down that has a symbol and no control, alt or meta modifier |
| `mouse` | `EvDevMouseDeviceDescription` | `EvDevMouse` | `REL_X`, `REL_Y` accumulate into a position clamped to the scaled size and are reported at `SYN_REPORT` as one move; `ABS_X`, `ABS_Y` without `BTN_TOUCH` (the pointer of a virtual machine) scale to the size; `BTN_LEFT`, `BTN_RIGHT`, `BTN_MIDDLE` are button events at the position; `REL_WHEEL`, `REL_HWHEEL` are wheel events |
| `multitouch` | `EvDevMultiTouchScreenDeviceDescription` (calibration as the single touch screen) | `EvDevMultiTouchScreen` | Protocol B: `ABS_MT_SLOT` selects a slot; `ABS_MT_TRACKING_ID` not negative begins a contact in it and -1 ends it; `ABS_MT_POSITION_X/Y` move it; at `SYN_REPORT` every changed slot reports begin, update or end with the slot as the touch identifier |

A keyboard layout is not read from the system (there is no `xkbcommon` here): the symbols are those of a US keyboard. That is a limit of the addition, stated in the README of the crate.

**libinput** (`input/lib_input/`, stage 2a, built). The port of the three partial files and the options. The thread owns the context and sends each event as a value (`LibInputEvent`: the type and the fields the mapping reads, taken before the event is destroyed); positions are asked of the library in a fixed range and scaled to the screen on the UI thread, where the mapping (`LibInputState`) keeps the pointer position and the contacts by slot. One correction, recorded in DEVIATIONS.md: the buttons use the states 0 and 1. The forwarded range is upstream's (the branch of the high-resolution wheel is ported and not reached). Relative motion and keys stay unmapped, as upstream; a machine that needs them uses the evdev backend.

`RawEventGroupingThreadingHelper` is ported; nothing upstream uses it.

## 8. The top-level, the lifetime and the entry points

`FramebufferToplevelImpl`, `LinuxFramebufferLifetime`, `LinuxFramebufferPlatform` (with the icon loader and the cursor factory that create objects without content), `LinuxFramebufferPlatformOptions` (`fps`, `should_render_on_ui_thread`) are ported member by member. The default input backend when none is given: evdev from the environment with `FERROUI_USE_EVDEV=1`, else `libinput`.

Entry points (`LinuxFramebufferPlatformExtensions`), one function per upstream overload, each returning the exit code:

| Upstream | Port |
|---|---|
| `StartLinuxFbDev(args, fbdev, scaling, inputBackend)` | `start_linux_fb_dev(args, fbdev, scaling, input_backend)` |
| `StartLinuxFbDev(args, fbdev, format, scaling, inputBackend)` | `start_linux_fb_dev_with_format(..)` |
| `StartLinuxFbDev(args, options, inputBackend)` | `start_linux_fb_dev_with_options(..)` |
| `StartLinuxDrm(args, card, scaling, inputBackend)` | `start_linux_drm(..)` |
| `StartLinuxDrm(args, card, connectorsForceProbe, options, inputBackend)` | `start_linux_drm_with_options(..)` |
| `StartLinuxDirect(args, outputBackend, inputBackend)` | `start_linux_direct(..)` |

## 9. File table

| Upstream file | Port (`src/Linux/FerroUI.LinuxFramebuffer/`) | Stage |
|---|---|---|
| `LinuxFramebufferPlatform.cs` | `linux_framebuffer_platform.rs` (the platform, the icon loader, the lifetime, the extensions) | 1 |
| `LinuxFramebufferPlatformOptions.cs` | `linux_framebuffer_platform_options.rs` | 1 |
| `Stubs.cs` | `stubs.rs` | 1 |
| `EpollDispatcherImpl.cs` | `epoll_dispatcher_impl.rs` | 1 |
| `FramebufferToplevelImpl.cs` | `framebuffer_toplevel_impl.rs` | 1 |
| `NativeUnsafeMethods.cs` | `native_unsafe_methods.rs` (the framebuffer structures and requests, the event types, axes and keys; the C functions are those of `libc`; the `libevdev` functions are not declared) | 1 |
| `Output/IOutputBackend.cs`, `Output/IGlOutputBackend.cs` | `output/i_output_backend.rs`, `output/i_gl_output_backend.rs` | 1, 2 |
| `Output/FbdevOutput.cs` | `output/fbdev_output.rs` (and `output/fb_device.rs`, the device trait, an addition) | 1 |
| `Output/FbDevBackBuffer.cs` | `output/fb_dev_back_buffer.rs` | 1 |
| `Output/FbDevOutputOptions.cs` | `output/fb_dev_output_options.rs` | 1 |
| `Output/Drm.cs` | `output/drm.rs` |2b |
| `Output/DrmBindings.cs` | `output/drm_bindings.rs` |2b |
| `Output/DrmOutput.cs` | `output/drm_output.rs` |2b |
| `Output/DrmOutputOptions.cs`, `DrmConnectorType.cs` | `output/drm_output_options.rs`, `drm_connector_type.rs` |2b |
| `Input/IInputBackend.cs`, `Input/IScreenInfoProvider.cs` | `input/i_input_backend.rs`, `input/i_screen_info_provider.rs` | 1 |
| `Input/NullInput/NullInputBackend.cs` | `input/null_input/null_input_backend.rs` | 1 |
| `Input/RawEventGroupingThreadingHelper.cs` | `input/raw_event_grouping_threading_helper.rs` | 1 |
| `Input/EvDev/EvDevBackend.cs` | `input/ev_dev/ev_dev_backend.rs` | 1 |
| `Input/EvDev/EvDevDevice.cs` | `input/ev_dev/ev_dev_device.rs` | 1 |
| `Input/EvDev/EvDevDeviceDescription.cs` | `input/ev_dev/ev_dev_device_description.rs` | 1 |
| `Input/EvDev/EvDevTouchScreen.cs` | `input/ev_dev/ev_dev_touch_screen.rs` | 1 |
| `Input/EvDev/EvDevTouchScreenDeviceDescription.cs` | `input/ev_dev/ev_dev_touch_screen_device_description.rs` | 1 |
| (additions) | `input/ev_dev/ev_dev_keyboard.rs`, `ev_dev_key_table.rs`, `ev_dev_mouse.rs`, `ev_dev_multi_touch_screen.rs` | 1 |
| `Input/LibInput/LibInputBackend.cs`, `.Pointer.cs`, `.Touch.cs` | `input/lib_input/lib_input_backend.rs`, `lib_input_backend_pointer.rs`, `lib_input_backend_touch.rs` |2a |
| `Input/LibInput/LibInputBackendOptions.cs` | `input/lib_input/lib_input_backend_options.rs` |2a |
| `Input/LibInput/LibInputNativeUnsafeMethods.cs` | `input/lib_input/lib_input_native_unsafe_methods.rs` |2a |
| `src/Shared/RawEventGrouping.cs` | `raw_event_grouping.rs` (the file of the X11 crate, as upstream compiles one file into both projects) | 1 |

A member of a later stage that is reachable in an earlier one fails with a message that names its stage (`start_linux_drm`) and counts as missing in the tracking.

## 10. Verification

| Level | Where | What it shows | What it cannot |
|---|---|---|---|
| 1. Cross check | The Mac: `cargo check -p ferroui-linuxframebuffer --target x86_64-unknown-linux-gnu --all-targets` (the example, which needs the native libraries of Skia, is compiled in the CI job and in the VM) | The Linux-only modules compile against the declarations of `libc` for Linux | That any call works |
| 2. Host tests | The Mac and the CI job: `cargo test -p ferroui-linuxframebuffer` | Everything that is a function of values: the layouts of the kernel structures against the kernel's sizes and offsets and the request numbers against their encoding; the pixel format a variable information maps to and the information `set_bpp` writes; the timer arithmetic of the dispatcher; the parsing of device descriptions; the state machines of the single touch screen, the keyboard, the mouse and the multi-touch screen against recorded event streams; the key table; the output against a plain file behind `IFbDevice` (the bytes of a frame arrive in the file in the format asked for) | The kernel's behaviour |
| 3. Linux tests | The CI job (Ubuntu, x86-64) and the VM: the same tests, plus those that need Linux: the dispatcher's loop (signal, timer, background processing, cancellation) on real `epoll` and `timerfd` | The loop on a Linux kernel | Devices |
| 4. The smoke example on virtual devices | `examples/linuxfb_smoke.rs`, lines `[ ok ]`/`[FAILED]`, then `SMOKE PASSED`. With `--file`: the file device (every machine). With `--fbdev <node>`: a kernel framebuffer device; with `--uinput`: virtual input devices the example creates through `/dev/uinput` (a touch screen, a keyboard, a mouse, a multi-touch screen) and writes events to, which must arrive as raw input at the top-level | The `ioctl`s, the mapping, the frame in the device's memory (read back from the device node), the input path from the kernel to raw events | A panel, a real driver's formats and timing, real touch hardware |
| 5. DRM on a virtual KMS device | The VM, where `vkms` loads: the smoke example with `--drm <card>` | Mode setting, GBM, EGL, page flips on Mesa's software path; the frame read back through the fbdev emulation or `glReadPixels` | A GPU driver, tearing, a real connector |
| 6. Pictures | The VM: the ControlCatalog on the framebuffer platform, the contents of the virtual framebuffer written to PNG files | That the catalog renders on this platform | |

Virtual devices, and where they exist:

- `vfb` (the kernel's virtual framebuffer, `CONFIG_FB_VIRTUAL`): not built for Ubuntu's kernels **[R]**; the VM script and the CI step try `modprobe vfb vfb_enable=1` and say what happened.
- `vkms` (`CONFIG_DRM_VKMS`): a module in Ubuntu's `linux-modules-extra` **[R]**. It registers a card, and through the DRM fbdev emulation a framebuffer device as well, which makes it the virtual device of both outputs when `vfb` is missing.
- `uinput`: built for Ubuntu's kernels **[R]**; needs root or access to `/dev/uinput`.

How a virtual framebuffer is told from the machine's console: the script records the nodes of `/sys/class/graphics/fb*` before loading a module and uses only a node that appeared afterwards and whose `name` is that of the virtual driver (`Virtual FB` for `vfb`, a name beginning with `vkms` for the emulation of `vkms`); the example refuses to write to a kernel device unless the name it reads with `FBIOGET_FSCREENINFO` matches the one given with `--expect-id`. An existing `/dev/fb0` of the machine is never opened for writing. The modules are unloaded when the script ends.

**Not verified by any of this: a real board.** No level runs on hardware with a panel and a touch screen: driver-specific pixel layouts (a 16-bit panel, a driver that refuses `FBIOPUT_VSCREENINFO`), `FBIO_WAITFORVSYNC` on a real controller, page flips at a panel's refresh rate, a touch controller's calibration. The measured results are added below this line as they are obtained.

### Measured on the development machine (2026-10-11) **[M]**

`cargo check -p ferroui-linuxframebuffer --target x86_64-unknown-linux-gnu --all-targets` is clean; `cargo test -p ferroui-linuxframebuffer` on the Mac: 45 tests (the layouts and request numbers, the timer arithmetic, the formats and `set_bpp`, the output against a file with the back buffer, direct rendering and the blit thread, the top-level, the descriptions, the four evdev state machines, the key table, the mapping of the `libinput` backend). The example needs the native libraries of Skia and is not compiled there.

### Measured by the CI job `linuxfb` (run 38108606784, 2026-10-11; Ubuntu, x86-64, kernel 6.17.0-1022-azure) **[CI]**

- The tests: 43 at that commit, with those that need Linux (the dispatcher's loop on `epoll` and `timerfd`: the signal from another thread, input, the timer after 30 ms, background processing, cancellation; the request numbers of the input `ioctl`s; the size of `input_absinfo` against the C library's).
- The kernel of the runner: no `vfb` ("CONFIG_FB_VIRTUAL is not set"); `vkms` loads and its framebuffer emulation appears as `/dev/fb1` "vkmsdrmfb", 1024x768, 32 bits (beside the machine's own `/dev/fb0` "hyperv_drmdrmfb", which is not opened); `/dev/uinput` is available. The module was unloaded at the end.
- The smoke runs, all passed: a file in BGRA, RGBA and RGB565 (the pixels read back from the file are the fill colour, in 16 bits within the precision of the format), direct rendering, the blit thread; a file with the four virtual input devices (touch screen: 3 raw events; keyboard: 10, with the texts "a" and "A"; mouse: 4; multi-touch screen: 5, each compared with the expected type, position and identifier); the same on `/dev/fb1` with the frame read back from the device node; the device refused when the expected identification does not match.

### Stage 2a and the catalog in the CI job (run 38110270775, 2026-10-11) **[CI]**

49 tests. The smoke run with `--libinput` passed against the library of the runner (`libinput10` 1.25): the virtual multi-touch screen (a direct device with a resolution) is taken by the path backend and its two contacts arrive as five raw events at the expected positions. The ControlCatalog (`control-catalog-desktop --fbdev --no-input`) ran on `/dev/fb1` of `vkms`, showed the seven pages of the picture run and ended by itself; the content of the device per page, read from its node, is uploaded as PNG files (1024x768; between 439 and 5130 distinct colours in a sample of each, so no blank frame). In that run the cycle through every page ran beside the picture run (both read `FERROUI_SMOKE_PAGES`), so a picture may show the neighbour of the page it is named after; the picture run has an interval variable of its own since. `vkms` was unloaded at the end. The run after that change (38110775919): the seven pages are selected one after the other and nothing else, seven pictures (429 to 5141 colours in the sample), every job of the pull request green.

### Measured in the virtual machine (2026-10-11; Ubuntu 24.04, ARM64, kernel 7.0.0-34-generic) **[VM]**

The same script: 43 tests; no `vfb`, `vkms` with `/dev/fb1` "vkmsdrmfb" beside the machine's `/dev/fb0` "virtio_gpudrmfb", `uinput`. The smoke runs passed but one: the RGB565 run panicked on the render timer thread ("a compositor that is confined to its render thread was rendered by another thread"), which was the default render timer starting a second thread and not the format (section 12; the platform uses the timer that keeps one thread since). After that run `vkms` could not be unloaded in the machine (one reference; on the CI runner it unloads): the script now tells the holder of the devices that they are going, names it, and takes the module up again in its next run.


## 11. Stages

| Stage | Content | What verifies it |
|---|---|---|
| 0 | This document; the project in scope in the tracking | |
| 1 (built) | The crate; the platform and its options; the dispatcher; the lifetime; the top-level; the fbdev output with the device trait; the evdev backend with upstream's single touch screen and the three added device types; the null backend; the example | Levels 1 to 4 |
| 2a (built) | The `libinput` backend | Levels 1 and 2 (the mapping of event values, the forwarded ranges, the calibration by orientation); level 4 with `--libinput`: a virtual multi-touch screen read through the library |
| 2b | The DRM output (`drm.rs`, `drm_bindings.rs`, `drm_output.rs`, the options) | Levels 1, 2 (structure sizes), 5 |
| 3 | CI steps; the CRITICAL-PATH row; the hand-over in CONTINUATION.md | The job |
| 4 | The ControlCatalog on the platform (`--fbdev`, `--drm`, as upstream's `ControlCatalog.Desktop`), pictures | Level 6 |

## 12. Deviations

Recorded in `DEVIATIONS.md`, section "Linux framebuffer platform": the renderer is the caller's choice; `libevdev` replaced by the device's own `ioctl`s; input crossing threads as values; the device trait of the fbdev output; the renamed environment variables; the added evdev device types; the corrected button states and wheel range of the `libinput` backend; what is not built yet and how each such place behaves.
