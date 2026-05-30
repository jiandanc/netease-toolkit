<script setup lang="ts">
import { useSettingsStore } from "../stores/settings";

const settings = useSettingsStore();

</script>

<template>
  <div class="max-w-2xl mx-auto space-y-6">
    <!-- Download Settings -->
    <section class="bg-apple-card rounded-xl shadow-card p-6">
      <h3 class="text-sm font-semibold text-apple-text mb-5">下载</h3>

      <div class="space-y-5">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm text-apple-text">默认下载路径</p>
            <p class="text-xs text-apple-secondary mt-0.5">下载的音频、封面和歌词将保存到此目录</p>
          </div>
          <div class="flex items-center gap-2">
            <code class="text-xs text-apple-secondary bg-apple-bg px-2.5 py-1.5 rounded-lg truncate max-w-[200px]">{{ settings.settings.downloadPath }}</code>
            <button class="px-3 py-1.5 rounded-lg border border-apple-divider text-xs text-apple-secondary hover:bg-apple-bg transition-all" @click="settings.selectDirectory('downloadPath')">
              选择目录
            </button>
          </div>
        </div>

        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm text-apple-text">下载时同时下载</p>
          </div>
          <div class="flex items-center gap-6">
            <label class="flex items-center gap-2 cursor-pointer select-none">
              <input type="checkbox" v-model="settings.settings.downloadCover" class="sr-only peer" />
              <span class="w-4 h-4 rounded border-2 border-apple-gray peer-checked:bg-apple-purple peer-checked:border-apple-purple flex items-center justify-center transition-all">
                <svg v-if="settings.settings.downloadCover" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="3"><polyline points="20 6 9 17 4 12"/></svg>
              </span>
              <span class="text-sm text-apple-text">封面</span>
            </label>
            <label class="flex items-center gap-2 cursor-pointer select-none">
              <input type="checkbox" v-model="settings.settings.downloadLyric" class="sr-only peer" />
              <span class="w-4 h-4 rounded border-2 border-apple-gray peer-checked:bg-apple-purple peer-checked:border-apple-purple flex items-center justify-center transition-all">
                <svg v-if="settings.settings.downloadLyric" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="3"><polyline points="20 6 9 17 4 12"/></svg>
              </span>
              <span class="text-sm text-apple-text">歌词</span>
            </label>
          </div>
        </div>

        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm text-apple-text">降音质重试</p>
            <p class="text-xs text-apple-secondary mt-0.5">高音质不可用时自动尝试较低音质</p>
          </div>
          <label class="flex items-center gap-2 cursor-pointer select-none">
            <input type="checkbox" v-model="settings.settings.qualityFallback" class="sr-only peer" />
            <span class="w-4 h-4 rounded border-2 border-apple-gray peer-checked:bg-apple-purple peer-checked:border-apple-purple flex items-center justify-center transition-all">
              <svg v-if="settings.settings.qualityFallback" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="3"><polyline points="20 6 9 17 4 12"/></svg>
            </span>
            <span class="text-sm text-apple-text">开启</span>
          </label>
        </div>

        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm text-apple-text">最大并发下载数</p>
            <p class="text-xs text-apple-secondary mt-0.5">同时下载的歌曲数量上限</p>
          </div>
          <div class="flex items-center gap-2">
            <button
              class="w-7 h-7 rounded-lg border border-apple-divider text-apple-secondary hover:bg-apple-bg transition-all flex items-center justify-center text-sm disabled:opacity-40 disabled:cursor-not-allowed"
              :disabled="settings.settings.maxConcurrentDownloads <= 1"
              @click="settings.settings.maxConcurrentDownloads = Math.max(1, settings.settings.maxConcurrentDownloads - 1)"
            >-</button>
            <span class="w-8 text-center text-sm font-medium text-apple-text">{{ settings.settings.maxConcurrentDownloads }}</span>
            <button
              class="w-7 h-7 rounded-lg border border-apple-divider text-apple-secondary hover:bg-apple-bg transition-all flex items-center justify-center text-sm disabled:opacity-40 disabled:cursor-not-allowed"
              :disabled="settings.settings.maxConcurrentDownloads >= 5"
              @click="settings.settings.maxConcurrentDownloads = Math.min(5, settings.settings.maxConcurrentDownloads + 1)"
            >+</button>
          </div>
        </div>
      </div>
    </section>

    <!-- Convert Settings -->
    <section class="bg-apple-card rounded-xl shadow-card p-6">
      <h3 class="text-sm font-semibold text-apple-text mb-5">转换</h3>

      <div class="space-y-5">
        <div class="flex items-center justify-between">
          <p class="text-sm text-apple-text">默认转换格式</p>
          <select
            v-model="settings.settings.convertFormat"
            class="text-xs border border-apple-divider rounded-lg px-3 py-1.5 bg-white text-apple-text outline-none focus:ring-2 focus:ring-apple-purple/20 focus:border-apple-purple"
          >
            <option value="mp3">MP3</option>
            <option value="m4a">M4A</option>
          </select>
        </div>

        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm text-apple-text">默认输出目录</p>
          </div>
          <div class="flex items-center gap-2">
            <code class="text-xs text-apple-secondary bg-apple-bg px-2.5 py-1.5 rounded-lg truncate max-w-[200px]">{{ settings.settings.convertOutputPath }}</code>
            <button class="px-3 py-1.5 rounded-lg border border-apple-divider text-xs text-apple-secondary hover:bg-apple-bg transition-all" @click="settings.selectDirectory('convertOutputPath')">
              选择目录
            </button>

          </div>
        </div>

        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm text-apple-text">最大并行转换数</p>
            <p class="text-xs text-apple-secondary mt-0.5">同时转换的文件数量上限</p>
          </div>
          <div class="flex items-center gap-2">
            <button
              class="w-7 h-7 rounded-lg border border-apple-divider text-apple-secondary hover:bg-apple-bg transition-all flex items-center justify-center text-sm disabled:opacity-40 disabled:cursor-not-allowed"
              :disabled="settings.settings.maxConcurrentConverts <= 1"
              @click="settings.settings.maxConcurrentConverts = Math.max(1, settings.settings.maxConcurrentConverts - 1)"
            >-</button>
            <span class="w-8 text-center text-sm font-medium text-apple-text">{{ settings.settings.maxConcurrentConverts }}</span>
            <button
              class="w-7 h-7 rounded-lg border border-apple-divider text-apple-secondary hover:bg-apple-bg transition-all flex items-center justify-center text-sm disabled:opacity-40 disabled:cursor-not-allowed"
              :disabled="settings.settings.maxConcurrentConverts >= 5"
              @click="settings.settings.maxConcurrentConverts = Math.min(5, settings.settings.maxConcurrentConverts + 1)"
            >+</button>
          </div>
        </div>
      </div>
    </section>

    <!-- About -->
    <section class="bg-apple-card rounded-xl shadow-card p-6">
      <h3 class="text-sm font-semibold text-apple-text mb-3">关于</h3>
      <p class="text-sm text-apple-text">NetEase Toolkit v1.0</p>
      <p class="text-xs text-apple-secondary mt-1">基于 Tauri + Vue 3 构建</p>
    </section>

    <!-- Save hint -->
    <div class="flex justify-end">
      <button
        class="px-6 py-2 rounded-xl bg-apple-purple text-white text-sm font-medium hover:brightness-110 transition-all"
        @click="settings.saveSettings()"
      >
        保存设置
      </button>
    </div>
  </div>
</template>
