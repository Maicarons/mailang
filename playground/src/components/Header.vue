<script setup lang="ts">
defineProps<{
  isReady: boolean
  isRunning: boolean
}>()

defineEmits<{
  run: []
  reset: []
}>()
</script>

<template>
  <header class="header">
    <div class="logo">
      <span class="logo-icon">麦</span>
      <span class="logo-text">MaìLang Playground</span>
      <span class="version">v0.1.0</span>
    </div>
    <div class="actions">
      <button
        class="btn btn-run"
        :disabled="!isReady || isRunning"
        @click="$emit('run')"
      >
        <span v-if="isRunning" class="spinner">⟳</span>
        <span v-else>▶</span>
        {{ isRunning ? 'Running...' : 'Run' }}
      </button>
      <button class="btn btn-reset" @click="$emit('reset')">↺ Reset</button>
      <span class="status" :class="{ ready: isReady }">
        {{ isReady ? '● WASM Ready' : '○ Loading WASM...' }}
      </span>
    </div>
  </header>
</template>

<style scoped>
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 16px;
  background: #11111b;
  border-bottom: 1px solid #313244;
}

.logo {
  display: flex;
  align-items: center;
  gap: 8px;
}

.logo-icon {
  font-size: 24px;
  background: linear-gradient(135deg, #89b4fa, #cba6f7);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  font-weight: bold;
}

.logo-text {
  font-size: 16px;
  font-weight: 600;
  color: #cdd6f4;
}

.version {
  font-size: 11px;
  color: #6c7086;
  background: #313244;
  padding: 2px 6px;
  border-radius: 4px;
}

.actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.btn {
  padding: 6px 16px;
  border: none;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-run {
  background: #a6e3a1;
  color: #1e1e2e;
}

.btn-run:hover:not(:disabled) {
  background: #94e2d5;
}

.btn-run:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-reset {
  background: #313244;
  color: #cdd6f4;
}

.btn-reset:hover {
  background: #45475a;
}

.spinner {
  display: inline-block;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.status {
  font-size: 12px;
  color: #6c7086;
}

.status.ready {
  color: #a6e3a1;
}
</style>
