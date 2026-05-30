import { createRouter, createWebHistory } from "vue-router";
import DownloadView from "./views/DownloadView.vue";
import LocalView from "./views/LocalView.vue";
import SettingsView from "./views/SettingsView.vue";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", redirect: "/download" },
    { path: "/download", name: "download", component: DownloadView },
    { path: "/local", name: "local", component: LocalView },
    { path: "/settings", name: "settings", component: SettingsView },
  ],
});

export default router;
