import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export interface AppSettings {
  downloadPath: string;
  downloadQuality: string;
  downloadCover: boolean;
  downloadLyric: boolean;
  convertFormat: string;
  convertOutputPath: string;
  qualityFallback: boolean;
  maxConcurrentDownloads: number;
  maxConcurrentConverts: number;
}

export const useSettingsStore = defineStore("settings", () => {
  const settings = ref<AppSettings>({
    downloadPath: "",
    downloadQuality: "exhigh",
    downloadCover: true,
    downloadLyric: true,
    convertFormat: "m4a",
    convertOutputPath: "",
    qualityFallback: true,
    maxConcurrentDownloads: 5,
    maxConcurrentConverts: 5,
  });

  async function loadSettings() {
    try {
      const result = await invoke<AppSettings>("cmd_load_settings");
      settings.value = { ...settings.value, ...result };
    } catch (e) {
      console.error("Load settings failed:", e);
    }
  }

  async function saveSettings() {
    try {
      await invoke("cmd_save_settings", { settings: settings.value });
    } catch (e) {
      console.error("Save settings failed:", e);
    }
  }

  async function selectDirectory(target: "downloadPath" | "convertOutputPath") {
    try {
      const currentPath = settings.value[target];
      const selected = await open({
        directory: true,
        multiple: false,
        title: "选择目录",
        defaultPath: currentPath || undefined,
      });
      if (selected) {
        settings.value[target] = selected;
      }
    } catch (e) {
      console.error("Select directory failed:", e);
    }
  }

  return { settings, loadSettings, saveSettings, selectDirectory };
});
