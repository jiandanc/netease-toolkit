<script setup lang="ts">
import { ref } from "vue";
import { useDownloadStore } from "../stores/download";
import { useSettingsStore } from "../stores/settings";

const download = useDownloadStore();
const settings = useSettingsStore();
const searchInput = ref("");

const qualityOptions = [
  { value: "standard", label: "标准" },
  { value: "exhigh", label: "极高" },
  { value: "lossless", label: "无损" },
  { value: "hires", label: "Hi-Res" },
  { value: "sky", label: "环绕声" },
  { value: "jyeffect", label: "高清环绕" },
  { value: "jymaster", label: "超清母带" },
];

function doSearch() {
  download.search(searchInput.value);
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "Enter") doSearch();
}
</script>

<template>
  <div class="max-w-4xl mx-auto space-y-6">
    <!-- Search Bar -->
    <div class="bg-apple-card rounded-xl shadow-card p-5 space-y-4">
      <div class="flex items-center gap-2">
        <span class="text-xs text-apple-secondary font-medium">音质</span>
        <select
          v-model="download.searchQuality"
          class="text-xs border border-apple-divider rounded-lg px-2.5 py-1.5 bg-white text-apple-text outline-none focus:ring-2 focus:ring-apple-purple/20 focus:border-apple-purple"
        >
          <option v-for="q in qualityOptions" :key="q.value" :value="q.value">
            {{ q.label }}
          </option>
        </select>
      </div>

      <div class="flex gap-2">
        <input
          v-model="searchInput"
          type="text"
          placeholder="输入歌曲名、歌手名、歌单链接或专辑链接..."
          class="flex-1 px-4 py-2.5 rounded-xl border border-apple-divider bg-white text-sm text-apple-text outline-none placeholder:text-apple-secondary/60 focus:ring-2 focus:ring-apple-purple/20 focus:border-apple-purple transition-all"
          @keydown="handleKeydown"
        />
        <button
          class="px-6 py-2.5 rounded-xl bg-apple-purple text-white text-sm font-medium hover:brightness-110 transition-all disabled:opacity-50 disabled:cursor-not-allowed shrink-0"
          :disabled="download.isSearching || !searchInput.trim()"
          @click="doSearch"
        >
          <span v-if="download.isSearching" class="flex items-center gap-2">
            <svg class="animate-spin" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M21 12a9 9 0 1 1-6.219-8.56"/></svg>
            搜索中
          </span>
          <span v-else>搜索</span>
        </button>
      </div>
    </div>

    <!-- Search Results -->
    <div v-if="download.searchResults.length > 0" class="bg-apple-card rounded-xl shadow-card">
      <div class="flex items-center justify-between px-5 h-11 border-b border-apple-divider">
        <div class="flex items-center gap-3">
          <button
            class="w-5 h-5 rounded-md border-2 flex items-center justify-center transition-all"
            :class="download.allSelected
              ? 'bg-apple-purple border-apple-purple text-white'
              : 'border-apple-gray hover:border-apple-purple'"
            @click="download.toggleSelectAll()"
          >
            <svg :class="{ invisible: !download.allSelected }" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
          </button>
          <span class="text-sm text-apple-text">搜索结果</span>
          <span :class="{ invisible: download.selectedCount <= 0 }" class="text-xs text-apple-purple font-medium">
            已选 {{ download.selectedCount }} 首
          </span>
        </div>
        <button
          :class="{ invisible: download.selectedCount <= 0 }"
          class="px-4 py-1.5 rounded-lg bg-apple-red text-white text-xs font-medium hover:brightness-110 transition-all"
          @click="download.startDownload()"
        >
          下载
        </button>
      </div>

      <div class="divide-y divide-apple-divider">
        <div
          v-for="song in download.searchResults"
          :key="song.id"
          class="flex items-center px-5 h-12 hover:bg-apple-bg/50 transition-colors cursor-pointer"
          @click="download.toggleSelect(song.id)"
        >
          <button
            class="w-5 h-5 rounded-md border-2 flex items-center justify-center transition-all shrink-0 mr-4"
            :class="download.selectedIds.has(song.id)
              ? 'bg-apple-purple border-apple-purple text-white'
              : 'border-apple-gray hover:border-apple-purple'"
            @click.stop="download.toggleSelect(song.id)"
          >
            <svg :class="{ invisible: !download.selectedIds.has(song.id) }" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
          </button>
          <svg class="w-5 h-5 text-apple-secondary shrink-0 mr-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="3"/></svg>
          <div class="flex-1 min-w-0">
            <p class="text-sm font-medium text-apple-text truncate">{{ song.name }}</p>
            <p class="text-xs text-apple-secondary truncate">{{ song.artists }} &middot; {{ song.album }}</p>
          </div>
          <div class="flex items-center gap-2 text-xs text-apple-secondary shrink-0 ml-4">
            <span v-if="song.fee === 1" class="px-1.5 py-0.5 rounded bg-apple-red/10 text-apple-red text-[10px] font-semibold">VIP</span>
            <span v-else-if="song.fee === 4" class="px-1.5 py-0.5 rounded bg-apple-blue/10 text-apple-blue text-[10px] font-semibold">付费</span>

          </div>
        </div>
      </div>
    </div>

    <!-- Empty state -->
    <div
      v-if="download.searchResults.length === 0 && !download.isSearching"
      class="flex flex-col items-center justify-center py-16 text-apple-secondary"
    >
      <svg class="w-12 h-12 mb-3 opacity-30" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>
      <p class="text-sm">搜索歌曲开始下载</p>
    </div>
  </div>
</template>
