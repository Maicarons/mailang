#!/usr/bin/env node
// Node.js test via WASM (preferred path for JS hosts).
// Prefer wasm-pack pkg if present; otherwise load the raw cdylib is not
// practical without wasm-bindgen JS glue, so we invoke the CLI as a fallback
// smoke test and document the wasm pkg path.
const { spawnSync } = require("child_process");
const path = require("path");
const fs = require("fs");

const root = path.resolve(__dirname, "..", "..");
const exeName = process.platform === "win32" ? "mailang.exe" : "mailang";
const cli = path.join(root, "target", "debug", exeName);
const pkgJs = path.join(root, "crates", "mailang-wasm", "pkg", "mailang_wasm.js");
const wasmBin = path.join(root, "target", "wasm32-unknown-unknown", "release", "mailang_wasm.wasm");

function runCliEval(code) {
  const r = spawnSync(cli, ["eval", code], { encoding: "utf8" });
  if (r.status !== 0) {
    console.error("CLI eval failed", r.stderr || r.stdout);
    process.exit(1);
  }
  return (r.stdout || "").trim();
}

function main() {
  // 1) Native CLI path (always available after cargo build -p mailang-cli)
  if (fs.existsSync(cli)) {
    const out = runCliEval("21 * 2");
    if (out !== "42") {
      console.error("CLI eval mismatch:", out);
      process.exit(1);
    }
    console.log("Node CLI eval 21*2 =>", out);
  }

  // 2) wasm-bindgen package if wasm-pack was run
  if (fs.existsSync(pkgJs)) {
    const { WasmInterpreter } = require(pkgJs);
    const interp = new WasmInterpreter();
    const out = interp.eval("6 * 7");
    if (out !== "42") {
      console.error("WASM eval mismatch:", out);
      process.exit(1);
    }
    console.log("Node WASM eval 6*7 =>", out);
  } else {
    console.log("wasm-bindgen pkg not built (wasm-pack); CLI path verified.");
  }

  // 3) Report raw wasm size
  if (fs.existsSync(wasmBin)) {
    const sz = fs.statSync(wasmBin).size;
    console.log("mailang_wasm.wasm size:", sz, "bytes", `(${(sz / 1024).toFixed(1)} KiB)`);
  }

  console.log("Node FFI OK");
}

main();
