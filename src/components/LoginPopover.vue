<script setup lang="ts">
import { useAuthStore } from "../stores/auth";
import { ref, onMounted, onUnmounted } from "vue";

const auth = useAuthStore();
const popoverRef = ref<HTMLElement | null>(null);
const headerHeight = ref(56);

onMounted(() => {
  const header = document.querySelector("header");
  if (header) {
    headerHeight.value = header.offsetHeight;
  }
  document.addEventListener("click", handleClickOutside, true);
});

onUnmounted(() => {
  document.removeEventListener("click", handleClickOutside, true);
});

function handleClickOutside(e: MouseEvent) {
  if (!auth.showLoginPopover) return;
  if (popoverRef.value && !popoverRef.value.contains(e.target as Node)) {
    auth.closeLoginPopover();
  }
}
</script>

<template>
  <Transition name="fade">
    <div
      v-if="auth.showLoginPopover"
      class="fixed left-0 right-0 bottom-0 z-30 bg-white"
      :style="{ top: headerHeight + 'px' }"
    >
      <div
        ref="popoverRef"
        class="absolute top-3 right-6 w-72 bg-apple-card rounded-2xl shadow-popover border border-apple-divider p-6"
      >
        <!-- Logged-in state -->
        <template v-if="auth.isLoggedIn">
          <div class="flex items-center justify-between mb-4">
            <h2 class="text-base font-semibold text-apple-text">个人</h2>
            <button
              class="w-7 h-7 flex items-center justify-center rounded-full hover:bg-apple-divider transition-colors text-apple-secondary"
              @click="auth.closeLoginPopover"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
            </button>
          </div>

          <div class="w-16 h-16 mx-auto mb-3 rounded-full overflow-hidden bg-apple-purple flex items-center justify-center">
            <img v-if="auth.userAvatar" :src="auth.userAvatar" class="w-full h-full object-cover" alt="avatar" />
            <span v-else class="text-white text-xl font-medium">{{ auth.userName?.charAt(0) || 'U' }}</span>
          </div>

          <p class="text-sm text-center font-medium text-apple-text mb-6">{{ auth.userName || '已登录' }}</p>

          <button
            class="w-full py-2.5 rounded-xl border border-apple-red text-apple-red text-sm font-medium hover:bg-apple-red/5 transition-all"
            @click="auth.logout(); auth.closeLoginPopover()"
          >
            退出登录
          </button>
        </template>

        <!-- Not logged-in state -->
        <template v-else>
          <div class="flex items-center justify-between mb-4">
            <h2 class="text-base font-semibold">登录</h2>
            <button
              class="w-7 h-7 flex items-center justify-center rounded-full hover:bg-apple-divider transition-colors text-apple-secondary"
              @click="auth.closeLoginPopover"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
            </button>
          </div>

          <div class="w-48 h-48 mx-auto mb-4 bg-white rounded-xl flex items-center justify-center overflow-hidden">
            <img
              v-if="auth.qrCodeBase64"
              :src="`data:image/png;base64,${auth.qrCodeBase64}`"
              class="w-full h-full object-contain"
              alt="QR Code"
            />
            <div v-else class="flex flex-col items-center gap-3 text-apple-secondary">
              <svg class="animate-spin" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12a9 9 0 1 1-6.219-8.56"/></svg>
              <span class="text-xs">生成二维码中...</span>
            </div>
          </div>

          <p class="text-xs text-apple-secondary text-center mb-3">
            请使用网易云音乐 APP 扫码登录
          </p>
          <p class="text-sm text-center font-medium mb-4" :class="{
            'text-apple-secondary': auth.loginStatus === 'idle' || auth.loginStatus === 'waiting',
            'text-apple-blue': auth.loginStatus === 'scanned',
            'text-apple-green': auth.loginStatus === 'confirmed',
            'text-apple-red': auth.loginStatus === 'expired',
          }">
            <template v-if="auth.loginStatus === 'scanned'">扫码成功，请在手机上确认</template>
            <template v-else-if="auth.loginStatus === 'confirmed'">登录成功！</template>
            <template v-else-if="auth.loginStatus === 'expired'">二维码已过期，请刷新</template>
            <template v-else>等待扫码...</template>
          </p>

          <button
            class="w-full py-2.5 rounded-xl bg-apple-red text-white text-sm font-medium hover:brightness-105 transition-all"
            @click="auth.generateQr()"
          >
            刷新二维码
          </button>
        </template>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.fade-enter-active { transition: opacity 0.2s ease-out; }
.fade-leave-active { transition: opacity 0.15s ease-in; }
.fade-enter-from,
.fade-leave-to { opacity: 0; }
</style>
