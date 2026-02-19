<script setup lang="ts">
import { NButton, NSpace, NSwitch, NText, NBadge, useMessage } from 'naive-ui'
import { SunnyOutline, MoonOutline, CloudUploadOutline, CloudDownloadOutline, AddOutline } from '@vicons/ionicons5'
import { NIcon } from 'naive-ui'
import { useUiStore } from '../../stores/uiStore'
import { useTunnelStore } from '../../stores/tunnelStore'
import { save, open } from '@tauri-apps/plugin-dialog'
import { exportConfigToFile, importConfigFromFile } from '../../composables/useTauri'
import { useGroupStore } from '../../stores/groupStore'

const uiStore = useUiStore()
const tunnelStore = useTunnelStore()
const groupStore = useGroupStore()
const message = useMessage()

async function handleExport() {
  try {
    const path = await save({
      title: '导出配置',
      defaultPath: 'tunnelhub_config.json',
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (path) {
      await exportConfigToFile(path)
      message.success('配置已导出')
    }
  } catch (e) {
    message.error(`导出失败: ${e}`)
  }
}

async function handleImport() {
  try {
    const path = await open({
      title: '导入配置',
      filters: [{ name: 'JSON', extensions: ['json'] }],
      multiple: false,
    })
    if (path) {
      await importConfigFromFile(path as string)
      await tunnelStore.fetchTunnels()
      await groupStore.fetchGroups()
      message.success('配置已导入')
    }
  } catch (e) {
    message.error(`导入失败: ${e}`)
  }
}
</script>

<template>
  <div class="header-container">
    <div class="header-left">
      <NText strong style="font-size: 18px; margin-right: 8px;">
        🔗 TunnelHub
      </NText>
      <NBadge
        :value="tunnelStore.runningCount"
        :max="99"
        :show-zero="false"
        type="success"
        :offset="[-4, -2]"
      >
        <NText depth="3" style="font-size: 13px;">
          {{ tunnelStore.totalCount }} 个隧道
        </NText>
      </NBadge>
    </div>

    <div class="header-right">
      <NSpace align="center" :size="8">
        <NButton size="small" quaternary @click="uiStore.openCreateTunnel()">
          <template #icon><NIcon :component="AddOutline" /></template>
          新建隧道
        </NButton>

        <NButton size="small" quaternary @click="handleImport">
          <template #icon><NIcon :component="CloudDownloadOutline" /></template>
          导入
        </NButton>

        <NButton size="small" quaternary @click="handleExport">
          <template #icon><NIcon :component="CloudUploadOutline" /></template>
          导出
        </NButton>

        <NSwitch
          :value="uiStore.darkMode"
          @update:value="uiStore.toggleDarkMode()"
          size="small"
        >
          <template #checked>
            <NIcon :component="MoonOutline" :size="14" />
          </template>
          <template #unchecked>
            <NIcon :component="SunnyOutline" :size="14" />
          </template>
        </NSwitch>
      </NSpace>
    </div>
  </div>
</template>

<style scoped>
.header-container {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 100%;
  padding: 0 20px;
  width: 100%;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.header-right {
  display: flex;
  align-items: center;
}
</style>
