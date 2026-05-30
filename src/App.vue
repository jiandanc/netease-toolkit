<script setup lang="ts">
import AppHeader from "./components/AppHeader.vue";
import LoginPopover from "./components/LoginPopover.vue";
import Toast from "./components/Toast.vue";
import { onMounted } from "vue";
import { useSettingsStore } from "./stores/settings";
import { useAuthStore } from "./stores/auth";

const settings = useSettingsStore();
const auth = useAuthStore();

onMounted(() => {
  settings.loadSettings();
  // Restore login state from cookie persisted on disk
  auth.checkLoginStatus();
});

</script>

<template>
  <AppHeader />
  <main class="flex-1 overflow-auto px-8 py-6">
    <router-view />
  </main>
  <LoginPopover />
  <Toast />
</template>
