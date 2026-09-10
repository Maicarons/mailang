#!/usr/bin/env python3
"""
MaìLang Benchmark Suite - Real measurements only
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
    for _ in range(warmup):
        run_timed(cmd)

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
    print("MaìLang Benchmark Suite - Real Measurements")
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
    for py in ["python3", "python"]:
        try:
            result = subprocess.run(["where" if os.name == "nt" else "which", py],
                                  capture_output=True, text=True)
            if result.returncode == 0:
                interpreters["python"] = result.stdout.strip().split('\n')[0].strip()
                break
        except:
            pass

    # Node.js
    try:
        result = subprocess.run(["where" if os.name == "nt" else "which", "node"],
                              capture_output=True, text=True)
        if result.returncode == 0:
            interpreters["node"] = result.stdout.strip().split('\n')[0].strip()
    except:
        pass

    # QuickJS
    qjs_path = os.path.join(SCRIPT_DIR, "..", "quickjs-bin", "qjs.exe")
    if os.path.exists(qjs_path):
        interpreters["quickjs"] = qjs_path

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
            "quickjs": "compare/standard/fibonacci.js",
        },
        "loop": {
            "desc": "Sum 0..100000",
            "mai": "src/loop.mai",
            "quickjs": "compare/standard/loop.js",
        },
        "function_call": {
            "desc": "100k function calls",
            "mai": "src/function_call.mai",
            "quickjs": "compare/standard/function_call.js",
        },
        "string_interp": {
            "desc": "10k string interpolations",
            "mai": "src/string_interp.mai",
            "quickjs": "compare/standard/string_interp.js",
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
        if "quickjs" in interpreters and "quickjs" in bench_info:
            qjs_file = os.path.join(SCRIPT_DIR, bench_info["quickjs"])
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

    # Print summary
    print("=" * 70)
    print("SUMMARY (avg ms, lower is better)")
    print("=" * 70)

    langs = ["mailang", "python", "node", "quickjs"]
    available_langs = [l for l in langs if any(l in r for r in all_results.values())]

    header = f"{'Benchmark':<20}"
    for lang in available_langs:
        header += f"{lang:>12}"
    print(header)
    print("-" * len(header))

    for bench_name, results in all_results.items():
        row = f"{bench_name:<20}"
        for lang in available_langs:
            if lang in results:
                row += f"{results[lang]['avg']:>12.2f}"
            else:
                row += f"{'N/A':>12}"
        print(row)

    # Ratio vs MaìLang
    print()
    print("Ratio vs MaìLang (>1 = MaìLang faster):")
    for bench_name, results in all_results.items():
        if "mailang" not in results:
            continue
        mai_time = results["mailang"]["avg"]
        row = f"{bench_name:<20}"
        for lang in available_langs:
            if lang == "mailang":
                row += f"{'1.00':>12}"
            elif lang in results and results[lang]["avg"] > 0:
                ratio = results[lang]["avg"] / mai_time
                row += f"{ratio:>12.2f}"
            else:
                row += f"{'N/A':>12}"
        print(row)

    # Save results
    output = {"benchmarks": {}}
    for bench_name, results in all_results.items():
        output["benchmarks"][bench_name] = {}
        for lang, r in results.items():
            output["benchmarks"][bench_name][lang] = {
                "avg_ms": r["avg"],
                "median_ms": r["median"],
                "min_ms": r["min"],
                "max_ms": r["max"],
                "runs": r["runs"],
            }

    out_file = os.path.join(RESULTS_DIR, "results_real.json")
    with open(out_file, "w") as f:
        json.dump(output, f, indent=2)
    print(f"\nResults saved to: {out_file}")

if __name__ == "__main__":
    main()
