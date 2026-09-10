<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { EditorView, basicSetup } from 'codemirror'
import { EditorState } from '@codemirror/state'
import { javascript } from '@codemirror/lang-javascript'
import { oneDark } from '@codemirror/theme-one-dark'
import { keymap } from '@codemirror/view'

const props = defineProps<{
  modelValue: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  run: []
}>()

const editorRef = ref<HTMLDivElement>()
let editorView: EditorView | undefined

// MaìLang-like syntax highlighting (using JavaScript mode as base)
const maiLanguage = javascript()

// Custom keymap for Ctrl+Enter to run
const runKeymap = keymap.of([{
  key: 'Ctrl-Enter',
  run: () => {
    emit('run')
    return true
  }
}])

onMounted(() => {
  if (!editorRef.value) return

  const updateListener = EditorView.updateListener.of((update) => {
    if (update.docChanged) {
      emit('update:modelValue', update.state.doc.toString())
    }
  })

  const state = EditorState.create({
    doc: props.modelValue,
    extensions: [
      basicSetup,
      maiLanguage,
      oneDark,
      runKeymap,
      updateListener,
      EditorView.theme({
        '&': {
          height: '100%',
          fontSize: '14px',
        },
        '.cm-scroller': {
          fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
        },
      }),
    ],
  })

  editorView = new EditorView({
    state,
    parent: editorRef.value,
  })
})

onUnmounted(() => {
  editorView?.destroy()
})

watch(() => props.modelValue, (newVal) => {
  if (editorView && editorView.state.doc.toString() !== newVal) {
    editorView.dispatch({
      changes: {
        from: 0,
        to: editorView.state.doc.length,
        insert: newVal,
      },
    })
  }
})
</script>

<template>
  <div class="editor-container">
    <div class="editor-header">
      <span class="editor-title">📝 Editor</span>
      <span class="editor-hint">Ctrl+Enter to run</span>
    </div>
    <div ref="editorRef" class="editor"></div>
  </div>
</template>

<style scoped>
.editor-container {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.editor-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 12px;
  background: #181825;
  border-bottom: 1px solid #313244;
  font-size: 12px;
}

.editor-title {
  color: #89b4fa;
  font-weight: 500;
}

.editor-hint {
  color: #6c7086;
}

.editor {
  flex: 1;
  overflow: hidden;
}

.editor :deep(.cm-editor) {
  height: 100%;
}
</style>
