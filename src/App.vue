<script setup lang="ts">
import { onMounted, onUnmounted, computed } from 'vue'
import { NConfigProvider, NMessageProvider, NDialogProvider, darkTheme, type GlobalThemeOverrides } from 'naive-ui'
import MainView from './views/MainView.vue'
import { useTunnelStore } from './stores/tunnelStore'
import { useGroupStore } from './stores/groupStore'
import { useUiStore } from './stores/uiStore'
import { onTunnelStatusChanged, onTunnelLog } from './composables/useTauri'
import type { UnlistenFn } from '@tauri-apps/api/event'

const tunnelStore = useTunnelStore()
const groupStore = useGroupStore()
const uiStore = useUiStore()

const theme = computed(() => uiStore.darkMode ? darkTheme : null)

const themeOverrides: GlobalThemeOverrides = {
  common: {
    fontFamily: 'Lato, v-sans, system-ui, -apple-system, sans-serif',
    fontFamilyMono: 'Fira Code, monospace',
  },
}

let unlistenStatus: UnlistenFn | null = null
let unlistenLog: UnlistenFn | null = null

onMounted(async () => {
  // Load initial data
  await Promise.all([
    tunnelStore.fetchTunnels(),
    groupStore.fetchGroups(),
  ])

  // Listen for backend events
  unlistenStatus = await onTunnelStatusChanged((event) => {
    tunnelStore.handleStatusChange(event)
  })

  unlistenLog = await onTunnelLog((event) => {
    tunnelStore.handleLogEntry(event)
  })
})

onUnmounted(() => {
  unlistenStatus?.()
  unlistenLog?.()
})
</script>

<template>
  <NConfigProvider :theme="theme" :theme-overrides="themeOverrides">
    <NMessageProvider>
      <NDialogProvider>
        <MainView />
      </NDialogProvider>
    </NMessageProvider>
  </NConfigProvider>
</template>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html, body, #app {
  height: 100%;
  overflow: hidden;
}
</style>
