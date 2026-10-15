# NOTICE

The files of this crate at its root (`ferro_vnc_logger.rs`, `headless_vnc_framebuffer_source.rs`,
`headless_vnc_platform_extensions.rs`) are ported from the `Avalonia.Headless.Vnc` project of Avalonia
(`src/Headless/Avalonia.Headless.Vnc`, MIT; see the `NOTICE.md` at the root of the repository).

## RemoteViewing

The files under `remote_viewing/` (but `logging.rs`, `utility/reentrant_lock.rs`, `vnc/version.rs`,
`vnc/vnc_transport.rs` and `vnc/server/vnc_event.rs`, which are written for the port) are ported from
RemoteViewing, the .NET VNC library the upstream project is built on:

- Project: <https://github.com/quamotion/remoteviewing> (the package `Quamotion.RemoteViewing`, version 1.1.211)
- License: BSD-2-Clause
- Ported: the server session and what it uses (`Vnc/Server/VncServerSession.cs`, `VncServerSessionOptions.cs`,
  `IVncServerSession.cs`, `PasswordProvidedEventArgs.cs`, `CreatingDesktopEventArgs.cs`,
  `FramebufferUpdatingEventArgs.cs`, `FramebufferUpdateRequest.cs`, `EncoderStatistics.cs`, `IVncFramebufferCache.cs`,
  `VncFramebufferCache.cs`, `VncEncoder.cs`, `RawEncoder.cs`, `TightEncoder.cs`, `TightCompression.cs`,
  `TightCompressionControl.cs`, `VncMessageType.cs`; `Vnc/AuthenticationMethod.cs`, `ExtendedDesktopSizeReason.cs`,
  `ExtendedDesktopSizeStatus.cs`, `IVncFramebufferSource.cs`, `IVncPasswordChallenge.cs`, `KeyChangedEventArgs.cs`,
  `KeySym.cs`, `PointerChangedEventArgs.cs`, `RemoteClipboardChangedEventArgs.cs`, `VncEncoding.cs`, `VncException.cs`,
  `VncFailureReason.cs`, `VncFramebuffer.cs`, `VncPasswordChallenge.cs`, `VncPixelFormat.cs`, `VncRectangle.cs`,
  `VncStream.cs`, `VncUtility.cs`; `Utility/AutoClear.cs`, `PeriodicThread.cs`). Each Rust file names its source.

```
RemoteViewing VNC Client/Server Library for .NET
Copyright (c) 2013 James F. Bellinger <http://www.zer7.com/software/remoteviewing>
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.
2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR
ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

## Microsoft.Extensions.Logging

`remote_viewing/logging.rs` ports the part of `Microsoft.Extensions.Logging.Abstractions` that RemoteViewing logs
through (the log levels, the event id, the logger contract and four of its extension methods): the .NET runtime
(<https://github.com/dotnet/runtime>), MIT, Copyright (c) .NET Foundation and Contributors.

## Crates

The RFB server uses `flate2` (zlib, MIT OR Apache-2.0), `jpeg-encoder` (JPEG, MIT OR Apache-2.0), `des` (DES of the
VNC password challenge, MIT OR Apache-2.0) and `getrandom` (MIT OR Apache-2.0); they are not vendored.
