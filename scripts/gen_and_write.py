import base64, pathlib, sys

content = """<script setup lang="ts">
import { useConvertStore } from "../stores/convert";
import { useLocalStore } from "../stores/local";

const convert = useConvertStore();
const local = useLocalStore();

function confirmConvert() {
  const files = local.selectedPaths;
  if (files.size === 0) return;
  convert.startConvert(Array.from(files));
}

function handleCancel() {
  convert.cancelConvert();
}

function handleDone() {
  convert.closeModal();
}
</script>

<template>
  <Transition name="modal">
    <div
      v-if="convert.showModal"
      class="fixed inset-0 z-40 flex items-center justify-center bg-black/20 backdrop-blur-sm"
      @click.self="handleCancel"
    >
      <div class="w-full max-w-md bg-apple-card rounded-2xl shadow-popover p-6 mx-4">
        <div class="flex items-center justify-between mb-5">
          <h2 class="text-base font-semibold text-apple-text">\u683c\u5f0f\u8f6c\u6362</h2>
          <button class="w-7 h-7 flex items-center justify-center rounded-full hover:bg-apple-divider transition-colors text-apple-secondary" @click="handleCancel">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
          </button>
        </div>

        <p class="text-sm text-apple-text mb-5">\u5df2\u9009 {{ local.selectedCount }} \u9996\u6b4c\u66f2</p>

        <div class="mb-5">
          <p class="text-xs text-apple-secondary font-medium mb-2">\u8f93\u51fa\u683c\u5f0f</p>
          <div class="flex gap-2">
            <button
              class="flex-1 py-2.5 rounded-xl text-sm font-medium transition-all border-2"
              :class="convert.format === 'mp3'
                ? 'border-apple-purple bg-apple-purple/5 text-apple-purple'
                : 'border-apple-divider text-apple-secondary hover:border-apple-gray'"
              @click="convert.format = 'mp3'"
            >MP3</button>
            <button
              class="flex-1 py-2.5 rounded-xl text-sm font-medium transition-all border-2"
              :class="convert.format === 'm4a'
                ? 'border-apple-purple bg-apple-purple/5 text-apple-purple'
                : 'border-apple-divider text-apple-secondary hover:border-apple-gray'"
              @click="convert.format = 'm4a'"
            >M4A</button>
          </div>
        </div>

        <div class="space-y-3 mb-5">
          <label class="flex items-center gap-3 cursor-pointer select-none">
            <input type="checkbox" v-model="convert.embedCover" class="sr-only peer" />
            <span class="w-5 h-5 rounded-md border-2 peer-checked:bg-apple-purple peer-checked:border-apple-purple flex items-center justify-center transition-all border-apple-gray">
              <svg v-if="convert.embedCover" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="3"><polyline points="20 6 9 17 4 12"/></svg>
            </span>
            <span class="text-sm text-apple-text">\u5d4c\u5165\u5c01\u9762</span>
          </label>
          <label class="flex items-center gap-3 cursor-pointer select-none">
            <input type="checkbox" v-model="convert.embedLyric" class="sr-only peer" />
            <span class="w-5 h-5 rounded-md border-2 peer-checked:bg-apple-purple peer-checked:border-apple-purple flex items-center justify-center transition-all border-apple-gray">
              <svg v-if="convert.embedLyric" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="3"><polyline points="20 6 9 17 4 12"/></svg>
            </span>
            <span class="text-sm text-apple-text">\u5d4c\u5165\u6b4c\u8bcd</span>
          </label>
        </div>

        <div class="mb-6">
          <p class="text-xs text-apple-secondary font-medium mb-2">\u8f93\u51fa\u76ee\u5f55</p>
          <div class="flex items-center gap-2">
            <code class="flex-1 text-xs text-apple-secondary bg-apple-bg px-3 py-2 rounded-lg truncate">{{ convert.outputPath }}</code>
            <button class="px-3 py-2 rounded-lg border border-apple-divider text-xs text-apple-secondary hover:bg-apple-bg transition-all">
              \u9009\u62e9\u76ee\u5f55
            </button>
          </div>
        </div>

        <div v-if="convert.tasks.length > 0" class="mb-3 max-h-40 overflow-y-auto space-y-2">
          <div
            v-for="task in convert.tasks"
            :key="task.id"
            class="flex items-center gap-3 px-2"
          >
            <svg class="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"
              :class="task.status === 'error' ? 'text-apple-red' : task.status === 'done' ? 'text-apple-green' : 'text-apple-secondary'"
            ><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="3"/></svg>
            <div class="flex-1 min-w-0">
              <p class="text-xs truncate"
                :class="task.status === 'error' ? 'text-apple-red' : task.status === 'done' ? 'text-apple-green' : 'text-apple-text'"
              >{{ task.name }}</p>
              <div v-if="task.status === 'converting'" class="flex items-center gap-2 mt-1">
                <span class="flex-1 h-1 bg-apple-divider rounded-full overflow-hidden">
                  <span class="block h-full bg-apple-purple rounded-full transition-all" :style="{ width: task.progress + '%' }"></span>
                </span>
                <span class="text-xs text-apple-secondary w-8 text-right">{{ task.progress }}%</span>
              </div>
            </div>
          </div>
        </div>

        <div v-if="convert.tasks.length > 0" class="mb-5 text-xs text-apple-secondary text-center">
          \u5171 {{ convert.totalCount }} \u9996 \u2014
          \u6210\u529f <span class="text-apple-green font-medium">{{ convert.successCount }}</span> \u9996\uff0c
          \u5931\u8d25 <span class="text-apple-red font-medium">{{ convert.errorCount }}</span> \u9996
        </div>

        <div class="flex gap-3">
          <button
            v-if="!convert.isDone"
            class="flex-1 py-2.5 rounded-xl border text-sm font-medium transition-all"
            :class="convert.isConverting
              ? 'border-apple-red text-apple-red hover:bg-apple-red/5'
              : 'border-apple-divider text-apple-secondary hover:bg-apple-bg'"
            @click="handleCancel"
          >
            {{ convert.isConverting ? '\u505c\u6b62' : '\u53d6\u6d88' }}
          </button>

          <button
            v-if="!convert.isDone"
            class="flex-1 py-2.5 rounded-xl text-sm font-medium transition-all"
            :class="convert.isConverting
              ? 'bg-apple-purple/40 text-white/60 cursor-not-allowed'
              : 'bg-apple-purple text-white hover:brightness-110'"
            :disabled="convert.isConverting"
            @click="confirmConvert"
          >
            {{ convert.isConverting ? '\u8f6c\u6362\u4e2d ' + convert.progressText : '\u5f00\u59cb\u8f6c\u6362' }}
          </button>

          <button
            v-if="convert.isDone"
            class="flex-1 py-2.5 rounded-xl bg-apple-purple text-white text-sm font-medium hover:brightness-110 transition-all"
            @click="handleDone"
          >
            \u5b8c\u6210
          </button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.modal-enter-active { transition: all 0.2s ease-out; }
.modal-leave-active { transition: all 0.15s ease-in; }
.modal-enter-from { opacity: 0; }
.modal-leave-to { opacity: 0; }
.modal-enter-active > div { animation: modalIn 0.2s ease-out; }
.modal-leave-active > div { animation: modalIn 0.15s ease-in reverse; }
@keyframes modalIn {
  from { opacity: 0; transform: scale(0.95) translateY(8px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}
</style>
"""

target = sys.argv[1]
pathlib.Path(target).write_text(content, encoding="utf-8")
print("OK")
