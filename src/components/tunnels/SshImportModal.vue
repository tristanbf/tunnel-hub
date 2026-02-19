<script setup lang="ts">
import { ref } from 'vue'
import {
  NModal, NCard, NInput, NButton, NSpace, NText, NAlert,
  useMessage,
} from 'naive-ui'
import { useUiStore } from '../../stores/uiStore'
import { useTunnelStore } from '../../stores/tunnelStore'
import { parseSshCommand } from '../../composables/useTauri'

const uiStore = useUiStore()
const tunnelStore = useTunnelStore()
const message = useMessage()

const sshCommand = ref('')
const parsing = ref(false)
const errorMsg = ref('')

async function handleImport() {
  const cmd = sshCommand.value.trim()
  if (!cmd) {
    message.warning('请输入 SSH 命令')
    return
  }

  parsing.value = true
  errorMsg.value = ''
  try {
    const config = await parseSshCommand(cmd)
    await tunnelStore.createTunnel(config)
    message.success(`隧道 "${config.name}" 已创建`)
    sshCommand.value = ''
    uiStore.closeSshImport()
  } catch (e) {
    errorMsg.value = String(e)
  } finally {
    parsing.value = false
  }
}

function handleClose() {
  sshCommand.value = ''
  errorMsg.value = ''
  uiStore.closeSshImport()
}
</script>

<template>
  <NModal
    :show="uiStore.showSshImport"
    @update:show="(v: boolean) => { if (!v) handleClose() }"
  >
    <NCard
      title="从 SSH 命令导入"
      :bordered="true"
      size="medium"
      style="width: 600px; max-width: 90vw;"
      closable
      @close="handleClose"
    >
      <NSpace vertical :size="12">
        <NText depth="3" style="font-size: 13px;">
          粘贴 SSH 命令，将自动解析为隧道配置。支持 -L (本地转发)、-R (远程转发)、-D (动态代理) 参数。
        </NText>

        <NInput
          v-model:value="sshCommand"
          type="textarea"
          :rows="4"
          placeholder="例: ssh -fNg -L 13389:192.168.1.100:3389 user@host -p 22"
          style="font-family: 'Fira Code', monospace; font-size: 13px;"
          @keydown.ctrl.enter="handleImport"
        />

        <NAlert v-if="errorMsg" type="error" :bordered="false">
          {{ errorMsg }}
        </NAlert>
      </NSpace>

      <template #footer>
        <NSpace justify="end">
          <NButton @click="handleClose">取消</NButton>
          <NButton type="primary" :loading="parsing" @click="handleImport">
            导入
          </NButton>
        </NSpace>
      </template>
    </NCard>
  </NModal>
</template>
