# Verify the register VM is behaviourally identical to the stack VM on real
# programs: every .mai program under the given roots is executed on both
# backends and stdout + stderr (including error messages) must match byte for
# byte.
#
# Two kinds of program cannot be compared and are skipped:
#   * output that is not deterministic (it prints wall-clock time), so two runs
#     of the *same* backend differ too;
#   * an interactive event loop with no event source in simulation mode, which
#     never terminates.
#
# Programs that are known to differ are listed in
# `benchmark/known_backend_mismatches.txt`. The run FAILS if a program differs
# that is not listed there, and warns about a listed program that has started
# matching (so the list can be pruned without blocking the fix that pruned it).
#
# Runs on Windows PowerShell and on PowerShell 7 (Linux/macOS CI).
#
# Usage:
#   cargo build --release -p mailang-cli
#   pwsh benchmark/compare_backends.ps1
param(
    [string[]]$Roots = @("examples", "libs"),
    [string[]]$Skip = @(
        "libs/time/example/basic.mai",
        "libs/gui/example/interactive.mai"
    ),
    [string]$KnownMismatchFile = "benchmark/known_backend_mismatches.txt"
)

# A native command writing to stderr must not become a terminating error
# (PowerShell 7.3+; absent and harmless on 5.1).
if (Test-Path variable:PSNativeCommandUseErrorActionPreference) {
    $PSNativeCommandUseErrorActionPreference = $false
}

# `$IsWindows` only exists on PowerShell 6+; 5.1 is always Windows.
$onWindows = ($IsWindows -or $env:OS -eq 'Windows_NT') -eq $true
$exeName = if ($onWindows) { "mailang.exe" } else { "mailang" }
$exe = "target/release/$exeName"
if (-not (Test-Path $exe)) {
    Write-Error "release binary missing ($exe) - run: cargo build --release -p mailang-cli"
    exit 1
}

# Repository-relative path with forward slashes, as written in the manifest.
function Get-RelPath([string]$Full) {
    $rel = Resolve-Path -Relative $Full
    return (($rel -replace '^[.][\\/]', '') -replace '\\', '/')
}

# Run one backend and return "exit<LF>output". stderr is merged into stdout in
# emission order and `"$_"` renders each native-stderr record as its plain text
# (redirecting to a file would inject PowerShell's "At line ... CategoryInfo"
# decoration instead).
function Invoke-Backend([string]$Exe, [string[]]$VmArgs, [string]$File) {
    $lines = & $Exe run @VmArgs $File 2>&1 | ForEach-Object { "$_" }
    $exit = $LASTEXITCODE
    # Line endings are platform noise, not behaviour.
    $text = (($lines -join "`n") -replace "`r`n", "`n").TrimEnd("`r")
    return "exit $exit`n$text"
}

# One run's result, trimmed for the report.
function Summarize([string]$Result) {
    $flat = ($Result -replace "`n", " | ")
    if ($flat.Length -gt 300) { $flat = $flat.Substring(0, 300) + " ..." }
    return $flat
}

$files = @()
foreach ($root in $Roots) {
    if (Test-Path $root) {
        $files += Get-ChildItem -Path $root -Recurse -Filter *.mai | Sort-Object FullName
    }
}

$skipped = @()
$compared = @()
foreach ($f in $files) {
    $rel = Get-RelPath $f.FullName
    if ($Skip -contains $rel) { $skipped += $rel } else { $compared += $f }
}

$mismatched = @()
foreach ($f in $compared) {
    $rel = Get-RelPath $f.FullName
    # M5: the register VM is the default, so name both sides explicitly rather
    # than relying on the flagless run (which is register now).
    $stack = Invoke-Backend $exe @("--vm=stack") $f.FullName
    $reg = Invoke-Backend $exe @("--vm=register") $f.FullName
    if ($stack -ne $reg) {
        $mismatched += $rel
        Write-Host "MISMATCH: $rel"
        Write-Host "  stack:    $(Summarize $stack)"
        Write-Host "  register: $(Summarize $reg)"
    }
}

$known = @()
if (Test-Path $KnownMismatchFile) {
    $known = @(
        Get-Content $KnownMismatchFile |
            ForEach-Object { $_.Trim() } |
            Where-Object { $_ -ne "" -and -not $_.StartsWith("#") }
    )
}

$extra = @($mismatched | Where-Object { $known -notcontains $_ })
$fixed = @($known | Where-Object { $mismatched -notcontains $_ })

Write-Host ""
Write-Host "compared: $($compared.Count)    identical: $($compared.Count - $mismatched.Count)    mismatched: $($mismatched.Count)    skipped: $($skipped.Count)"
foreach ($s in $skipped) { Write-Host "  skipped: $s" }

if ($fixed.Count -gt 0) {
    Write-Host ""
    Write-Host "NOTE: these listed programs now agree - remove them from $KnownMismatchFile :"
    foreach ($f in $fixed) { Write-Host "  $f" }
}

if ($extra.Count -gt 0) {
    Write-Host ""
    Write-Host "FAIL: $($extra.Count) program(s) differ but are not in $KnownMismatchFile :"
    foreach ($e in $extra) { Write-Host "  $e" }
    exit 1
}

Write-Host ""
Write-Host "OK: no new backend mismatch"
