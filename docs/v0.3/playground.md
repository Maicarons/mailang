<script setup>
import { ref, onMounted } from 'vue'

const code = ref(`class Animal {
    let name: str
    fn init(name: str) {
        this.name = name
    }
    fn speak() -> str {
        return "{this.name} speaks"
    }
}

class Dog extends Animal {
    override fn speak() -> str {
        return "{this.name} barks!"
    }
}

let dog = Dog("Rex")
println(dog.speak())

let nums = [1, 2, 3, 4, 5]
var sum = 0
for n in nums {
    sum = sum + n
}
println("Sum = {sum}")`)

const output = ref('')
const running = ref(false)
const engineReady = ref(false)
const execTime = ref('')
let interpreter = null

const examples = {
  'Hello World': `println("浣犲ソ锛孧a矛Lang锛侌煂?)`,
  '鍙橀噺涓庣被鍨?: `let x = 42
let y = 3.14
let name = "Ma矛Lang"
println("x = {x}")
println("y = {y}")
println("name = {name}")`,
  '鍑芥暟': `fn add(a, b) {
    return a + b
}
fn greet(name: str) {
    return "Hello, {name}!"
}
println(add(1, 2))
println(greet("Ma矛Lang"))`,
  '闈㈠悜瀵硅薄': `class Animal {
    let name: str
    fn init(name: str) {
        this.name = name
    }
    fn speak() -> str {
        return "{this.name} speaks"
    }
}
class Dog extends Animal {
    override fn speak() -> str {
        return "{this.name} barks!"
    }
}
let dog = Dog("Rex")
println(dog.speak())`,
  '闂寘': `fn make_counter() {
    var count = 0
    return fn() {
        count = count + 1
        return count
    }
}
let counter = make_counter()
println(counter())
println(counter())
println(counter())`,
  '鏂愭尝閭ｅ': `fn fib(n) {
    if n <= 1 {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}
for i in 0..10 {
    println("fib({i}) = {fib(i)}")
}`,
}

function loadExample(name) {
  if (examples[name]) {
    code.value = examples[name]
  }
}

async function runCode() {
  if (!interpreter) {
    output.value = '寮曟搸鍔犺浇涓紝璇风◢鍊?..'
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
      output.value = result.output || '(鏃犺緭鍑?'
      execTime.value = `${elapsed}ms`
    } else {
      output.value = result.error
    }
  } catch (e) {
    output.value = 'WASM 閿欒: ' + e.message
  }
  running.value = false
}

onMounted(async () => {
  try {
    const mod = await import(/* @vite-ignore */ (import.meta.env.BASE_URL || '/') + 'wasm/mailang_wasm.js')
    await mod.default()
    interpreter = new mod.WasmInterpreter()
    engineReady.value = true
  } catch (e) {
    output.value = 'WASM 鍔犺浇澶辫触: ' + e.message
  }
})
</script>

# Playground

鍦ㄧ嚎杩愯 Ma矛Lang 浠ｇ爜锛屾棤闇€瀹夎浠讳綍宸ュ叿銆?

<div style="margin: 24px 0;">
  <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px;">
    <div style="display: flex; gap: 8px; flex-wrap: wrap;">
      <span v-for="(_, name) in examples" :key="name"
            @click="loadExample(name)"
            style="padding: 4px 12px; border: 1px solid var(--vp-c-divider); border-radius: 6px; cursor: pointer; font-size: 0.85em; user-select: none;">
        {{ name }}
      </span>
    </div>
    <div style="display: flex; gap: 8px; align-items: center;">
      <span v-if="execTime" style="font-size: 0.8em; color: var(--vp-c-text-3);">{{ execTime }}</span>
      <button @click="runCode" :disabled="running || !engineReady"
              style="padding: 6px 20px; background: #238636; color: #fff; border: none; border-radius: 6px; cursor: pointer; font-weight: 600;">
        {{ running ? '杩愯涓?..' : '鈻?杩愯' }}
      </button>
    </div>
  </div>

  <textarea v-model="code"
            style="width: 100%; min-height: 300px; padding: 16px; border: 1px solid var(--vp-c-divider); border-radius: 8px; font-family: 'JetBrains Mono', 'Fira Code', Consolas, monospace; font-size: 14px; line-height: 1.6; resize: vertical; background: var(--vp-c-bg-alt); color: var(--vp-c-text-1); tab-size: 4;"
            spellcheck="false"></textarea>

  <div v-if="output || running"
       style="margin-top: 12px; padding: 16px; border: 1px solid var(--vp-c-divider); border-radius: 8px; background: var(--vp-c-bg-alt); font-family: 'JetBrains Mono', 'Fira Code', Consolas, monospace; font-size: 13px; white-space: pre-wrap; min-height: 2em;">
    <span v-if="running" style="color: var(--vp-c-text-3);">杩愯涓?..</span>
    <span v-else :style="{ color: output.includes('Error') || output.includes('閿欒') ? '#f85149' : 'var(--vp-c-text-1)' }">{{ output }}</span>
  </div>

  <div style="margin-top: 8px; font-size: 0.8em; color: var(--vp-c-text-3);">
    鎻愮ず锛氭寜 Ctrl+Enter 鍙揩閫熻繍琛?路 寮曟搸鐘舵€侊細{{ engineReady ? '灏辩华 鉁? : '鍔犺浇涓?..' }}
  </div>
</div>
