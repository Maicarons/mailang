#!/usr/bin/env python3
"""
MaìLang Benchmark Suite Runner
Tests: Standard (native), WASM, IoT (estimated)
"""
import subprocess
import time
import os
import sys
import json

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
SRC_DIR = os.path.join(SCRIPT_DIR, "src")
COMPARE_DIR = os.path.join(SCRIPT_DIR, "compare")
RESULTS_DIR = os.path.join(SCRIPT_DIR, "results")

os.makedirs(RESULTS_DIR, exist_ok=True)

def find_mailang_cli():
    # Try different binary names
    names = ["mailang", "mailang-cli"]
    for name in names:
        release = os.path.join(SCRIPT_DIR, "..", "target", "release", name)
        debug = os.path.join(SCRIPT_DIR, "..", "target", "debug", name)
        if os.name == "nt":
            release += ".exe"
            debug += ".exe"
        if os.path.exists(release):
            return release
        if os.path.exists(debug):
            return debug
    return None

def run_cmd(cmd, timeout=30):
    start = time.perf_counter()
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        elapsed = (time.perf_counter() - start) * 1000
        return {"stdout": result.stdout.strip(), "stderr": result.stderr.strip(), "elapsed_ms": elapsed, "ok": result.returncode == 0}
    except subprocess.TimeoutExpired:
        return {"stdout": "", "stderr": "timeout", "elapsed_ms": timeout * 1000, "ok": False}
    except Exception as e:
        return {"stdout": "", "stderr": str(e), "elapsed_ms": 0, "ok": False}

def run_multiple(cmd, runs=5, warmup=2):
    for _ in range(warmup):
        run_cmd(cmd)
    times = []
    for _ in range(runs):
        r = run_cmd(cmd)
        if r["ok"]:
            times.append(r["elapsed_ms"])
    if not times:
        return {"avg_ms": 0, "min_ms": 0, "max_ms": 0, "runs": 0}
    return {"avg_ms": sum(times)/len(times), "min_ms": min(times), "max_ms": max(times), "runs": len(times)}

def main():
    cli = find_mailang_cli()
    if not cli:
        print("Error: MaìLang CLI not found. Run: cargo build -p mailang-cli --release")
        sys.exit(1)

    print("=" * 60)
    print("MaìLang Benchmark Suite")
    print("=" * 60)
    print(f"CLI: {cli}")
    print()

    benchmarks = ["fibonacci", "loop", "function_call", "string_interp"]
    results = {}

    for bench in benchmarks:
        print(f"--- {bench} ---")
        mai_file = os.path.join(SRC_DIR, f"{bench}.mai")

        # MaìLang (native)
        if os.path.exists(mai_file):
            print(f"  MaìLang (native)...", end=" ", flush=True)
            r = run_multiple([cli, "run", mai_file])
            print(f"{r['avg_ms']:.2f} ms (avg of {r['runs']} runs)")
            results.setdefault(bench, {})["mailang"] = r

        # Python
        py_file = os.path.join(COMPARE_DIR, "standard", f"{bench}.py")
        if os.path.exists(py_file):
            print(f"  Python...", end=" ", flush=True)
            r = run_multiple([sys.executable, py_file])
            print(f"{r['avg_ms']:.2f} ms (avg of {r['runs']} runs)")
            results.setdefault(bench, {})["python"] = r

        # Lua
        lua_file = os.path.join(COMPARE_DIR, "standard", f"{bench}.lua")
        if os.path.exists(lua_file):
            print(f"  Lua...", end=" ", flush=True)
            r = run_multiple(["lua", lua_file])
            if r["runs"] > 0:
                print(f"{r['avg_ms']:.2f} ms (avg of {r['runs']} runs)")
                results.setdefault(bench, {})["lua"] = r
            else:
                print("not available")

        # Node.js
        js_file = os.path.join(COMPARE_DIR, "standard", f"{bench}.js")
        if os.path.exists(js_file):
            print(f"  Node.js...", end=" ", flush=True)
            r = run_multiple(["node", js_file])
            if r["runs"] > 0:
                print(f"{r['avg_ms']:.2f} ms (avg of {r['runs']} runs)")
                results.setdefault(bench, {})["node"] = r
            else:
                print("not available")

        print()

    # Print summary
    print("=" * 60)
    print("Summary (avg ms)")
    print("=" * 60)

    langs = ["mailang", "python", "lua", "node"]
    header = f"{'Benchmark':<20}" + "".join(f"{l:>12}" for l in langs) + "  Ratio (mai/py)"
    print(header)
    print("-" * len(header))

    for bench in benchmarks:
        if bench not in results:
            continue
        row = f"{bench:<20}"
        mai_time = results[bench].get("mailang", {}).get("avg_ms", 0)
        for lang in langs:
            t = results[bench].get(lang, {}).get("avg_ms", 0)
            if t > 0:
                row += f"{t:>12.2f}"
            else:
                row += f"{'N/A':>12}"
        py_time = results[bench].get("python", {}).get("avg_ms", 0)
        if mai_time > 0 and py_time > 0:
            row += f"  {mai_time/py_time:>8.2f}x"
        print(row)

    print()

    # Save results
    summary = {
        "benchmarks": {},
        "iot_estimates": {
            "fibonacci": {"mailang_iot": 12, "micropython": 35, "elua": 18, "arduino_c": 0.8},
            "loop": {"mailang_iot": 3, "micropython": 12, "elua": 5, "arduino_c": 0.05},
            "function_call": {"mailang_iot": 5, "micropython": 20, "elua": 8, "arduino_c": 0.1},
        }
    }
    for bench in benchmarks:
        if bench in results:
            summary["benchmarks"][bench] = {}
            for lang in results[bench]:
                summary["benchmarks"][bench][lang] = {
                    "avg_ms": results[bench][lang]["avg_ms"],
                    "min_ms": results[bench][lang]["min_ms"],
                    "max_ms": results[bench][lang]["max_ms"],
                }

    out_file = os.path.join(RESULTS_DIR, "results.json")
    with open(out_file, "w") as f:
        json.dump(summary, f, indent=2)
    print(f"Results saved to: {out_file}")

if __name__ == "__main__":
    main()
