# NOTICE

This crate is a port of the WinUIEmbedSample sample of the upstream project
(MIT; see the `NOTICE.md` at the root of the repository).

- The markup documents `FerroApp.xaml` and `EmbeddedView.xaml` are converted from the upstream
  documents by `scripts/convert_catalog_xaml.py`; the conversion changes names only (the name of
  the upstream project in the texts of the view is the name of this project).
- The markup of the window of WinUI (`MARKUP` of `main_window.rs`) is the upstream
  `MainWindow.xaml` with the changes that loading it at run time asks for (`docs/porting/winui.md`,
  section 7.6).
- The Rust sources are ports of the upstream C# sources, file by file.
