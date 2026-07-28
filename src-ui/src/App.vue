<script setup lang="ts">
import { computed, onMounted, onUnmounted } from "vue";
import OrbView from "./views/OrbView.vue";
import HoverPanelView from "./views/HoverPanelView.vue";
import ManagementView from "./views/ManagementView.vue";
import { useTokenBallStore } from "./store";
import { closeMainWindow, hideWindow } from "./services/tauri";

const store = useTokenBallStore();
const params = new URLSearchParams(window.location.search);
const view = computed(() => params.get("view") ?? "main");

function handleEscapeKey(event: KeyboardEvent) {
  if (event.key !== "Escape" || event.repeat) return;

  if (view.value === "hover") {
    event.preventDefault();
    void hideWindow("hover");
    return;
  }

  if (view.value === "main") {
    event.preventDefault();
    void closeMainWindow();
  }
}

onMounted(async () => {
  document.body.dataset.view = view.value;
  window.addEventListener("keydown", handleEscapeKey);
  try {
    await store.init();
  } catch (error) {
    store.error = `应用初始化失败：${String(error)}`;
    store.ready = true;
  }
});

onUnmounted(() => {
  window.removeEventListener("keydown", handleEscapeKey);
});
</script>

<template>
  <OrbView v-if="view === 'orb'" />
  <HoverPanelView v-else-if="view === 'hover'" />
  <ManagementView v-else />
</template>
