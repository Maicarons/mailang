/* tslint:disable */
/* eslint-disable */

export class WasmInterpreter {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Evaluate MaìLang code and return the output.
     * Returns captured println lines, or the expression result if none.
     */
    eval(code: string): string;
    /**
     * Evaluate code and return a structured result.
     * Returns a JSON string: {"ok": true, "output": "..."} or {"ok": false, "error": "..."}
     */
    eval_json(code: string): string;
    /**
     * Get supported features as a comma-separated string.
     */
    static features(): string;
    constructor();
    /**
     * Reset the interpreter state (clear globals, etc.)
     */
    reset(): void;
    /**
     * Get the version of the MaìLang interpreter.
     */
    static version(): string;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasminterpreter_free: (a: number, b: number) => void;
    readonly wasminterpreter_eval: (a: number, b: number, c: number) => [number, number];
    readonly wasminterpreter_eval_json: (a: number, b: number, c: number) => [number, number];
    readonly wasminterpreter_features: () => [number, number];
    readonly wasminterpreter_new: () => number;
    readonly wasminterpreter_reset: (a: number) => void;
    readonly wasminterpreter_version: () => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
