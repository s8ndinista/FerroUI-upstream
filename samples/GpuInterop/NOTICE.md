# NOTICE

This sample is a port of the GpuInterop sample of the upstream project
(MIT; see the `NOTICE.md` at the root of the repository).

- The markup documents `App.xaml`, `MainWindow.xaml` and `GpuDemo.xaml` are converted from the
  upstream documents by `scripts/convert_catalog_xaml.py`; the conversion changes names only.
- The Rust sources are ports of the upstream C# sources of the sample, file by file.
- `VulkanDemo/Assets/Shaders/` (the GLSL sources, the `Makefile` and the compiled SPIR-V files) is
  copied from the upstream sample unchanged.
- `D3DDemo/MiniCube.fx` is copied from the upstream sample unchanged. It carries its own notice:
  Copyright (c) 2010-2013 SharpDX - Alexandre Mutel, under the MIT license whose text is at the
  top of the file.
- The model of the teapot is the one of the ControlCatalog sample
  (`samples/ControlCatalog/Pages/teapot.bin`), which the upstream project file links into this
  sample too.
- `D3DDemo/d3d11.idl` declares the COM interfaces of Direct3D 11, DXGI and the shader compiler
  from the method tables of the system headers; it is written for this port (upstream calls them
  through the bindings of Silk.NET).
- `smoke.rs`, `task_support.rs` and `tests/` are not ports: the upstream sample has no smoke run
  and no tests.
