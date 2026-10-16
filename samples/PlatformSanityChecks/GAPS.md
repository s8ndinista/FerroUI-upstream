# PlatformSanityChecks: gaps

What the sample does that the port does differently. Status: 1 document (`App.xaml`), converted without an edit; `cargo test -p platform-sanity-checks` passes (3 tests). The program runs on Linux with an X server (the X11 platform); on another system it says so and exits.

| Gap | What the sample does | The upstream statement | The port | Status |
|---|---|---|---|---|
| PSC001 | The program initializes the runtime platform services through the initializer of a builder. | `Program.cs`: `AppBuilder.Configure<App>().RuntimePlatformServicesInitializer!()` | `Configure<App>()` leaves `RuntimePlatformServicesInitializer` null (only `UseStandardRuntimePlatformSubsystem`, which the platforms call, sets it), so the reference fails at this statement with a null reference before any check. The port calls `use_standard_runtime_platform_subsystem()` on the builder first, which sets the initializer the statement means, and the checks run. | a deviation: the reference is out of date there |
| PSC002 | `Die` kills the process. | `Process.GetCurrentProcess().Kill()` | `std::process::abort()`: the process ends at once, without unwinding, with the signal of an abort instead of `SIGKILL`. | not a gap of the framework |

Not gaps:

- `App.xaml` names no class, so it is not compiled as the document of `App`; `App.Initialize` (the load of the document by the XAML loader) fails in the reference and in the port alike (`tests/mod.rs`). The program never initializes the application.
- `ThreadPool.QueueUserWorkItem` is a thread of its own in the port.
