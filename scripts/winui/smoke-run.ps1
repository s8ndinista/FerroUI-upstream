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
    ended (Stop-Process -Force, which is TerminateProcess) after its threads are listed (each with
    its state and the module and offset where it started) and their stacks are printed through the
    console debugger of the image (cdb, a non-invasive attach), and the script exits with 124.
    Otherwise it exits with the exit code of the process.

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

# Where an address of the process is: the module that holds it and the offset in it.
function Format-Address([System.Diagnostics.Process]$Target, [long]$Address) {
    try {
        foreach ($module in $Target.Modules) {
            $base = $module.BaseAddress.ToInt64()
            if ($Address -ge $base -and $Address -lt $base + $module.ModuleMemorySize) {
                return ("{0}+0x{1:x}" -f $module.ModuleName, ($Address - $base))
            }
        }
    } catch {
        # The list of modules of a process that is ending may not be readable.
    }
    return ("0x{0:x}" -f $Address)
}

# The stacks of every thread of the process, through the console debugger of the Debugging Tools
# for Windows when the image has it: a non-invasive attach (the threads are suspended and read,
# nothing is injected, which works in a process inside ExitProcess), with the symbols of the
# executable beside it and those of the system from Microsoft's symbol server.
function Show-Stacks([System.Diagnostics.Process]$Target) {
    $architecture = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" }
    $cdb = Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10\Debuggers\$architecture\cdb.exe"
    if (-not (Test-Path $cdb)) {
        Write-Output "---- no console debugger at $cdb`: the stacks are not printed"
        return
    }
    $symbols = Join-Path $directory "symbols"
    $symbolPath = "$([System.IO.Path]::GetDirectoryName($executablePath));srv*$symbols*https://msdl.microsoft.com/download/symbols"
    $stacks = Join-Path $directory "$name.stacks.log"
    Remove-Item $stacks -ErrorAction SilentlyContinue
    Write-Output "---- the stacks of the threads of $name (cdb -pv)"
    $debugger = Start-Process -FilePath $cdb -NoNewWindow -PassThru -RedirectStandardOutput $stacks `
        -ArgumentList @("-pv", "-p", $Target.Id, "-y", "`"$symbolPath`"", "-c", "`"~*kn 60; qd`"")
    if (-not $debugger.WaitForExit(240000)) {
        Stop-Process -Id $debugger.Id -Force -ErrorAction SilentlyContinue
        Write-Output "(the debugger did not finish within 240 s)"
    }
    if (Test-Path $stacks) { Get-Content $stacks }
}

if (-not $ended) {
    Show-Output
    $process.Refresh()
    $threads = @($process.Threads)
    Write-Output "---- $name did not end within $TimeoutSeconds s: $($threads.Count) thread(s)"
    foreach ($thread in $threads) {
        $start = "an unknown address"
        try { $start = Format-Address $process $thread.StartAddress.ToInt64() } catch { }
        $reason = if ("$($thread.ThreadState)" -eq "Wait") { " ($($thread.WaitReason))" } else { "" }
        Write-Output ("    thread {0}: {1}{2}, started in {3}" -f $thread.Id, $thread.ThreadState, $reason, $start)
    }
    Show-Stacks $process
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
