# NOTICE

This crate is a port of the XEmbedSample sample of the upstream project
(MIT; see the `NOTICE.md` at the root of the repository).

- The Rust sources are ports of the upstream C# sources, file by file: `Program.cs` is
  `program.rs` and `SocketEx.cs` is `socket_ex.rs`. `gtk.rs`, the functions of GTK 3 the sample
  calls, stands for the GtkSharp package the upstream project references.
- `HarfbuzzWorkaround.cs` has no counterpart (see `GAPS.md`, XE001).
