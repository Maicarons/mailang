<script setup>
import { ref, onMounted } from 'vue'

const code = ref('println("Hello, Mailang!")\nlet nums = [1, 2, 3, 4, 5]\nvar sum = 0\nfor n in nums {\n    sum = sum + n\n}\nprintln("Sum = {sum}")')

const output = ref('')
const running = ref(false)
const engineReady = ref(false)
const execTime = ref('')
let interpreter = null

const examples = {
  'Hello World': 'println("你好，MaìLang！")',
  'Variables': 'let x = 42\nlet y = 3.14\nprintln("x = {x}")\nprintln("y = {y}")',
  'Functions': 'fn add(a, b) {\n    return a + b\n}\nprintln(add(1, 2))',
  'OOP': 'class Animal {\n    let name: str\n    fn init(name: str) {\n        this.name = name\n    }\n    fn speak() -> str {\n        return "{this.name} speaks"\n    }\n}\nlet a = Animal("Rex")\nprintln(a.speak())',
  'Closures': 'fn make_counter() {\n    var count = 0\n    return fn() {\n        count = count + 1\n        return count\n    }\n}\nlet c = make_counter()\nprintln(c())\nprintln(c())',
  'Fibonacci': 'fn fib(n) {\n    if n <= 1 {\n        return n\n    }\n    return fib(n - 1) + fib(n - 2)\n}\nfor i in 0..10 {\n    println("fib({i}) = {fib(i)}")\n}',
}

function loadExample(name) {
  if (examples[name]) {
    code.value = examples[name]
  }
}

async function runCode() {
  if (!interpreter) {
    output.value = 'Engine loading...'
    return
  }
  running.value = true
  output.value = ''
  execTime.value = ''
  const start = performance.now()
  try {
    interpreter.reset()
    const result = JSON.parse(interpreter.eval_json(code.value))
    const elapsed = (performance.now() - start).toFixed(1)
    if (result.ok) {
      output.value = result.output || '(no output)'
      execTime.value = elapsed + 'ms'
    } else {
      output.value = result.error
    }
  } catch (e) {
    output.value = 'WASM error: ' + e.message
  }
  running.value = false
}

onMounted(async () => {
  try {
    const base = import.meta.env.BASE_URL || '/'
    const mod = await import(/* @vite-ignore */ base + 'wasm/mailang_wasm.js')
    await mod.default()
    interpreter = new mod.WasmInterpreter()
    engineReady.value = true
  } catch (e) {
    output.value = 'WASM load failed: ' + e.message
  }
})
</script>

# Playground

> **Project Links**: [GitHub](https://github.com/Maicarons/mailang)

Run MaìLang code online — no tools to install.

<div style="margin: 24px 0;">
  <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px; flex-wrap: wrap; gap: 8px;">
    <div style="display: flex; gap: 8px; flex-wrap: wrap;">
      <span v-for="(_, name) in examples" :key="name"
            @click="loadExample(name)"
            style="padding: 4px 12px; border: 1px solid var(--vp-c-divider); border-radius: 6px; cursor: pointer; font-size: 0.85em; user-select: none;">
        &lbrace;&lbrace; name &rbrace;&rbrace;
      </span>
    </div>
    <div style="display: flex; gap: 8px; align-items: center;">
      <span v-if="execTime" style="font-size: 0.8em; color: var(--vp-c-text-3);">&lbrace;&lbrace; execTime &rbrace;&rbrace;</span>
      <button @click="runCode" :disabled="running || !engineReady"
              style="padding: 6px 20px; background: #238636; color: #fff; border: none; border-radius: 6px; cursor: pointer; font-weight: 600;">
        &lbrace;&lbrace; running ? 'Running...' : 'Run' &rbrace;&rbrace;
      </button>
    </div>
  </div>

  <textarea v-model="code"
            style="width: 100%; min-height: 300px; padding: 16px; border: 1px solid var(--vp-c-divider); border-radius: 8px; font-family: 'JetBrains Mono', 'Fira Code', Consolas, monospace; font-size: 14px; line-height: 1.6; resize: vertical; background: var(--vp-c-bg-alt); color: var(--vp-c-text-1); tab-size: 4;"
            spellcheck="false"></textarea>

  <div v-if="output || running"
       style="margin-top: 12px; padding: 16px; border: 1px solid var(--vp-c-divider); border-radius: 8px; background: var(--vp-c-bg-alt); font-family: 'JetBrains Mono', 'Fira Code', Consolas, monospace; font-size: 13px; white-space: pre-wrap; min-height: 2em;">
    <span v-if="running" style="color: var(--vp-c-text-3);">Running...</span>
    <span v-else>&lbrace;&lbrace; output &rbrace;&rbrace;</span>
  </div>

  <div style="margin-top: 8px; font-size: 0.8em; color: var(--vp-c-text-3);">
    Tip: Press Ctrl+Enter to run &middot; Engine: &lbrace;&lbrace; engineReady ? 'Ready' : 'Loading...' &rbrace;&rbrace;
  </div>
</div>
