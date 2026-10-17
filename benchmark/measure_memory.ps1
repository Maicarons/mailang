# Measure the peak working set of an allocation-heavy MaìLang program.
#
# Regression gate for D4 (ROADMAP Phase N N4): the census used to hold every
# dead node until the next collection, and the collection interval grew by
# x1.5 per collect, so peak memory grew linearly with the iteration count
# (151 MB at 2M iterations, 333 MB at 4M).
#
#   powershell -ExecutionPolicy Bypass -File benchmark\measure_memory.ps1
#   powershell -ExecutionPolicy Bypass -File benchmark\measure_memory.ps1 -Iters 2000000,4000000
#
# Exit code 1 if the largest run exceeds -LimitMb (default 64).

param(
    # Iteration counts, comma separated (kept as a string so `-File` can pass
    # it: `-Iters 2000000,4000000`).
    [string]$Iters = "2000000,4000000",
    [int]$LimitMb = 64,
    [string]$Exe = "target/release/mailang.exe",
    [string]$Vm = "",
    [int]$SampleMs = 15
)

if (-not (Test-Path $Exe)) {
    Write-Error "binary missing ($Exe) - run: cargo build --release -p mailang-cli"
    exit 2
}

$prog = Join-Path $env:TEMP "mailang_mem_probe.mai"
$argList = @("run")
if ($Vm -ne "") { $argList += "--vm=$Vm" }

$sizes = $Iters -split '[,\s]+' | Where-Object { $_ -ne "" } | ForEach-Object { [int]$_ }

# M5: an empty -Vm means "whatever the CLI default is", which is the register
# VM since M5. Pass `-Vm stack` to gate the stack backend instead.
Write-Output ("binary: {0}   vm: {1}   limit: {2} MB" -f $Exe, ($Vm -replace '^$', 'default (register)'), $LimitMb)
$failed = $false

foreach ($n in $sizes) {
    [IO.File]::WriteAllText($prog, "var i = 0`nvar s = 0`nwhile i < $n {`n    s = s + len([1, 2, 3])`n    i = i + 1`n}`nprintln(s)`n")

    $p = Start-Process -FilePath $Exe -ArgumentList ($argList + @($prog)) -PassThru -NoNewWindow
    $peak = 0
    while (-not $p.HasExited) {
        try { $p.Refresh(); if ($p.PeakWorkingSet64 -gt $peak) { $peak = $p.PeakWorkingSet64 } } catch {}
        Start-Sleep -Milliseconds $SampleMs
    }
    $mb = [math]::Round($peak / 1MB, 1)
    $status = if ($mb -le $LimitMb) { "ok  " } else { "FAIL" }
    if ($mb -gt $LimitMb) { $failed = $true }
    Write-Output ("{0} {1,9} iterations   peak working set {2,7} MB" -f $status, $n, $mb)
}

Remove-Item $prog -ErrorAction SilentlyContinue
if ($failed) {
    Write-Output "FAIL: allocation loop exceeded the memory limit (regression of D4?)"
    exit 1
}
Write-Output "OK: peak working set stays flat as the iteration count grows"
