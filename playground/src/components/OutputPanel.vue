<script setup lang="ts">
defineProps<{
  output: string
  error: string
}>()
</script>

<template>
  <div class="output-container">
    <div class="output-header">
      <span class="output-title">📤 Output</span>
    </div>
    <div class="output-content">
      <pre v-if="output" class="output-text">{{ output }}</pre>
      <pre v-if="error" class="output-error">{{ error }}</pre>
      <div v-if="!output && !error" class="output-empty">
        <span class="empty-icon">▶</span>
        <span>Click "Run" or press Ctrl+Enter to execute</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.output-container {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.output-header {
  padding: 6px 12px;
  background: #181825;
  border-bottom: 1px solid #313244;
  font-size: 12px;
}

.output-title {
  color: #a6e3a1;
  font-weight: 500;
}

.output-content {
  flex: 1;
  padding: 12px;
  overflow: auto;
  background: #1e1e2e;
}

.output-text {
  color: #cdd6f4;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 14px;
  line-height: 1.6;
  white-space: pre-wrap;
  margin: 0;
}

.output-error {
  color: #f38ba8;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 14px;
  line-height: 1.6;
  white-space: pre-wrap;
  margin: 0;
}

.output-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #6c7086;
  gap: 8px;
}

.empty-icon {
  font-size: 32px;
  opacity: 0.3;
}
</style>
