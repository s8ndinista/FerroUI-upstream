# NOTICE

This crate is a port of the SingleProjectSandbox sample of the upstream project
(MIT; see the `NOTICE.md` at the root of the repository).

- The markup documents `App.xaml`, `MainView.xaml` and `MainWindow.xaml` are converted from the
  upstream documents by `scripts/convert_catalog_xaml.py`; the conversion changes names only.
- The Rust sources are ports of the upstream C# sources, file by file
  (`Platforms/<platform>/` as upstream).
