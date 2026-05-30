<script setup lang="ts">
import { useRoute, useRouter } from "vue-router";
import { useAuthStore } from "../stores/auth";
import { useLocalStore } from "../stores/local";

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const local = useLocalStore();

function handleTabClick(name: string) {
  router.push({ name });
  if (name === "local") {
    local.scanLocal();
  }
}

const tabs = [
  { name: "download", label: "下载" },
  { name: "local", label: "本地" },
  { name: "settings", label: "设置" },
];
</script>

<template>
  <header class="relative flex items-center justify-center h-14 px-6 bg-apple-card/80 backdrop-blur-lg border-b border-apple-divider shrink-0">
    <nav class="flex items-center gap-1 bg-apple-bg rounded-xl p-0.5">
      <button
        v-for="tab in tabs"
        :key="tab.name"
        class="relative px-5 py-1.5 rounded-[10px] text-sm font-medium transition-all"
        :class="route.name === tab.name
          ? 'bg-white text-apple-text shadow-sm'
          : 'text-apple-secondary hover:text-apple-text'"
        @click="handleTabClick(tab.name)"
      >
        {{ tab.label }}
        <!-- Badge on 本地 tab -->
        <span
          v-if="tab.name === 'local' && local.downloadingCount > 0"
          class="absolute -top-1.5 -right-1.5 min-w-[18px] h-[18px] flex items-center justify-center rounded-full bg-apple-red text-white text-[10px] font-bold leading-none px-1"
        >
          {{ local.downloadingCount }}
        </span>
      </button>
    </nav>

    <div class="absolute right-6 top-1/2 -translate-y-1/2">
      <button
        v-if="!auth.isLoggedIn"
        class="w-9 h-9 rounded-full bg-apple-red flex items-center justify-center text-white text-xs font-semibold transition-colors shrink-0"
        @click="auth.openLoginPopover()"
      >
        登录
      </button>
      <button
        v-else
        class="w-9 h-9 rounded-full overflow-hidden border-2 border-apple-divider hover:border-apple-purple transition-colors shrink-0"
        @click="auth.openLoginPopover()"
      >
        <img v-if="auth.userAvatar" :src="auth.userAvatar" class="w-full h-full object-cover" alt="avatar" />
        <span v-else class="w-full h-full flex items-center justify-center bg-apple-purple text-white text-xs font-medium">
          {{ auth.userName?.charAt(0) || "U" }}
        </span>
      </button>
    </div>
  </header>
</template>
