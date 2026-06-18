import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useSettingsStore } from "./settings";
import { useLocalStore } from "./local";

export interface SongInfo {
  id: number;
  name: string;
  artists: string;
  album: string;
  duration: number;
  size?: string;
  picUrl?: string;
  fee?: number;
}

export interface DownloadTask {
  id: number;
  name: string;
  artists: string;
  status: "waiting" | "downloading" | "done" | "error";
  progress: number;
  fileSize: string;
  speed?: string;
  hasCover: boolean;
  hasLyric: boolean;
  error?: string;
}

export interface DownloadProgressEvent {
  song_id: number;
  progress: number;
  speed: string;
}

export interface PlaylistTrack {
  id: number;
  name: string;
  artists: string;
  fee: number;
  album: string;
  pic_url: string;
}

export interface PlaylistResponse {
  id: number;
  name: string;
  cover_img_url: string;
  creator: string;
  track_count: number;
  description: string;
  tracks: PlaylistTrack[];
}

export interface AlbumSong {
  id: number;
  name: string;
  artists: string;
  fee: number;
  album: string;
  pic_url: string;
}

export interface AlbumResponse {
  id: number;
  name: string;
  cover_img_url: string;
  artist: string;
  songs: AlbumSong[];
}

export const useDownloadStore = defineStore("download", () => {
  const searchResults = ref<SongInfo[]>([]);
  const selectedIds = ref<Set<number>>(new Set());
  const downloadQueue = ref<DownloadTask[]>([]);
  const isSearching = ref(false);
  const searchQuality = ref("exhigh");

  // Listen for real-time download progress events from Rust backend
  listen<DownloadProgressEvent>("download-progress", (event) => {
    const { song_id, progress, speed } = event.payload;
    for (const task of downloadQueue.value) {
      if (task.id === song_id) {
        task.progress = progress;
        task.speed = speed;
        break;
      }
    }
  });

  const selectedCount = computed(() => selectedIds.value.size);
  const allSelected = computed(() =>
    searchResults.value.length > 0 && selectedIds.value.size === searchResults.value.length
  );

  function toggleSelect(id: number) {
    const newSet = new Set(selectedIds.value);
    if (newSet.has(id)) {
      newSet.delete(id);
    } else {
      newSet.add(id);
    }
    selectedIds.value = newSet;
  }

  function toggleSelectAll() {
    if (allSelected.value) {
      selectedIds.value = new Set();
    } else {
      selectedIds.value = new Set(searchResults.value.map((s) => s.id));
    }
  }

  /** Try to parse a netease URL and extract type + id, or return null */
  function parseNeteaseUrl(input: string): { type: "playlist" | "album"; id: number } | null {
    // 163cn.tv short links — resolve first
    if (/^https?:\/\/163cn\.tv\//i.test(input)) {
      return { type: "playlist", id: -1 }; // signal: short link, resolve later
    }

    try {
      const url = new URL(input);
      if (!url.hostname.endsWith("music.163.com")) return null;

      const fullPath = url.pathname + url.hash;

      // Determine type
      const isAlbum = /album/.test(fullPath);
      const isPlaylist = /playlist/.test(fullPath);
      const linkType = isAlbum ? "album" : isPlaylist ? "playlist" : null;
      if (!linkType) return null;

      // 1) Try query string (real ?id= or hash-fragment ?id=)
      const hashQs = url.hash.includes("?") ? url.hash.slice(url.hash.indexOf("?") + 1) : "";
      const realQs = url.search.slice(1);
      const params = new URLSearchParams(realQs || hashQs || "");
      const idFromQs = params.get("id");
      if (idFromQs) {
        const id = parseInt(idFromQs, 10);
        if (!isNaN(id)) return { type: linkType, id };
      }

      // 2) Try path segments: /playlist/389345581 or #/playlist/389345581
      const idMatch = fullPath.match(/(?:album|playlist)\/(\d+)(?:\?|$)/);
      if (idMatch) {
        const id = parseInt(idMatch[1], 10);
        if (!isNaN(id)) return { type: linkType, id };
      }

      return null;
    } catch {
      return null; // not a valid URL
    }
  }

  async function search(keywords: string) {
    if (!keywords.trim()) return;
    isSearching.value = true;
    selectedIds.value = new Set();
    try {
      const parsed = parseNeteaseUrl(keywords.trim());

      if (parsed) {
        let type = parsed.type;
        let id = parsed.id;

        // Resolve short link
        if (id === -1) {
          const resolvedUrl = await invoke<string>("cmd_resolve_redirect_url", {
            shortUrl: keywords.trim(),
          });
          // Re-parse the resolved URL
          const resolved = parseNeteaseUrl(resolvedUrl);
          if (!resolved) {
            searchResults.value = [];
            isSearching.value = false;
            return;
          }
          type = resolved.type;
          id = resolved.id;
        }

        if (type === "playlist") {
          const playlist = await invoke<PlaylistResponse>("cmd_get_playlist_detail", {
            playlistId: id,
          });
          searchResults.value = playlist.tracks.map((r) => ({
            id: r.id,
            name: r.name,
            artists: r.artists,
            album: r.album,
            duration: 0,
            picUrl: r.pic_url || "",
            fee: r.fee || 0,
          }));
        } else {
          const album = await invoke<AlbumResponse>("cmd_get_album_detail", {
            albumId: id,
          });
          searchResults.value = album.songs.map((r) => ({
            id: r.id,
            name: r.name,
            artists: r.artists,
            album: r.album,
            duration: 0,
            picUrl: r.pic_url || "",
            fee: r.fee || 0,
          }));
        }
      } else {
        // Normal keyword search
        const results = await invoke<SongInfo[]>("cmd_search_music", {
          keywords,
          limit: 30,
        });
        searchResults.value = results.map((r) => ({
          id: r.id,
          name: r.name,
          artists: r.artists,
          album: r.album,
          duration: 0,
          picUrl: r.picUrl || "",
          fee: r.fee || 0,
        }));
      }
    } catch (e) {
      console.error("Search failed:", e);
      searchResults.value = [];
    }
    isSearching.value = false;
  }

  async function startDownload() {
    const settings = useSettingsStore();
    const local = useLocalStore();
    const tasksToDownload = searchResults.value.filter((s) => selectedIds.value.has(s.id));
    selectedIds.value = new Set();

    // Add all tasks to queue as waiting
    for (const song of tasksToDownload) {
      downloadQueue.value.unshift({
        id: song.id,
        name: song.name,
        artists: song.artists,
        status: "waiting",
        progress: 0,
        fileSize: "",
        hasCover: false,
        hasLyric: false,
      });
    }
    local.syncDownloading();

    // Concurrent pool: download up to maxConcurrentDownloads songs in parallel
    const concurrency = settings.settings.maxConcurrentDownloads;
    let running = 0;
    let index = 0;

    await new Promise<void>((resolve) => {
      function kickNext() {
        while (running < concurrency && index < tasksToDownload.length) {
          const song = tasksToDownload[index++];
          running++;
          downloadOne(song).finally(() => {
            running--;
            if (running === 0 && index >= tasksToDownload.length) {
              resolve();
            } else {
              kickNext();
            }
          });
        }
        if (tasksToDownload.length === 0) resolve();
      }
      kickNext();
    });
  }

  async function downloadOne(song: SongInfo) {
    const settings = useSettingsStore();
    const local = useLocalStore();

    // Mark as downloading
    const idx = downloadQueue.value.findIndex((t) => t.id === song.id);
    if (idx !== -1) {
      downloadQueue.value[idx] = { ...downloadQueue.value[idx], status: "downloading" };
    }

    try {
      const result = await invoke<{
        success: boolean;
        filePath: string;
        hasCover: boolean;
        hasLyric: boolean;
        fileSize: number;
      }>("cmd_download_song", {
        songId: song.id,
        quality: searchQuality.value,
        downloadDir: settings.settings.downloadPath,
        downloadCover: settings.settings.downloadCover,
        downloadLyric: settings.settings.downloadLyric,
        qualityFallback: settings.settings.qualityFallback,
      });

      const i = downloadQueue.value.findIndex((t) => t.id === song.id);
      if (i !== -1) {
        const fileSizeStr = result.fileSize > 1024 * 1024
          ? `${(result.fileSize / (1024 * 1024)).toFixed(1)}MB`
          : `${Math.round(result.fileSize / 1024)}KB`;
        downloadQueue.value[i] = {
          ...downloadQueue.value[i],
          status: "done",
          progress: 100,
          fileSize: fileSizeStr,
          hasCover: result.hasCover,
          hasLyric: result.hasLyric,
        };
      }

      await local.scanLocal();
      local.clearDoneItem(song.id);

      setTimeout(() => {
        downloadQueue.value = downloadQueue.value.filter((t) => t.id !== song.id);
      }, 2000);
    } catch (e) {
      const i = downloadQueue.value.findIndex((t) => t.id === song.id);
      if (i !== -1) {
        downloadQueue.value[i] = {
          ...downloadQueue.value[i],
          status: "error",
          error: String(e),
        };
      }
      local.syncDownloading();
    }
  }

  async function retryDownload(songId: number, quality: string, name: string, artists: string) {
    const settings = useSettingsStore();
    const local = useLocalStore();
    
    downloadQueue.value = downloadQueue.value.filter((t) => t.id !== songId);
    local.syncDownloading();
    
    downloadQueue.value.unshift({
      id: songId,
      name,
      artists,
      status: "downloading",
      progress: 0,
      fileSize: "",
      hasCover: false,
      hasLyric: false,
    });
    local.syncDownloading();
    
    try {
      const result = await invoke<{
        success: boolean;
        filePath: string;
        hasCover: boolean;
        hasLyric: boolean;
        fileSize: number;
      }>("cmd_download_song", {
        songId,
        quality,
        downloadDir: settings.settings.downloadPath,
        downloadCover: settings.settings.downloadCover,
        downloadLyric: settings.settings.downloadLyric,
        qualityFallback: settings.settings.qualityFallback,
      });
      
      const idx = downloadQueue.value.findIndex((t) => t.id === songId);
      if (idx !== -1) {
        const fileSizeStr = result.fileSize > 1024 * 1024
          ? `${(result.fileSize / (1024 * 1024)).toFixed(1)}MB`
          : `${Math.round(result.fileSize / 1024)}KB`;
        downloadQueue.value[idx] = {
          ...downloadQueue.value[idx],
          status: "done",
          progress: 100,
          fileSize: fileSizeStr,
          hasCover: result.hasCover,
          hasLyric: result.hasLyric,
        };
      }
      
      await local.scanLocal();
      local.clearDoneItem(songId);
      
      setTimeout(() => {
        downloadQueue.value = downloadQueue.value.filter((t) => t.id !== songId);
      }, 2000);
    } catch (e) {
      const idx = downloadQueue.value.findIndex((t) => t.id === songId);
      if (idx !== -1) {
        downloadQueue.value[idx] = {
          ...downloadQueue.value[idx],
          status: "error",
          error: String(e),
        };
      }
      local.syncDownloading();
    }
  }

  return {
    searchResults, selectedIds, downloadQueue, isSearching, searchQuality,
    selectedCount, allSelected,
    toggleSelect, toggleSelectAll, search, startDownload, retryDownload,
  };
});
