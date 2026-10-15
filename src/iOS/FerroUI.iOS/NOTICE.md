# NOTICE

This crate is ported from the `Avalonia.iOS` project of Avalonia (`src/iOS/Avalonia.iOS`, MIT; see the
`NOTICE.md` at the root of the repository). No file of the upstream project carries a licence header of
another project.

## Dependencies outside the workspace

The crate calls UIKit, Core Animation, Core Foundation, Metal and EAGL (OpenGL ES) through these crates, on iOS targets
only, each pinned to an exact version in the manifest (`docs/porting/ios-platform.md`, section 2):

| Crate | Version | Licence |
|---|---|---|
| `objc2` | 0.6.5 | MIT |
| `block2` | 0.6.2 | MIT |
| `objc2-foundation` | 0.3.2 | MIT |
| `objc2-core-foundation` | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| `objc2-quartz-core` | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| `objc2-metal` | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| `objc2-ui-kit` | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| `objc2-open-gl-es` | 0.3.2 | Zlib OR Apache-2.0 OR MIT |

All eight are parts of the objc2 project (https://github.com/madsmtm/objc2). On Apple targets the crate
also uses `libc` (MIT OR Apache-2.0) for functions of the C library (`pthread_main_np`, and `dlopen` and
`dlsym` for the entry points of OpenGL ES).

Nothing of these crates is copied into this one. The class `CAEAGLLayer`, which the bindings of Core Animation
do not declare, is found by name (`eagl/eagl_layer_surface.rs`), and the functions of OpenGL ES are loaded from
the library of the system by name. Two frameworks of the system are called without a bindings crate: one function of AudioToolbox (`AudioServicesPlaySystemSound`, declared in `ios_platform_feedback.rs` from
`AudioToolbox/AudioServices.h`) and the class `UTType` with five constants of UniformTypeIdentifiers (found by
name and declared in `storage/ios_storage_provider.rs` from `UniformTypeIdentifiers/UTCoreTypes.h`). The declarations of `interop.rs` are written from the
system headers of Core Foundation and libdispatch, as the declarations of the upstream file are.
