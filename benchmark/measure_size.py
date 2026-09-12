#!/usr/bin/env python3
"""Measure MaìLang binary size for host and embedded targets."""
import os
import subprocess
import sys

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


def main():
    print("=" * 60)
    print("MaìLang binary size report")
    print("=" * 60)
    print()

    # Ensure host CLI is built (release).
    r = run(["cargo", "build", "--release", "-p", "mailang-cli"])
    if r.returncode != 0:
        print(r.stderr)
        sys.exit(1)

    host_exe = os.path.join(ROOT, "target", "release", "mailang.exe")
    if not os.path.exists(host_exe):
        host_exe = os.path.join(ROOT, "target", "release", "mailang")
    if os.path.exists(host_exe):
        size = os.path.getsize(host_exe)
        print(f"host CLI (release): {size:,} bytes ({size/1024/1024:.2f} MiB)")
    else:
        print("host CLI not found")

    print()
    print("Embedded: mailang-bytecode (no_std, rlib)")
    print("-" * 60)

    for target, label in TARGETS[1:]:
        if not ensure_target(target):
            print(f"  [{target}] SKIP — target not installed (rustup target add {target})")
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
            continue

        rlib = os.path.join(
            ROOT, "target", target, "release", "libmailang_bytecode.rlib"
        )
        if os.path.exists(rlib):
            size = os.path.getsize(rlib)
            print(f"  [{label}] rlib: {size:,} bytes ({size/1024:.1f} KiB)")
        else:
            print(f"  [{label}] rlib not found at {rlib}")

    print()
    print("Notes:")
    print("  - rlib is an intermediate artifact; final MCU .bin/.elf depends on")
    print("    the firmware image, linker script, and which VM crates you include.")
    print("  - mailang-bytecode is the portable core (opcodes, values, .mailangbc format).")
    print("  - Reference (approx, for scale only): MicroPython ~300KB+, JerryScript ~200KB+ flash.")


if __name__ == "__main__":
    main()
