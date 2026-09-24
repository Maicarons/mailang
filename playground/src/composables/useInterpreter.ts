import { ref, onMounted } from 'vue'

interface WasmInterpreterInstance {
  eval(code: string): string
  eval_json(code: string): string
  reset(): void
  free(): void
}

let wasmModule: any = null

export function useInterpreter() {
  const isReady = ref(false)
  const output = ref('')
  const error = ref('')
  let interpreter: WasmInterpreterInstance | null = null

  async function initWasm() {
    try {
      // Load WASM module using fetch + blob URL approach
      const response = await fetch('/wasm/mailang_wasm.js')
      if (!response.ok) throw new Error('WASM JS not found')

      const jsText = await response.text()

      // Patch import.meta.url to use absolute path
      const patchedJs = jsText.replace(
        "new URL('mailang_wasm_bg.wasm', import.meta.url)",
        "new URL('/wasm/mailang_wasm_bg.wasm', window.location.origin)"
      )
      const blob = new Blob([patchedJs], { type: 'application/javascript' })
      const blobUrl = URL.createObjectURL(blob)

      wasmModule = await import(/* @vite-ignore */ blobUrl)
      URL.revokeObjectURL(blobUrl)

      await wasmModule.default()
      interpreter = new wasmModule.WasmInterpreter()
      isReady.value = true
    } catch (e) {
      console.error('Failed to load WASM:', e)
      error.value = 'Failed to load MaìLang interpreter'
    }
  }

  /** Full error string for UI: keep line/col if present, never truncate. */
  function formatError(raw: unknown): string {
    if (raw == null) return 'Unknown error'
    if (typeof raw === 'string') return raw
    const e = raw as { message?: unknown; stack?: unknown }
    // Prefer message (often includes "at line X, column Y"); fall back to full object.
    if (typeof e.message === 'string' && e.message.length > 0) return e.message
    try {
      return JSON.stringify(raw)
    } catch {
      return String(raw)
    }
  }

  /** Interpreter eval returns "Error: ..." for failures; also catch bare error-ish strings. */
  function isErrorResult(result: string): boolean {
    if (result.startsWith('Error:') || result.startsWith('Error ')) return true
    if (/^(SyntaxError|ParserError|RuntimeError|LexerError|TypeError|VmError)\b/.test(result)) {
      return true
    }
    // Multi-line diagnostics that include line/col
    if (/\bat line \d+/.test(result) && /error|Error|Invalid|Unexpected|Unterminated/.test(result)) {
      return true
    }
    return false
  }

  async function runCode(code: string) {
    if (!interpreter) {
      error.value = 'Interpreter not initialized'
      return
    }

    output.value = ''
    error.value = ''

    try {
      const result = interpreter.eval(code)
      if (isErrorResult(result)) {
        // Show the full error string (including line/col if present)
        error.value = result
      } else {
        output.value = result
      }
    } catch (e: any) {
      error.value = formatError(e)
    }
  }

  function reset() {
    if (interpreter) {
      interpreter.reset()
    }
    output.value = ''
    error.value = ''
  }

  onMounted(() => {
    initWasm()
  })

  return {
    isReady,
    output,
    error,
    runCode,
    reset,
  }
}
