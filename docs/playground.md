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
  'Hello World': `println("你好，MaìLang！🌍")`,
  '变量与类型': `let x = 42
let y = 3.14
let name = "MaìLang"
println("x = {x}")
println("y = {y}")
println("name = {name}")`,
  '函数': `fn add(a, b) {
    return a + b
}
fn greet(name = "World") {
    return "Hello, {name}!"
}
println(add(1, 2))
println(greet("MaìLang"))`,
  '面向对象': `class Animal {
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
  '闭包': `fn make_counter() {
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
  '斐波那契': `fn fib(n) {
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
    output.value = '引擎加载中，请稍候...'
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
      output.value = result.output || '(无输出)'
      execTime.value = `${elapsed}ms`
    } else {
      output.value = result.error
    }
  } catch (e) {
    output.value = 'WASM 错误: ' + e.message
  }
  running.value = false
}

onMounted(async () => {
  try {
    const mod = await import('./wasm/mailang_wasm.js')
    await mod.default()
    interpreter = new mod.WasmInterpreter()
    engineReady.value = true
  } catch (e) {
    output.value = 'WASM 加载失败: ' + e.message
  }
})
</script>

# Playground

在线运行 MaìLang 代码，无需安装任何工具。

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
        {{ running ? '运行中...' : '▶ 运行' }}
      </button>
    </div>
  </div>

  <textarea v-model="code"
            style="width: 100%; min-height: 300px; padding: 16px; border: 1px solid var(--vp-c-divider); border-radius: 8px; font-family: 'JetBrains Mono', 'Fira Code', Consolas, monospace; font-size: 14px; line-height: 1.6; resize: vertical; background: var(--vp-c-bg-alt); color: var(--vp-c-text-1); tab-size: 4;"
            spellcheck="false"></textarea>

  <div v-if="output || running"
       style="margin-top: 12px; padding: 16px; border: 1px solid var(--vp-c-divider); border-radius: 8px; background: var(--vp-c-bg-alt); font-family: 'JetBrains Mono', 'Fira Code', Consolas, monospace; font-size: 13px; white-space: pre-wrap; min-height: 2em;">
    <span v-if="running" style="color: var(--vp-c-text-3);">运行中...</span>
    <span v-else :style="{ color: output.includes('Error') || output.includes('错误') ? '#f85149' : 'var(--vp-c-text-1)' }">{{ output }}</span>
  </div>

  <div style="margin-top: 8px; font-size: 0.8em; color: var(--vp-c-text-3);">
    提示：按 Ctrl+Enter 可快速运行 · 引擎状态：{{ engineReady ? '就绪 ✓' : '加载中...' }}
  </div>
</div>
