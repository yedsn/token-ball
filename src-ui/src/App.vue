<script setup lang="ts">
import { computed, onMounted } from "vue";
import OrbView from "./views/OrbView.vue";
import HoverPanelView from "./views/HoverPanelView.vue";
import ManagementView from "./views/ManagementView.vue";
import { useTokenBallStore } from "./store";

const store = useTokenBallStore();
const params = new URLSearchParams(window.location.search);
const view = computed(() => params.get("view") ?? "main");

onMounted(async () => {
  document.body.dataset.view = view.value;
  await store.init();
});
</script>

<template>
  <OrbView v-if="view === 'orb'" />
  <HoverPanelView v-else-if="view === 'hover'" />
  <ManagementView v-else />
</template>
