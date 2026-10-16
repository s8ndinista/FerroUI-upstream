# GpuInterop: port notes and gaps

The sample draws with a graphics API of the platform into a drawing surface of the compositor
(`Compositor::create_drawing_surface`, `CompositionSurfaceVisual`), through the GPU interop of
the compositor (`Compositor::try_get_composition_gpu_interop`, `ICompositionGpuInterop`: the
import of external images and semaphores, and the updates of the surface with a keyed mutex,
with a pair of semaphores, with timeline semaphores or with automatic synchronization).

Status: 3 documents, all compiled by the build; `cargo test -p gpu-interop --lib` passes (17
tests). The smoke run (`--smoke`, `smoke.rs`) passes on macOS with the Vulkan demo against the
Metal compositor and against the OpenGL (CGL) compositor.

```sh
cargo test -p gpu-interop --lib
cargo run -p gpu-interop -- --smoke
```

## Files

| Upstream | Port | Notes |
|---|---|---|
| The three markup documents (the converter names them `.xaml`) | `App.xaml`, `MainWindow.xaml`, `GpuDemo.xaml` | Converted, names only. |
| `App.xaml.cs` | `app.rs` | `DemoType` is a property of the application, set by the factory of the application builder (`configure_with`). |
| `MainWindow.xaml.cs` | `main_window.rs` | Both constructors: `new` (Vulkan) and `with_demo_type`. |
| `GpuDemo.xaml.cs` | `gpu_demo.rs` | The six direct properties and `IGpuDemo`. `Demo` has the type of the interface (`Option<Rc<dyn IGpuDemo>>`); the interface has an `as_object` hook, registered with `ValueTypes::register_interface_object`, so that the content control bound to `Demo` shows the control behind the handle. |
| `DemoType.cs` | `demo_type.rs` | |
| `DrawingSurfaceDemoBase.cs` | `drawing_surface_demo_base.rs` | An abstract class with the virtual methods `initialize_graphics_resources`, `free_graphics_resources`, `render_frame`, `supports_disco`. It implements `IGpuDemo` through its handle of the interface (`DrawingSurfaceDemoBase::as_gpu_demo`, declared with `ferro_class_info!`). `Initialize` and `DoInitialize` are asynchronous, as upstream: a future started on the dispatcher that awaits `try_get_composition_gpu_interop`. |
| `NativeMethods.cs` | `native_methods.rs` | IOSurface and Core Foundation (macOS), `close` of the C library. |
| `Program.cs` | `program.rs` | `--d3d` (Windows), `--dmabuf`; the Windows and X11 options and `VulkanOptions` with the debug layer, as upstream. Plus `--smoke` (`smoke.rs`, not a port). |
| `VulkanDemo/ByteString.cs` | `VulkanDemo/byte_string.rs` | |
| `VulkanDemo/D3DMemoryHelper.cs` | `VulkanDemo/d3d_memory_helper.rs` | Windows. |
| `VulkanDemo/VulkanBufferHelper.cs` | `VulkanDemo/vulkan_buffer_helper.rs` | |
| `VulkanDemo/VulkanCommandBufferPool.cs` | `VulkanDemo/vulkan_command_buffer_pool.rs` | A submitted buffer is moved into the list of used buffers of the pool. |
| `VulkanDemo/VulkanContent.cs` | `VulkanDemo/vulkan_content.rs` | The model and the SPIR-V shaders are compiled into the program (embedded resources upstream). |
| `VulkanDemo/VulkanContext.cs` | `VulkanDemo/vulkan_context.rs` | The loader, the instance and the device are the three tables of `ash`; the extensions a device was created with stand for `TryGetDeviceExtension`. |
| `VulkanDemo/VulkanDemoControl.cs` | `VulkanDemo/vulkan_demo_control.rs` | |
| `VulkanDemo/VulkanDmaBufDemoControl.cs` | `VulkanDemo/vulkan_dma_buf_demo_control.rs` | Linux. |
| `VulkanDemo/VulkanDmaBufImage.cs` | `VulkanDemo/vulkan_dma_buf_image.rs` | The derived class holds its base image (`Deref`). |
| `VulkanDemo/VulkanDmaBufSwapchain.cs` | `VulkanDemo/vulkan_dma_buf_swapchain.rs` | |
| `VulkanDemo/VulkanExtensions.cs` | `VulkanDemo/vulkan_extensions.rs` | `ThrowOnError` gives an error value; see "Exceptions" below. |
| `VulkanDemo/VulkanImage.cs` | `VulkanDemo/vulkan_image.rs` | `SaveTexture` reads the IOSurface on macOS and goes through Skia elsewhere (G005). |
| `VulkanDemo/VulkanMemoryHelper.cs` | `VulkanDemo/vulkan_memory_helper.rs` | |
| `VulkanDemo/VulkanSemaphorePair.cs` | `VulkanDemo/vulkan_semaphore_pair.rs` | |
| `VulkanDemo/VulkanSwapchain.cs` | `VulkanDemo/vulkan_swapchain.rs` | Over `SwapchainBase` of the base crate. |
| `VulkanDemo/VulkanTimelineSemaphore.cs` | `VulkanDemo/vulkan_timeline_semaphore.rs` | macOS: a Metal shared event. |
| `VulkanDemo/Assets/Shaders/*` | the same | Copied. |
| `D3DDemo/D3D11DemoControl.cs` | `D3DDemo/d3d11_demo_control.rs` | Windows; the class exists everywhere, as upstream. |
| `D3DDemo/D3D11Swapchain.cs` | `D3DDemo/d3d11_swapchain.rs` | Windows. |
| `D3DDemo/D3DContent.cs` | `D3DDemo/d3d_content.rs` | Windows; see G004. |
| `D3DDemo/MiniCube.fx` | the same | Copied. |
| `GpuInterop.csproj`, `Properties/launchSettings.json` | `Cargo.toml`, `build.rs` | The project file and the launch profiles of .NET. The two profiles are the command lines of `program.rs`. |

## The graphics APIs

| Upstream (Silk.NET) | Port |
|---|---|
| Silk.NET.Vulkan and its EXT and KHR extensions | `ash` (the bindings the Vulkan crate of the framework uses), with the loader opened at run time |
| Silk.NET.Direct3D11, Silk.NET.DXGI, Silk.NET.Direct3D.Compilers | `D3DDemo/d3d11.idl`, generated by the MicroCom generator of the framework (as the Windows backend generates its own), with the structures in `D3DDemo/d3d11.rs` and `D3D11CreateDevice`, `CreateDXGIFactory1`, `D3DCompile` and `D3DGetInputSignatureBlob` linked by `raw-dylib` |
| SkiaSharp (`GRContext` over the Vulkan device, for `SaveTexture`) | `skia-safe`, where it has Vulkan (Linux; Windows with the feature `vulkan`) |
| `System.Numerics` | `ferroui_base::numerics`; the two left-handed matrices of .NET 9 the Direct3D demo uses are in `d3d11_demo_control.rs` |

Where each demo draws, as upstream: the Vulkan demo shares opaque file descriptors with
semaphores (Linux), Direct3D 11 textures or opaque NT handles (Windows) and IOSurfaces with a
timeline semaphore exported as a Metal shared event (macOS, MoltenVK); the Direct3D 11 demo shares
DXGI global shared handles with keyed mutexes (Windows, ANGLE); the dma-buf demo shares Linux
dma-bufs with the EGL compositor (Linux, X11 with EGL). Upstream has no page of its own for Metal
or OpenGL: on macOS the Vulkan demo is imported by the Metal compositor or by the OpenGL
compositor, whichever the platform renders with.

## Exceptions

Upstream throws where Vulkan or COM fail. The initialization of a demo catches what it throws
and shows its text; so does the port, with the error value of the call (`VulkanDemoError`, the
text of `ThrowHResult`). What a frame throws upstream ends the application; the port panics with
the same text.

## Gaps and differences

| Id | What the sample does | The port | Status |
|---|---|---|---|
| G001 | A debug build attaches the developer tools (`#if DEBUG .WithDeveloperTools()`). | The application builder has no developer tools extension. Left out of `program.rs`. | open: nothing to reproduce |
| G002 | The property `Demo` of `GpuDemo` has the type `IGpuDemo` and holds the demo control, which `<ContentControl Content="{Binding $parent[local:GpuDemo].Demo}"/>` shows. | As upstream: `Option<Rc<dyn IGpuDemo>>`, shown as the control (`tests/shell.rs`: `the_main_window_shows_the_vulkan_demo_in_the_demo_control`). The framework showed a control as its text unless the value was boxed as `Ref<Control>`, and the sample typed the property `Option<Ref<Control>>`; corrected in the framework (`Control::from_boxed`, `ValueTypes::register_interface_object`; DEVIATIONS.md, corrected divergences). | closed: framework fixed |
| G003 | `Initialize` awaits `TryGetCompositionGpuInterop`, whose answer comes from the render thread. | As upstream: `try_get_composition_gpu_interop` returns a future that a compositor confined to its render thread completes from a job of that thread, so the first request is answered. The framework answered `None` while the render thread had not reported its features yet, and the sample asked again after the next job of the render thread; corrected in the framework (DEVIATIONS.md, corrected divergences), the retry is gone. | closed: framework fixed |
| G004 | `D3DContent` compiles `D3DDemo\MiniCube.fx`, which the project copies next to the program, with `D3DCompileFromFile`. | The file is compiled into the program and compiled from memory with `D3DCompile`, under the same name. | difference of the build |
| G005 | `VulkanContext` creates a Skia context over the Vulkan device on Linux and Windows (`SaveTexture` reads images back with it), and fails when it cannot. | Skia has Vulkan on Linux, and on Windows with the feature `vulkan` (Skia compiled from source); without it there is no context and `save_texture` says so ("Need skia to dump textures, sorry"). The Vulkan demo needs the feature on Windows anyway: it is what gives the Windows backend its Vulkan rendering mode. | difference of the build |
| G006 | On macOS the loader of the Vulkan SDK in `/usr/local/lib` is used when it is there. | The loader of Homebrew on Apple silicon (`/opt/homebrew/lib`) is tried next, then the one the system finds. | addition |
| G007 | `DisposeAsync` of the swapchains and of the resources awaits the disposal of every image before the device is destroyed; `D3D11DemoControl.FreeGraphicsResources` blocks on it. | The disposals are futures on the dispatcher (`task_support.rs`); the swapchain collects those of its images and the resources destroy the device after them. The Direct3D demo does not block: its textures hold references of their own. | difference of the platform |
| G008 | The demo control is held by its demo (`ParentControl`), and holds the demo. | The demo holds its demo control weakly, so that the two do not keep each other alive. | difference of the platform |

## Verification

- On the Mac (Apple M3 Pro, macOS 26): the smoke run of the Vulkan demo on MoltenVK against the
  Metal compositor (`FERROUI_SMOKE_RENDERING=metal`) and against the OpenGL compositor
  (`FERROUI_SMOKE_RENDERING=opengl` in a build with `--cfg ferro_skia_apple_ganesh`) passes every
  check: the demo initializes on the device, the teapot is in the snapshot of the compositor
  (16 % of the demo left of the sliders is its colour, the rest the red clear colour of the demo at
  an alpha of 0.1 over the window), and the snapshots change with the yaw and with the disco mode.
  The Vulkan loader and MoltenVK were the ones the Android emulator of the SDK ships
  (`DYLD_LIBRARY_PATH` and `VK_DRIVER_FILES` pointed at `emulator/lib64/vulkan`).
- After the framework fixes of G002 and G003 (the property `Demo` of the interface type, the interop awaited without the retry), the smoke run passes again against both compositors with the same results.
- Type-checked for `x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu` against a stand-in of
  the desktop entry point (for Linux with the real Skia and its Vulkan back end).
- Not run: the Direct3D 11 demo and the Vulkan demo on Windows, the Vulkan and dma-buf demos on
  Linux, `save_texture`, the debug messenger with the validation layer.
