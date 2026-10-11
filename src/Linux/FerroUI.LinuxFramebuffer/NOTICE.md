# NOTICE

This crate is ported from the `Avalonia.LinuxFramebuffer` project of Avalonia
(MIT; see the `NOTICE.md` at the root of the repository), and
`raw_event_grouping.rs` from the shared source file
`src/Shared/RawEventGrouping.cs` of the same project.

The structures, request numbers and codes in `native_unsafe_methods.rs` and
the key codes in `input/ev_dev/ev_dev_key_table.rs` are those of the Linux
kernel's user-space interface (`linux/fb.h`, `linux/input.h`,
`linux/input-event-codes.h`), which programs may use under the kernel's
system call exception (`Linux-syscall-note`).
