# Footprint Dashboard

> **Project Links**: [GitHub](https://github.com/Maicarons/mailang) · [IoT Deployment](/v0.3/en/guide/iot) · [Real Hardware](/v0.3/en/guide/hardware) · script [`benchmark/measure_size.py`](https://github.com/Maicarons/mailang/blob/master/benchmark/measure_size.py)

This page explains how to measure MaìLang artifact sizes with `benchmark/measure_size.py` and how to read the output. **This page invents no unmeasured numbers**: MaìLang sizes are whatever this machine measures; third-party runtimes (MicroPython, etc.) only get comparison caveats and placeholders.

## How to Run

```bash
python benchmark/measure_size.py
```

The script will:

1. `cargo build --release -p mailang-cli` and print the **host CLI** size
2. Build `mailang-bytecode` (`--no-default-features`) for any installed embedded targets and print the **rlib** size
3. Emit a paste-ready **Markdown table** (local date / platform is written above the table header)

Targets that are not installed are marked `—` with a reason, not filled with 0.

## How to Read the Output

| Column | Meaning |
|--------|---------|
| Artifact | The product: `mailang-cli` or the `mailang-bytecode` rlib |
| Target | Purpose label for the host or embedded triple |
| Size (bytes) | **Locally measured** byte count |
| Size | Human-readable (KiB / MiB) |
| Notes | Scope of the artifact |

### Key Distinctions

- **host CLI**: full `std` interpreter + REPL + module system — **not** MCU firmware.
- **mailang-bytecode rlib**: the `no_std` portable core (instructions, `Value`, `.mailangbc` format). It is an **intermediate product**, not a flashable `.bin`.
- **Final firmware** = rlib/VM code + your HAL + linker script + startup code. Use a linker map / `size` tool to inspect the final image.

## Sample Table (placeholders — trust your local run)

The numeric columns below are **placeholders**. Run the script and paste the real output into release notes or a PR:

| Artifact | Target | Size (bytes) | Size | Notes |
|----------|--------|-------------:|------|-------|
| mailang-cli (release) | host | *(run script)* | *(run script)* | full std CLI |
| mailang-bytecode rlib | embedded (Cortex-M4F) | *(run script / target installed)* | *(…)* | no_std + alloc core |
| mailang-bytecode rlib | embedded (ESP32-C3 class) | *(run script / target installed)* | *(…)* | no_std + alloc core |

## MicroPython / JerryScript Comparison Caveats

**Do not** compare numbers like "MicroPython ~300KB" from the web directly against the rlib:

| Factor | Explanation |
|--------|-------------|
| Different artifact shapes | rlib ≠ `.bin`; MicroPython numbers are usually complete firmware images |
| Different feature scope | MicroPython includes REPL, filesystem, many built-in modules; the MaìLang embedded path can link just bytecode + VM |
| Board-level differences | The same runtime differs a lot across flash layouts and linker scripts |
| Build options | LTO, `opt-level = "z"`, panic strategy, and stripping all change the result |
| Version drift | Numbers change when upstream versions update |

When comparing:

1. Measure both sides on the **same board**, **same toolchain**, and a **similar feature set**;
2. Record the commit, profile, target, and whether FS/REPL is included in the report;
3. Treat comparison rows in the script output as scale notes only — **keep them as placeholders or write "vendor-dependent"** unless you measured them yourself.

## Measuring Embedded Targets

```bash
rustup target add thumbv7em-none-eabihf
rustup target add riscv32imc-unknown-none-elf
python benchmark/measure_size.py
```

Measure the final MCU image separately:

```bash
# Example: inspect the linked product
arm-none-eabi-size firmware.elf
# or parse the .map for .text/.rodata/.bss
```

## Scope Statement (honest boundaries)

- This tool only measures the **host CLI** and the **mailang-bytecode rlib**.
- It does **not** measure complete ESP32 / Cortex-M4 firmware.
- It ships **no** built-in third-party runtime baseline numbers; comparison rows are deliberately blank or marked "measure locally / vendor-dependent".

After running, archive the Markdown table together with its date and triples so stale numbers are not reused.
