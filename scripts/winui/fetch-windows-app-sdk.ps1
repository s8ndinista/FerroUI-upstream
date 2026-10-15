<#
.SYNOPSIS
    Puts the bootstrap library of the Windows App SDK beside an executable of the WinUI integration
    and, with -InstallRuntime, installs the Windows App Runtime of the same release
    (docs/porting/winui.md, section 7).

.DESCRIPTION
    An unpackaged application of the Windows App SDK needs two things on the machine:

    1. Microsoft.WindowsAppRuntime.Bootstrap.dll beside the executable. It is part of the package
       Microsoft.WindowsAppSDK on nuget.org (runtimes/win-<arch>/native). The script downloads the
       package of -Version (a .nupkg is a zip file) to a directory of its own under the temporary
       directory and copies the library of the architecture to -Destination.
    2. The Windows App Runtime of the release (its framework package and main package). With
       -InstallRuntime the script downloads the installer of the release from Microsoft
       (https://aka.ms/windowsappsdk/<major.minor>/<version>/windowsappruntimeinstall-<arch>.exe)
       and runs it quietly. That needs an administrator, or a machine where the user may install
       packages; the installer of a release that is installed already only says so.

    The release is the one the bindings of the integration are generated from, 1.6
    (Microsoft.WindowsAppSDK 1.6.241114003). The script prints what it did and stops at the first
    failure.

.EXAMPLE
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\winui\fetch-windows-app-sdk.ps1 -Destination C:\ferroui-target\debug -InstallRuntime
#>
param(
    [Parameter(Mandatory = $true)][string]$Destination,
    [string]$Version = "1.6.241114003",
    [string]$Architecture = "",
    [switch]$InstallRuntime
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

if ($Architecture -eq "") {
    switch ($env:PROCESSOR_ARCHITECTURE) {
        "ARM64" { $Architecture = "arm64" }
        "x86" { $Architecture = "x86" }
        default { $Architecture = "x64" }
    }
}
$majorMinor = ($Version.Split(".")[0..1]) -join "."
Write-Host "Windows App SDK $Version (release $majorMinor), architecture $Architecture"

# 1. The bootstrap library.
$work = Join-Path ([System.IO.Path]::GetTempPath()) "ferroui-windowsappsdk-$Version"
New-Item -ItemType Directory -Force -Path $work | Out-Null
$package = Join-Path $work "microsoft.windowsappsdk.$Version.zip"
if (-not (Test-Path $package)) {
    $url = "https://www.nuget.org/api/v2/package/Microsoft.WindowsAppSDK/$Version"
    Write-Host "Downloading $url"
    Invoke-WebRequest -Uri $url -OutFile $package
}
$extracted = Join-Path $work "package"
if (-not (Test-Path $extracted)) {
    Expand-Archive -Path $package -DestinationPath $extracted
}
$library = Join-Path $extracted "runtimes\win-$Architecture\native\Microsoft.WindowsAppRuntime.Bootstrap.dll"
if (-not (Test-Path $library)) {
    throw "The package has no $library"
}
New-Item -ItemType Directory -Force -Path $Destination | Out-Null
Copy-Item -Force -Path $library -Destination $Destination
Write-Host "Copied Microsoft.WindowsAppRuntime.Bootstrap.dll to $Destination"

# 2. The runtime.
if ($InstallRuntime) {
    $installer = Join-Path $work "windowsappruntimeinstall-$Architecture.exe"
    if (-not (Test-Path $installer)) {
        $url = "https://aka.ms/windowsappsdk/$majorMinor/$Version/windowsappruntimeinstall-$Architecture.exe"
        Write-Host "Downloading $url"
        try {
            Invoke-WebRequest -Uri $url -OutFile $installer
        } catch {
            # The link of the newest servicing release of the release, which has the same interfaces.
            $url = "https://aka.ms/windowsappsdk/$majorMinor/latest/windowsappruntimeinstall-$Architecture.exe"
            Write-Host "Not found; downloading $url"
            Invoke-WebRequest -Uri $url -OutFile $installer
        }
    }
    Write-Host "Running $installer --quiet"
    $process = Start-Process -FilePath $installer -ArgumentList "--quiet" -Wait -PassThru -NoNewWindow
    Write-Host "The installer ended with exit code $($process.ExitCode)"
    if ($process.ExitCode -ne 0) {
        throw "The installer of the Windows App Runtime failed with exit code $($process.ExitCode)"
    }
    Get-AppxPackage -Name "Microsoft.WindowsAppRuntime.$majorMinor*" | ForEach-Object {
        Write-Host "Installed: $($_.PackageFullName)"
    }
}
