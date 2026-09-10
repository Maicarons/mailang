#!/usr/bin/env python3
"""
MaìLang Full Benchmark Suite
Tests: Native, WASM estimation, IoT estimation
Compares: Python, Lua, Node.js, QuickJS
"""
import subprocess
import time
import os
import sys
import json
import statistics

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
SRC_DIR = os.path.join(SCRIPT_DIR, "src")
COMPARE_DIR = os.path.join(SCRIPT_DIR, "compare")
RESULTS_DIR = os.path.join(SCRIPT_DIR, "results")
os.makedirs(RESULTS_DIR, exist_ok=True)

def find_binary(name):
    """Find a binary in PATH"""
    try:
        result = subprocess.run(["where" if os.name == "nt" else "which", name],
                              capture_output=True, text=True)
        if result.returncode == 0:
            return result.stdout.strip().split('\n')[0].strip()
    except:
        pass
    return None

def run_timed(cmd, timeout=60):
    """Run command and measure time"""
    start = time.perf_counter()
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        elapsed_ms = (time.perf_counter() - start) * 1000
        return {
            "ok": result.returncode == 0,
            "stdout": result.stdout.strip(),
            "stderr": result.stderr.strip(),
            "elapsed_ms": elapsed_ms
        }
    except subprocess.TimeoutExpired:
        return {"ok": False, "stdout": "", "stderr": "timeout", "elapsed_ms": timeout * 1000}
    except Exception as e:
        return {"ok": False, "stdout": "", "stderr": str(e), "elapsed_ms": 0}

def run_benchmark(cmd, runs=7, warmup=3):
    """Run benchmark multiple times and collect statistics"""
    # Warmup
    for _ in range(warmup):
        run_timed(cmd)

    # Measure
    times = []
    for _ in range(runs):
        r = run_timed(cmd)
        if r["ok"]:
            times.append(r["elapsed_ms"])

    if not times:
        return None

    return {
        "avg": statistics.mean(times),
        "median": statistics.median(times),
        "min": min(times),
        "max": max(times),
        "stdev": statistics.stdev(times) if len(times) > 1 else 0,
        "runs": len(times)
    }

def main():
    print("=" * 70)
    print("MaìLang Benchmark Suite - Full Comparison")
    print("=" * 70)
    print()

    # Find interpreters
    interpreters = {}

    # MaìLang
    for name in ["mailang", "mailang-cli"]:
        for ext in ["", ".exe"]:
            for profile in ["release", "debug"]:
                path = os.path.join(SCRIPT_DIR, "..", "target", profile, name + ext)
                if os.path.exists(path):
                    interpreters["mailang"] = path
                    break

    # Python
    py = find_binary("python3") or find_binary("python")
    if py:
        interpreters["python"] = py

    # Lua
    lua = find_binary("lua")
    if lua:
        interpreters["lua"] = lua

    # Node.js
    node = find_binary("node")
    if node:
        interpreters["node"] = node

    # QuickJS
    qjs = find_binary("qjs") or find_binary("quickjs")
    if qjs:
        interpreters["quickjs"] = qjs

    print("Available interpreters:")
    for name, path in interpreters.items():
        print(f"  {name}: {path}")
    print()

    if "mailang" not in interpreters:
        print("Error: MaìLang CLI not found. Run: cargo build -p mailang-cli --release")
        sys.exit(1)

    # Benchmarks
    benchmarks = {
        "fibonacci": {
            "desc": "Fibonacci(30) recursive",
            "mai": "src/fibonacci.mai",
        },
        "loop": {
            "desc": "Sum 0..100000",
            "mai": "src/loop.mai",
        },
        "function_call": {
            "desc": "100k function calls",
            "mai": "src/function_call.mai",
        },
        "string_interp": {
            "desc": "10k string interpolations",
            "mai": "src/string_interp.mai",
        },
    }

    all_results = {}

    for bench_name, bench_info in benchmarks.items():
        print(f"--- {bench_name}: {bench_info['desc']} ---")
        results = {}

        # MaìLang
        mai_file = os.path.join(SCRIPT_DIR, bench_info["mai"])
        if os.path.exists(mai_file):
            print(f"  MaìLang...", end=" ", flush=True)
            r = run_benchmark([interpreters["mailang"], "run", mai_file])
            if r:
                results["mailang"] = r
                print(f"{r['avg']:.2f} ms (avg)")
            else:
                print("failed")

        # Python
        if "python" in interpreters:
            py_file = os.path.join(COMPARE_DIR, "standard", f"{bench_name}.py")
            if os.path.exists(py_file):
                print(f"  Python...", end=" ", flush=True)
                r = run_benchmark([interpreters["python"], py_file])
                if r:
                    results["python"] = r
                    print(f"{r['avg']:.2f} ms (avg)")
                else:
                    print("failed")

        # Lua
        if "lua" in interpreters:
            lua_file = os.path.join(COMPARE_DIR, "standard", f"{bench_name}.lua")
            if os.path.exists(lua_file):
                print(f"  Lua...", end=" ", flush=True)
                r = run_benchmark([interpreters["lua"], lua_file])
                if r:
                    results["lua"] = r
                    print(f"{r['avg']:.2f} ms (avg)")
                else:
                    print("failed")

        # Node.js
        if "node" in interpreters:
            js_file = os.path.join(COMPARE_DIR, "standard", f"{bench_name}.js")
            if os.path.exists(js_file):
                print(f"  Node.js...", end=" ", flush=True)
                r = run_benchmark([interpreters["node"], js_file])
                if r:
                    results["node"] = r
                    print(f"{r['avg']:.2f} ms (avg)")
                else:
                    print("failed")

        # QuickJS
        if "quickjs" in interpreters:
            qjs_file = os.path.join(COMPARE_DIR, "standard", f"{bench_name}.js")
            if os.path.exists(qjs_file):
                print(f"  QuickJS...", end=" ", flush=True)
                r = run_benchmark([interpreters["quickjs"], qjs_file])
                if r:
                    results["quickjs"] = r
                    print(f"{r['avg']:.2f} ms (avg)")
                else:
                    print("failed")

        all_results[bench_name] = results
        print()

    # Print summary table
    print("=" * 70)
    print("SUMMARY (avg ms, lower is better)")
    print("=" * 70)

    langs = ["mailang", "python", "lua", "node", "quickjs"]
    available_langs = [l for l in langs if any(l in r for r in all_results.values())]

    # Header
    header = f"{'Benchmark':<20}"
    for lang in available_langs:
        header += f"{lang:>12}"
    print(header)
    print("-" * len(header))

    # Data rows
    for bench_name, results in all_results.items():
        row = f"{bench_name:<20}"
        for lang in available_langs:
            if lang in results:
                row += f"{results[lang]['avg']:>12.2f}"
            else:
                row += f"{'N/A':>12}"
        print(row)

    # Ratio row
    print()
    print("Ratio vs MaìLang (lower = MaìLang faster):")
    for bench_name, results in all_results.items():
        if "mailang" not in results:
            continue
        mai_time = results["mailang"]["avg"]
        row = f"{bench_name:<20}"
        for lang in available_langs:
            if lang == "mailang":
                row += f"{'1.00':>12}"
            elif lang in results and results[lang]["avg"] > 0:
                ratio = mai_time / results[lang]["avg"]
                row += f"{ratio:>12.2f}"
            else:
                row += f"{'N/A':>12}"
        print(row)

    # IoT estimation
    print()
    print("=" * 70)
    print("IoT ESTIMATION (based on native PC performance)")
    print("=" * 70)
    print()
    print("Assumption: Cortex-M4 @ 168MHz is ~100x slower than modern PC")
    print("Assumption: MicroPython is ~3x slower than CPython on same hardware")
    print("Assumption: eLua is ~2x slower than standard Lua")
    print()

    PC_TO_IOT = 100  # Cortex-M4 is ~100x slower than PC

    print(f"{'Benchmark':<20}{'MaìLang':>12}{'MicroPython':>12}{'eLua':>12}{'Arduino C':>12}")
    print("-" * 68)

    for bench_name, results in all_results.items():
        mai_time = results.get("mailang", {}).get("avg", 0) / 1000  # to seconds
        py_time = results.get("python", {}).get("avg", 0) / 1000
        lua_time = results.get("lua", {}).get("avg", 0) / 1000

        mai_iot = mai_time * PC_TO_IOT * 1000  # ms on IoT
        micropython = py_time * PC_TO_IOT * 3 * 1000  # MicroPython is ~3x slower than CPython
        elua = lua_time * PC_TO_IOT * 2 * 1000 if lua_time > 0 else 0
        arduino_c = mai_iot / 50  # Native C is ~50x faster than interpreted

        row = f"{bench_name:<20}{mai_iot:>10.1f} ms"
        row += f"{micropython:>10.1f} ms"
        if elua > 0:
            row += f"{elua:>10.1f} ms"
        else:
            row += f"{'N/A':>12}"
        row += f"{arduino_c:>10.2f} ms"
        print(row)

    # Save results
    output = {
        "native": {},
        "iot_estimates": {}
    }
    for bench_name, results in all_results.items():
        output["native"][bench_name] = {}
        for lang, r in results.items():
            output["native"][bench_name][lang] = {
                "avg_ms": r["avg"],
                "median_ms": r["median"],
                "min_ms": r["min"],
                "max_ms": r["max"],
            }

        mai_time = results.get("mailang", {}).get("avg", 0) / 1000
        py_time = results.get("python", {}).get("avg", 0) / 1000
        lua_time = results.get("lua", {}).get("avg", 0) / 1000

        output["iot_estimates"][bench_name] = {
            "mailang_iot_ms": mai_time * PC_TO_IOT * 1000,
            "micropython_ms": py_time * PC_TO_IOT * 3 * 1000,
            "elua_ms": lua_time * PC_TO_IOT * 2 * 1000 if lua_time > 0 else None,
            "arduino_c_ms": mai_time * PC_TO_IOT * 1000 / 50,
        }

    out_file = os.path.join(RESULTS_DIR, "results_full.json")
    with open(out_file, "w") as f:
        json.dump(output, f, indent=2)
    print(f"\nResults saved to: {out_file}")

if __name__ == "__main__":
    main()
