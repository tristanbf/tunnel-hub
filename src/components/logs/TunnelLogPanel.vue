<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import {
  NDrawer, NDrawerContent, NLog, NText, NSpace, NButton, NIcon,
} from 'naive-ui'
import { TrashOutline } from '@vicons/ionicons5'
import { useUiStore } from '../../stores/uiStore'
import { useTunnelStore } from '../../stores/tunnelStore'

const uiStore = useUiStore()
const tunnelStore = useTunnelStore()
const logRef = ref<InstanceType<typeof NLog> | null>(null)

const tunnelName = computed(() => {
  if (!uiStore.logPanelTunnelId) return ''
  const tunnel = tunnelStore.tunnels.find(t => t.id === uiStore.logPanelTunnelId)
  return tunnel?.name ?? uiStore.logPanelTunnelId
})

const logEntries = computed(() => {
  if (!uiStore.logPanelTunnelId) return []
  return tunnelStore.getLogs(uiStore.logPanelTunnelId)
})

const logText = computed(() => {
  return logEntries.value
    .map(entry => {
      const time = new Date(entry.timestamp).toLocaleTimeString()
      const level = entry.level.toUpperCase().padEnd(5)
      return `[${time}] ${level} ${entry.message}`
    })
    .join('\n')
})

function clearLogs() {
  if (uiStore.logPanelTunnelId) {
    tunnelStore.logs.delete(uiStore.logPanelTunnelId)
  }
}

// Auto-scroll to bottom as logs update
watch(logText, async () => {
  await nextTick()
  logRef.value?.scrollTo({ position: 'bottom', silent: true })
})
</script>

<template>
  <NDrawer
    :show="uiStore.showLogPanel"
    :height="350"
    placement="bottom"
    @update:show="(v: boolean) => { if (!v) uiStore.closeLogPanel() }"
  >
    <NDrawerContent closable>
      <template #header>
        <NSpace align="center" :size="12">
          <NText strong>日志 - {{ tunnelName }}</NText>
          <NButton size="tiny" quaternary @click="clearLogs">
            <template #icon><NIcon :component="TrashOutline" :size="14" /></template>
            清除
          </NButton>
        </NSpace>
      </template>

      <NLog
        ref="logRef"
        :log="logText || '暂无日志...'"
        :rows="14"
        language="log"
        style="font-family: 'Fira Code', monospace; font-size: 12px;"
      />
    </NDrawerContent>
  </NDrawer>
</template>
