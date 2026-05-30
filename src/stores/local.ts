import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "./settings";
import { useDownloadStore } from "./download";

export interface LocalSong {
  name: string;
  artists: string;
  format: string;
  fileSize: string;
  filePath: string;
  hasCover: boolean;
  hasLyric: boolean;
  quality: string;
}

export interface DownloadingItem {
  id: number;
  name: string;
  artists: string;
  progress: number;
  speed?: string;
  status: "downloading" | "done" | "error";
  error?: string;
  attemptedQuality?: string;
}

export const useLocalStore = defineStore("local", () => {
  const songs = ref<LocalSong[]>([]);
  const selectedPaths = ref<Set<string>>(new Set());
  const searchQuery = ref("");
  const isLoading = ref(false);

  /** Synced from download store — items currently being downloaded */
  const downloadingItems = ref<DownloadingItem[]>([]);

  const downloadingCount = computed(() =>
    downloadingItems.value.filter((d) => d.status === "downloading").length
  );

  const filteredSongs = computed(() => {
    if (!searchQuery.value.trim()) return songs.value;
    const q = searchQuery.value.toLowerCase();
    return songs.value.filter(
      (s) => s.name.toLowerCase().includes(q) || s.artists.toLowerCase().includes(q)
    );
  });

  const selectedCount = computed(() => selectedPaths.value.size);
  const allSelected = computed(() =>
    filteredSongs.value.length > 0 && selectedPaths.value.size === filteredSongs.value.length
  );

  function toggleSelect(path: string) {
    const newSet = new Set(selectedPaths.value);
    if (newSet.has(path)) {
      newSet.delete(path);
    } else {
      newSet.add(path);
    }
    selectedPaths.value = newSet;
  }

  function toggleSelectAll() {
    if (allSelected.value) {
      selectedPaths.value = new Set();
    } else {
      selectedPaths.value = new Set(filteredSongs.value.map((s) => s.filePath));
    }
  }

  async function scanLocal() {
    isLoading.value = true;
    const settings = useSettingsStore();
    try {
      const results = await invoke<LocalSong[]>("cmd_scan_local_dir", {
        dir: settings.settings.downloadPath,
      });
      songs.value = results;
    } catch (e) {
      console.error("Scan local failed:", e);
      songs.value = [];
    }
    isLoading.value = false;
  }

  /** Sync downloading items from download store */
  function syncDownloading() {
    const ds = useDownloadStore();
    downloadingItems.value = ds.downloadQueue
      .filter((t) => t.status !== "done")
      .map((t) => ({
        id: t.id,
        name: t.name,
        artists: t.artists,
        progress: t.progress,
        speed: t.speed,
        status: t.status === "waiting" || t.status === "downloading" ? "downloading" : t.status,
        error: t.error,
      }));
  }

  async function deleteSelected() {
    const paths = Array.from(selectedPaths.value);
    try {
      await invoke("cmd_delete_files", { paths });
    } catch (e) {
      console.error("Delete failed:", e);
    }
    selectedPaths.value = new Set();
    await scanLocal();
  }

  async function openFolder() {
    const settings = useSettingsStore();
    try {
      await invoke("cmd_open_in_finder", { path: settings.settings.downloadPath });
    } catch (e) {
      console.error("Open folder failed:", e);
    }
  }

  /** Remove a done download from the queue (called after scanLocal refresh) */
  function clearDoneItem(id: number) {
    downloadingItems.value = downloadingItems.value.filter((d) => d.id !== id);
  }

  return {
    songs, selectedPaths, searchQuery, isLoading,
    downloadingItems, downloadingCount,
    filteredSongs, selectedCount, allSelected,
    toggleSelect, toggleSelectAll, scanLocal,
    syncDownloading, deleteSelected, openFolder, clearDoneItem,
  };
});
