#!/usr/bin/env python3
"""Measure MaìLang binary size for host and embedded targets.

Prints:
  - host CLI binary size (release)
  - mailang-bytecode rlib size (no_std) for installed embedded targets
  - a markdown table snippet you can paste into docs / PR descriptions

Only sizes measured on THIS machine are printed as numbers. Third-party
figures (MicroPython etc.) are caveats/placeholders — never invented sizes.
"""
import os
import platform
import subprocess
import sys
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Targets we care about for IoT claims.
TARGETS = [
    ("x86_64-pc-windows-msvc", "host"),
    ("thumbv7em-none-eabihf", "embedded (Cortex-M4F)"),
    ("riscv32imc-unknown-none-elf", "embedded (ESP32-C3 class)"),
]


def run(cmd, cwd=ROOT):
    print("+", " ".join(cmd))
    return subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)


def ensure_target(target: str) -> bool:
    r = run(["rustup", "target", "list", "--installed"])
    return target in r.stdout


def human(n: int) -> str:
    if n >= 1024 * 1024:
        return f"{n/1024/1024:.2f} MiB"
    return f"{n/1024:.1f} KiB"


def main():
    print("=" * 60)
    print("MaìLang binary size report")
    print("=" * 60)
    print(f"host: {platform.system()} {platform.machine()}  date: {time.strftime('%Y-%m-%d')}")
    print()

    rows = []  # (artifact, target/label, bytes or None, note)

    # Ensure host CLI is built (release).
    r = run(["cargo", "build", "--release", "-p", "mailang-cli"])
    if r.returncode != 0:
        print(r.stderr)
        sys.exit(1)

    host_exe = os.path.join(ROOT, "target", "release", "mailang.exe")
    if not os.path.exists(host_exe):
        host_exe = os.path.join(ROOT, "target", "release", "mailang")
    host_size = None
    if os.path.exists(host_exe):
        host_size = os.path.getsize(host_exe)
        print(f"host CLI (release): {host_size:,} bytes ({human(host_size)})")
        rows.append(("mailang-cli (release)", "host", host_size, "full std CLI"))
    else:
        print("host CLI not found")
        rows.append(("mailang-cli (release)", "host", None, "not built"))

    print()
    print("Embedded: mailang-bytecode (no_std, rlib)")
    print("-" * 60)

    for target, label in TARGETS[1:]:
        if not ensure_target(target):
            print(f"  [{target}] SKIP — target not installed (rustup target add {target})")
            rows.append(("mailang-bytecode rlib", label, None, f"target not installed ({target})"))
            continue
        r = run(
            [
                "cargo",
                "build",
                "--release",
                "-p",
                "mailang-bytecode",
                "--no-default-features",
                "--target",
                target,
            ]
        )
        if r.returncode != 0:
            print(f"  [{target}] BUILD FAILED")
            print(r.stderr[-2000:])
            rows.append(("mailang-bytecode rlib", label, None, "build failed"))
            continue

        rlib = os.path.join(
            ROOT, "target", target, "release", "libmailang_bytecode.rlib"
        )
        # Windows MSVC may emit mailang_bytecode.lib naming variants
        if not os.path.exists(rlib):
            alt = os.path.join(ROOT, "target", target, "release", "mailang_bytecode.lib")
            rlib = alt if os.path.exists(alt) else rlib
        if os.path.exists(rlib):
            size = os.path.getsize(rlib)
            print(f"  [{label}] rlib: {size:,} bytes ({human(size)})")
            rows.append(("mailang-bytecode rlib", label, size, "no_std + alloc core"))
        else:
            print(f"  [{label}] rlib not found at {rlib}")
            rows.append(("mailang-bytecode rlib", label, None, "rlib path missing"))

    # Markdown table snippet
    print()
    print("Markdown table snippet")
    print("-" * 60)
    print()
    print("| Artifact | Target | Size (bytes) | Size | Notes |")
    print("|----------|--------|-------------:|------|-------|")
    for artifact, label, size, note in rows:
        if size is None:
            print(f"| {artifact} | {label} | — | — | {note} |")
        else:
            print(f"| {artifact} | {label} | {size:,} | {human(size)} | {note} |")
    print()
    print("| Runtime (for scale only) | Flash (typical) | Notes |")
    print("|--------------------------|----------------:|-------|")
    print("| MaìLang host CLI | *(measure above)* | full std host binary |")
    print("| MaìLang mailang-bytecode rlib | *(measure above)* | portable core only; not a firmware image |")
    print("| MicroPython | *(vendor/board dependent)* | full VM + flash FS + REPL; compare like-for-like |")
    print("| JerryScript | *(vendor/build dependent)* | engine only vs full SDK matters |")
    print()

    print("Notes:")
    print("  - rlib is an intermediate artifact; final MCU .bin/.elf depends on")
    print("    the firmware image, linker script, and which VM crates you include.")
    print("  - mailang-bytecode is the portable core (opcodes, values, .mailangbc format).")
    print("  - MicroPython / JerryScript cells are placeholders on purpose:")
    print("    do NOT paste unverified third-party sizes as MaìLang measurements.")
    print("  - How to read this table: docs/guide/footprint.md")


if __name__ == "__main__":
    main()
