<#
.SYNOPSIS
    Runs the smoke mode of a sample of the WinUI integration with a limit, and ends the process if
    it does not end by itself (docs/porting/winui.md, section 7.3).

.DESCRIPTION
    The step of CI that ran `timeout 300 control-gallery-winui.exe --smoke` from the shell of Git
    for Windows did not end when the process stayed after its checks had passed (runs 38741696373
    and 38769434562): the limit of `timeout` did not end the process, and the step ran until its own
    limit of 45 minutes. This script starts the executable with its output in two files, waits for
    it at most -TimeoutSeconds, and then prints the output. A process that has not ended by then is
    ended (Stop-Process -Force, which is TerminateProcess) after its threads are listed, and the
    script exits with 124. Otherwise it exits with the exit code of the process.

.EXAMPLE
    pwsh -NoProfile -File scripts/winui/smoke-run.ps1 -Executable target/debug/control-gallery-winui.exe
#>
param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [string]$Arguments = "--smoke",
    [int]$TimeoutSeconds = 300
)

$ErrorActionPreference = "Stop"

$executablePath = (Resolve-Path $Executable).Path
$name = [System.IO.Path]::GetFileNameWithoutExtension($executablePath)
$directory = Join-Path ([System.IO.Path]::GetTempPath()) "ferroui-smoke-run"
New-Item -ItemType Directory -Force -Path $directory | Out-Null
$out = Join-Path $directory "$name.out.log"
$err = Join-Path $directory "$name.err.log"
Remove-Item $out, $err -ErrorAction SilentlyContinue

$env:RUST_BACKTRACE = "1"
$process = Start-Process -FilePath $executablePath -ArgumentList $Arguments -NoNewWindow -PassThru `
    -RedirectStandardOutput $out -RedirectStandardError $err
# The exit code of a process started with redirected output is only kept when its handle was
# taken while it ran.
$null = $process.Handle
$ended = $process.WaitForExit($TimeoutSeconds * 1000)

function Show-Output {
    if (Test-Path $out) { Get-Content $out }
    if ((Test-Path $err) -and (Get-Item $err).Length -gt 0) {
        Write-Output "---- the error stream of $name"
        Get-Content $err
    }
}

if (-not $ended) {
    Show-Output
    $threads = @($process.Threads)
    Write-Output "---- $name did not end within $TimeoutSeconds s: $($threads.Count) thread(s)"
    foreach ($thread in $threads) {
        Write-Output ("    thread {0}: {1}, {2}" -f $thread.Id, $thread.ThreadState, $thread.WaitReason)
    }
    Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    $null = $process.WaitForExit(30000)
    Write-Output "::error::$name did not end within $TimeoutSeconds s and was ended"
    exit 124
}

$process.WaitForExit()
Show-Output
$code = $process.ExitCode
Write-Output "$name ended with the exit code $code"
exit $code
