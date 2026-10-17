# Measure fib(30) on both VM backends (ROADMAP Phase M / M1 gate).
#
# Usage:
#   cargo build --release -p mailang-cli
#   powershell -ExecutionPolicy Bypass -File benchmark\measure_fib.ps1
param([int]$Runs = 3)

$exe = ".\target\release\mailang.exe"
if (-not (Test-Path $exe)) {
    Write-Error "release binary missing - run: cargo build --release -p mailang-cli"
    exit 1
}

function Measure-Backend($label, $argList) {
    $times = @()
    for ($i = 0; $i -lt $Runs; $i++) {
        $sw = [Diagnostics.Stopwatch]::StartNew()
        & $exe @argList bench_fib30.mai > $null
        $sw.Stop()
        $times += $sw.ElapsedMilliseconds
    }
    $min = ($times | Measure-Object -Minimum).Minimum
    $avg = [math]::Round(($times | Measure-Object -Average).Average)
    Write-Host ("{0,-9} min={1,5} ms  avg={2,5} ms  samples=[{3}]" -f $label, $min, $avg, ($times -join ", "))
    return $min
}

# M5: the register VM is the default, so BOTH compared sides name their
# backend explicitly (`--vm=stack` / `--vm=register`). The flagless line just
# reports what a plain `mailang run` costs today; the default itself is pinned
# by Rust tests (`mailang-core` `test_default_backend_is_register` and the
# `mailang-cli` `select_vm` unit tests), which is why this script does not try
# to infer it from timings - both backends are within a few percent.
$stackMs = Measure-Backend "stack" @("run", "--vm=stack")
$registerMs = Measure-Backend "register" @("run", "--vm=register")
$defaultMs = Measure-Backend "default" @("run")
$ratio = if ($stackMs -gt 0) { $registerMs / $stackMs } else { 0 }
Write-Host ("ratio    {0:N2}x  (v0.4 gate: <= 2.00x)" -f $ratio)
if ($ratio -gt 2.0) { Write-Host "M1 NOT met" } else { Write-Host "M1 met" }
Write-Host ("default run: {0} ms (should track the register line)" -f $defaultMs)