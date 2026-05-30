import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useSettingsStore } from "./settings";

export interface ConvertTask {
  id: string;
  name: string;
  status: "waiting" | "converting" | "done" | "error";
  progress: number;
  error?: string;
}

export const useConvertStore = defineStore("convert", () => {
  const tasks = ref<ConvertTask[]>([]);
  const showModal = ref(false);
  const format = ref("mp3");
  const embedCover = ref(true);
  const embedLyric = ref(true);
  const outputPath = ref("");

  const isConverting = ref(false);
  const cancelRequested = ref(false);
  const totalCount = ref(0);

  const doneCount = computed(() =>
    tasks.value.filter((t) => t.status === "done" || t.status === "error").length
  );
  const successCount = computed(() =>
    tasks.value.filter((t) => t.status === "done").length
  );
  const errorCount = computed(() =>
    tasks.value.filter((t) => t.status === "error").length
  );
  const progressText = computed(() =>
    doneCount.value + "/" + totalCount.value
  );
  const isDone = computed(() =>
    totalCount.value > 0 && doneCount.value >= totalCount.value
  );

  let progressUnlisten: UnlistenFn | null = null;

  async function startConvert(files: string[]) {
    showModal.value = true;
    totalCount.value = files.length;
    isConverting.value = true;
    cancelRequested.value = false;
    tasks.value = [];

    const settings = useSettingsStore();
    const outDir = outputPath.value || settings.settings.convertOutputPath;

    if (!progressUnlisten) {
      progressUnlisten = await listen<{
        fileKey: string;
        progress: number;
        status: string;
        error?: string;
        outputPath?: string;
      }>("convert-progress", (event) => {
        const idx = tasks.value.findIndex((t) => t.id === event.payload.fileKey);
        if (idx !== -1) {
          tasks.value[idx] = {
            ...tasks.value[idx],
            progress: event.payload.progress,
            status: event.payload.status as ConvertTask["status"],
            error: event.payload.error,
          };
        }
      });
    }

    // Build task list and enqueue all files
    const queue: string[] = [...files];
    let running = 0;
    const maxConcurrent = Math.max(1, Math.min(5, settings.settings.maxConcurrentConverts));

    // Add all tasks in waiting state
    for (const file of files) {
      tasks.value.push({
        id: file,
        name: file.split("/").pop() || file.split("\\").pop() || file,
        status: "waiting",
        progress: 0,
      });
    }

    const scheduleNext = () => {
      if (cancelRequested.value || queue.length === 0) return;
      if (running >= maxConcurrent) return;

      const file = queue.shift()!;
      running++;

      const idx = tasks.value.findIndex((t) => t.id === file);
      if (idx !== -1) {
        tasks.value[idx] = { ...tasks.value[idx], status: "converting", progress: 0 };
      }

      invoke<{
        output_path: string;
        success: boolean;
        error: string | null;
      }>("cmd_convert_audio", {
        params: {
          inputPath: file,
          outputFormat: format.value,
          embedCover: embedCover.value,
          embedLyric: embedLyric.value,
          outputDir: outDir,
        },
      })
        .then((result) => {
          const i = tasks.value.findIndex((t) => t.id === file);
          if (i !== -1) {
            tasks.value[i] = {
              ...tasks.value[i],
              status: result.success ? "done" : "error",
              progress: result.success ? 100 : 0,
              error: result.error || undefined,
            };
          }
        })
        .catch((e) => {
          const i = tasks.value.findIndex((t) => t.id === file);
          if (i !== -1) {
            tasks.value[i] = {
              ...tasks.value[i],
              status: "error",
              error: String(e),
            };
          }
        })
        .finally(() => {
          running--;
          if (doneCount.value >= totalCount.value) {
            isConverting.value = false;
          } else {
            scheduleNext();
          }
        });
    };

    // Kick off initial batch
    for (let i = 0; i < maxConcurrent && i < files.length; i++) {
      scheduleNext();
    }
  }

  function cancelConvert() {
    if (isConverting.value) {
      // During conversion: stop queued tasks
      cancelRequested.value = true;
    } else {
      // After completion: close modal and reset
      tasks.value = [];
      totalCount.value = 0;
      showModal.value = false;
    }
  }

  function openModal() {
    showModal.value = true;
  }

  function closeModal() {
    showModal.value = false;
    tasks.value = [];
    totalCount.value = 0;
    isConverting.value = false;
    cancelRequested.value = false;
  }

  return {
    tasks, showModal, format, embedCover, embedLyric, outputPath,
    isConverting, cancelRequested, totalCount,
    doneCount, successCount, errorCount, progressText, isDone,
    startConvert, cancelConvert, openModal, closeModal,
  };
});
