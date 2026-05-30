<script setup lang="ts">
import { ref } from "vue";

interface ToastMessage {
  id: number;
  text: string;
  type: "success" | "error" | "info";
}

const toasts = ref<ToastMessage[]>([]);
let nextId = 0;

function addToast(text: string, type: "success" | "error" | "info" = "info") {
  const id = nextId++;
  toasts.value.push({ id, text, type });
  setTimeout(() => {
    toasts.value = toasts.value.filter((t) => t.id !== id);
  }, 3000);
}

defineExpose({ addToast });
</script>

<template>
  <div class="fixed bottom-6 left-1/2 -translate-x-1/2 z-50 flex flex-col gap-2 pointer-events-none">
    <TransitionGroup name="toast">
      <div
        v-for="toast in toasts"
        :key="toast.id"
        class="pointer-events-auto px-5 py-3 rounded-xl shadow-popover text-sm font-medium transition-all duration-300"
        :class="{
          'bg-apple-green text-white': toast.type === 'success',
          'bg-apple-red text-white': toast.type === 'error',
          'bg-apple-text text-white': toast.type === 'info',
        }"
      >
        {{ toast.text }}
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.toast-enter-active { animation: slideUp 0.3s ease-out; }
.toast-leave-active { animation: slideUp 0.2s ease-in reverse; }
@keyframes slideUp {
  from { opacity: 0; transform: translateY(12px); }
  to { opacity: 1; transform: translateY(0); }
}
</style>
