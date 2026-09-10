<script setup lang="ts">
import { ref, onMounted } from 'vue'
import CodeEditor from './components/CodeEditor.vue'
import OutputPanel from './components/OutputPanel.vue'
import ExampleSelector from './components/ExampleSelector.vue'
import Header from './components/Header.vue'
import { useInterpreter } from './composables/useInterpreter'
import { examples } from './examples'

const { isReady, output, error, runCode, reset } = useInterpreter()
const code = ref(examples[0].code)
const isRunning = ref(false)

async function handleRun() {
  if (!isReady.value || isRunning.value) return
  isRunning.value = true
  await runCode(code.value)
  isRunning.value = false
}

function handleReset() {
  reset()
  code.value = examples[0].code
}

function handleSelect(example: { code: string }) {
  code.value = example.code
  output.value = ''
  error.value = ''
}
</script>

<template>
  <div class="app">
    <Header
      :is-ready="isReady"
      :is-running="isRunning"
      @run="handleRun"
      @reset="handleReset"
    />
    <div class="main">
      <div class="sidebar">
        <ExampleSelector :examples="examples" @select="handleSelect" />
      </div>
      <div class="editor-pane">
        <CodeEditor v-model="code" @run="handleRun" />
      </div>
      <div class="output-pane">
        <OutputPanel :output="output" :error="error" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: #1e1e2e;
  color: #cdd6f4;
}

.main {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.sidebar {
  width: 240px;
  background: #181825;
  border-right: 1px solid #313244;
  overflow-y: auto;
}

.editor-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  border-right: 1px solid #313244;
}

.output-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
}
</style>
