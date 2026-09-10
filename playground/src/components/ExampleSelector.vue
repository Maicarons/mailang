<script setup lang="ts">
defineProps<{
  examples: Array<{ name: string; description: string; code: string }>
}>()

defineEmits<{
  select: [example: { code: string }]
}>()
</script>

<template>
  <div class="examples">
    <div class="examples-header">📚 Examples</div>
    <div class="examples-list">
      <button
        v-for="(example, i) in examples"
        :key="i"
        class="example-item"
        @click="$emit('select', example)"
      >
        <span class="example-name">{{ example.name }}</span>
        <span class="example-desc">{{ example.description }}</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.examples {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.examples-header {
  padding: 10px 12px;
  font-size: 13px;
  font-weight: 600;
  color: #cba6f7;
  border-bottom: 1px solid #313244;
}

.examples-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px;
}

.example-item {
  display: flex;
  flex-direction: column;
  width: 100%;
  padding: 8px 10px;
  border: none;
  background: transparent;
  color: #cdd6f4;
  text-align: left;
  cursor: pointer;
  border-radius: 6px;
  transition: background 0.15s;
}

.example-item:hover {
  background: #313244;
}

.example-name {
  font-size: 13px;
  font-weight: 500;
}

.example-desc {
  font-size: 11px;
  color: #6c7086;
  margin-top: 2px;
}
</style>
