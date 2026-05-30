import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export const useAuthStore = defineStore("auth", () => {
  const isLoggedIn = ref(false);
  const userAvatar = ref("");
  const userName = ref("");
  const showLoginPopover = ref(false);
  const qrCodeBase64 = ref("");
  const loginStatus = ref<"idle" | "waiting" | "scanned" | "confirmed" | "expired">("idle");
  const unikey = ref("");

  let pollTimer: ReturnType<typeof setInterval> | null = null;

  async function generateQr() {
    loginStatus.value = "waiting";
    qrCodeBase64.value = "";
    try {
      const result = await invoke<{ unikey: string; qr_image_base64: string }>("cmd_qr_login_generate");
      unikey.value = result.unikey;
      qrCodeBase64.value = result.qr_image_base64;
      // Start polling for login status
      startPolling();
    } catch (e) {
      loginStatus.value = "expired";
      console.error("QR generate failed:", e);
    }
  }

  function startPolling() {
    stopPolling();
    pollTimer = setInterval(async () => {
      await checkLogin();
    }, 2000);
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  }

  async function checkLogin() {
    if (!unikey.value) return;
    try {
      const result = await invoke<{ code: number; cookie?: string }>("cmd_qr_login_check", {
        unikey: unikey.value,
      });
      const code = result.code;
      if (code === 800) {
        loginStatus.value = "expired";
        stopPolling();
      } else if (code === 801) {
        loginStatus.value = "waiting";
      } else if (code === 802) {
        loginStatus.value = "scanned";
      } else if (code === 803) {
        loginStatus.value = "confirmed";
        isLoggedIn.value = true;
        stopPolling();
        // Auto-close popover after a moment
        setTimeout(() => closeLoginPopover(), 1000);
      }
    } catch (e) {
      console.error("QR check failed:", e);
    }
  }

  async function logout() {
    try {
      await invoke("cmd_logout");
    } catch (e) {
      console.error("Logout failed:", e);
    }
    isLoggedIn.value = false;
    userAvatar.value = "";
    userName.value = "";
    stopPolling();
  }

  async function checkLoginStatus() {
    try {
      const status = await invoke<{ isLoggedIn: boolean; userAvatar: string; userName: string }>("cmd_get_login_status");
      isLoggedIn.value = status.isLoggedIn;
      if (status.userAvatar) userAvatar.value = status.userAvatar;
      if (status.userName) userName.value = status.userName;
    } catch (e) {
      console.error("Check login status failed:", e);
    }
  }

  function openLoginPopover() {
    showLoginPopover.value = true;
    // Only generate QR code if not already logged in
    if (!isLoggedIn.value && !qrCodeBase64.value) {
      generateQr();
    }
  }

  function closeLoginPopover() {
    showLoginPopover.value = false;
  }

  return {
    isLoggedIn, userAvatar, userName,
    showLoginPopover, qrCodeBase64, loginStatus, unikey,
    generateQr, checkLogin, logout, checkLoginStatus,
    openLoginPopover, closeLoginPopover,
  };
});
