<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { useLocalStore } from "../stores/local";
import { useDownloadStore } from "../stores/download";
import { useConvertStore } from "../stores/convert";
import { useSettingsStore } from "../stores/settings";
import ConvertModal from "../components/ConvertModal.vue";

const local = useLocalStore();
const download = useDownloadStore();
const convert = useConvertStore();
const settings = useSettingsStore();

const qualityLabelMap: Record<string, string> = {
  "jymaster": "超清母带",
  "jyeffect": "高清环绕",
  "sky": "环绕声",
  "hires": "Hi-Res",
  "lossless": "无损",
  "exhigh": "极高",
  "standard": "标准",
};

let syncInterval: ReturnType<typeof setInterval>;

onMounted(() => {
  local.scanLocal();
  syncInterval = setInterval(() => {
    local.syncDownloading();
  }, 2000);
});

onUnmounted(() => {
  clearInterval(syncInterval);
});

function handleConvert() {
  convert.outputPath = settings.settings.convertOutputPath;
  convert.openModal();
}

async function retryDownload(item: { id: number; name: string; artists: string }) {
  await download.retryDownload(item.id, settings.settings.downloadQuality, item.name, item.artists);
  local.syncDownloading();
}
</script>

<template>
  <div class="max-w-4xl mx-auto space-y-4">
    <!-- Toolbar -->
    <div class="bg-apple-card rounded-xl shadow-card p-4">
      <div class="flex items-center justify-between gap-4">
        <div class="flex items-center gap-3">
          <button
            class="w-5 h-5 rounded-md border-2 flex items-center justify-center transition-all"
            :class="local.allSelected
              ? 'bg-apple-purple border-apple-purple text-white'
              : 'border-apple-gray hover:border-apple-purple'"
            @click="local.toggleSelectAll()"
          >
            <svg :class="{ invisible: !local.allSelected }" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
          </button>
          <span class="text-sm text-apple-text font-medium">本地 ({{ local.songs.length }})</span>
          <span :class="{ invisible: local.selectedCount <= 0 }" class="text-xs text-apple-purple font-medium">已选 {{ local.selectedCount }} 首</span>
        </div>

        <div class="flex items-center gap-2">
          <div class="relative">
            <svg class="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-apple-secondary" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>
            <input
              v-model="local.searchQuery"
              type="text"
              placeholder="搜索本地..."
              class="w-40 pl-8 pr-3 py-1.5 rounded-lg border border-apple-divider bg-white text-xs text-apple-text outline-none placeholder:text-apple-secondary/60 focus:ring-2 focus:ring-apple-purple/20 focus:border-apple-purple transition-all"
            />
          </div>
          <button
            :class="{ invisible: local.selectedCount <= 0 }"
            class="px-3 py-1.5 rounded-lg bg-apple-purple text-white text-xs font-medium hover:brightness-110 transition-all"
            @click="handleConvert"
          >
            格式转换
          </button>
          <button
            :class="{ invisible: local.selectedCount <= 0 }"
            class="px-3 py-1.5 rounded-lg border border-apple-red text-apple-red text-xs font-medium hover:bg-apple-red/5 transition-all"
            @click="local.deleteSelected()"
          >
            删除
          </button>
          <button
            class="px-3 py-1.5 rounded-lg border border-apple-divider text-apple-secondary text-xs font-medium hover:bg-apple-bg transition-all"
            @click="local.openFolder()"
          >
            打开文件夹
          </button>
        </div>
      </div>
    </div>

    <!-- Downloading items -->
    <div
      v-if="local.downloadingItems.length > 0"
      class="bg-apple-card rounded-xl shadow-card divide-y divide-apple-divider"
    >
      <div class="px-5 py-2 border-b border-apple-divider">
        <span class="text-xs text-apple-secondary font-medium">下载中</span>
      </div>
      <div
        v-for="item in local.downloadingItems"
        :key="item.id"
        class="flex items-center px-5 h-12"
        :class="item.status === 'error' ? 'bg-apple-red/5' : ''"
      >
        <svg class="w-5 h-5 shrink-0 mr-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"
          :class="item.status === 'error' ? 'text-apple-red' : 'text-apple-secondary'"
        ><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="3"/></svg>

        <div class="flex-1 min-w-0">
          <p class="text-sm truncate"
            :class="item.status === 'error' ? 'text-apple-red' : 'text-apple-text'"
          >
            {{ item.name }}
            <span :class="item.status === 'error' ? 'text-apple-red/70' : 'text-apple-secondary'" class="font-normal">- {{ item.artists }}</span>
            <span v-if="item.status === 'downloading'" class="inline-flex items-center ml-2 align-middle">
              <span class="w-20 h-1 bg-apple-divider rounded-full overflow-hidden inline-block align-middle">
                <span class="block h-full bg-apple-purple rounded-full transition-all" :style="{ width: item.progress + '%' }"></span>
              </span>
              <span class="text-xs text-apple-secondary ml-1.5 align-middle">{{ item.progress }}%</span>
              <span v-if="item.speed" class="text-xs text-apple-secondary ml-1 align-middle">&middot; {{ item.speed }}</span>
            </span>
            <span v-else-if="item.status === 'error'" class="text-xs text-apple-red ml-2 font-medium truncate max-w-[200px]">{{ item.error || '下载失败' }}</span>
          </p>
        </div>
        <button
          v-if="item.status === 'error'"
          class="px-3 py-1 rounded-lg border border-apple-red text-apple-red text-xs font-medium hover:bg-apple-red/10 transition-all shrink-0 ml-3"
          @click="retryDownload(item)"
        >
          重试
        </button>
      </div>
    </div>

    <!-- Song List -->
    <div v-if="local.filteredSongs.length > 0" class="bg-apple-card rounded-xl shadow-card divide-y divide-apple-divider">
      <div class="px-5 py-2 border-b border-apple-divider">
        <span class="text-xs text-apple-secondary font-medium">已下载</span>
      </div>
      <div
        v-for="song in local.filteredSongs"
        :key="song.filePath"
        class="flex items-center px-5 h-12 hover:bg-apple-bg/50 transition-colors cursor-pointer"
        @click="local.toggleSelect(song.filePath)"
      >
        <button
          class="w-5 h-5 rounded-md border-2 flex items-center justify-center transition-all shrink-0 mr-4"
          :class="local.selectedPaths.has(song.filePath)
            ? 'bg-apple-purple border-apple-purple text-white'
            : 'border-apple-gray hover:border-apple-purple'"
          @click.stop="local.toggleSelect(song.filePath)"
        >
          <svg :class="{ invisible: !local.selectedPaths.has(song.filePath) }" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
        </button>

        <svg class="w-5 h-5 text-apple-secondary shrink-0 mr-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="3"/></svg>

        <div class="flex-1 min-w-0">
          <p class="text-sm font-medium text-apple-text truncate">{{ song.name }} <span class="text-apple-secondary font-normal">- {{ song.artists }}</span></p>
          <p class="text-xs text-apple-secondary">{{ song.format.toUpperCase() }} &middot; {{ song.fileSize }} &middot; {{ qualityLabelMap[song.quality] || song.quality }}</p>
        </div>

        <div class="flex items-center gap-4 shrink-0 ml-4">
          <div class="flex items-center gap-1.5">
            <span class="text-xs">封面</span>
            <span :class="song.hasCover ? 'text-apple-green' : 'text-apple-gray'" class="font-bold text-sm">{{ song.hasCover ? '✓' : '✗' }}</span>
          </div>
          <div class="flex items-center gap-1.5">
            <span class="text-xs">歌词</span>
            <span :class="song.hasLyric ? 'text-apple-green' : 'text-apple-gray'" class="font-bold text-sm">{{ song.hasLyric ? '✓' : '✗' }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Empty state -->
    <div
      v-if="local.filteredSongs.length === 0 && local.downloadingItems.length === 0"
      class="flex flex-col items-center justify-center py-16 text-apple-secondary"
    >
      <svg class="w-12 h-12 mb-3 opacity-30" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>
      <p class="text-sm">还没有下载歌曲</p>
      <p class="text-xs mt-1">切换到「下载」页搜索音乐吧</p>
    </div>

    <ConvertModal />
  </div>
</template>
