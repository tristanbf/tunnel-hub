<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import {
  NDrawer, NDrawerContent, NForm, NFormItem, NInput, NInputNumber,
  NSelect, NSwitch, NButton, NSpace, NDivider, NText,
  useMessage,
} from 'naive-ui'
import { FolderOpenOutline } from '@vicons/ionicons5'
import { NIcon } from 'naive-ui'
import { open } from '@tauri-apps/plugin-dialog'
import type { TunnelConfig } from '../../types'
import { createDefaultTunnel, generateSshCommand } from '../../types'
import { useTunnelStore } from '../../stores/tunnelStore'
import { useGroupStore } from '../../stores/groupStore'
import { useUiStore } from '../../stores/uiStore'
import JumpHostList from './JumpHostList.vue'

const tunnelStore = useTunnelStore()
const groupStore = useGroupStore()
const uiStore = useUiStore()
const message = useMessage()

const formData = ref<TunnelConfig>(createDefaultTunnel())
const submitting = ref(false)

const isEditing = computed(() => !!uiStore.editingTunnelId)
const title = computed(() => isEditing.value ? '编辑隧道' : '新建隧道')

// Auth type for the main connection
const authType = ref<string>('agent')

// Group options for select
const groupOptions = computed(() => {
  return [
    { label: '(无分组)', value: '' },
    ...groupStore.groups.map(g => ({
      label: groupStore.getGroupPath(g.id).join(' / '),
      value: g.id,
    })),
  ]
})

// Populate form when editing
watch(() => uiStore.showTunnelForm, (show) => {
  if (show) {
    if (uiStore.editingTunnelId) {
      const tunnel = tunnelStore.tunnels.find(t => t.id === uiStore.editingTunnelId)
      if (tunnel) {
        formData.value = { ...tunnel, jump_hosts: tunnel.jump_hosts.map(j => ({ ...j })) }
        authType.value = tunnel.auth.type
      }
    } else {
      formData.value = createDefaultTunnel()
      // Pre-fill group if one is selected
      formData.value.group_id = groupStore.selectedGroupId
      authType.value = 'agent'
    }
  }
})

// Sync auth type changes
watch(authType, (type) => {
  switch (type) {
    case 'password':
      formData.value.auth = { type: 'password', password: '' }
      break
    case 'keyfile':
      formData.value.auth = { type: 'keyfile', path: '', passphrase: undefined }
      break
    case 'agent':
      formData.value.auth = { type: 'agent' }
      break
  }
})

const authTypeOptions = [
  { label: 'SSH Agent', value: 'agent' },
  { label: '密码', value: 'password' },
  { label: '密钥文件', value: 'keyfile' },
]

const forwardModeOptions = [
  { label: '本地转发 (Local)', value: 'local' },
  { label: '远程转发 (Remote)', value: 'remote' },
  { label: '动态转发 / SOCKS5 (Dynamic)', value: 'dynamic' },
]

const needsRemoteTarget = computed(() =>
  formData.value.forward_mode !== 'dynamic'
)

// SSH command preview
const sshCommandPreview = computed(() => {
  const f = formData.value
  if (!f.ssh_host || !f.local_port) return ''
  return generateSshCommand(f)
})

// Key file picker
async function pickKeyFile() {
  try {
    const path = await open({
      title: '选择 SSH 密钥文件',
      multiple: false,
      filters: [
        { name: '所有文件', extensions: ['*'] },
        { name: '密钥文件', extensions: ['pem', 'key', 'pub', 'ppk'] },
      ],
    })
    if (path && formData.value.auth.type === 'keyfile') {
      (formData.value.auth as any).path = path as string
    }
  } catch (e) {
    message.error(`选择文件失败: ${e}`)
  }
}

async function handleSubmit() {
  // Basic validation
  if (!formData.value.name.trim()) {
    message.warning('请输入隧道名称')
    return
  }
  if (!formData.value.ssh_host.trim()) {
    message.warning('请输入 SSH 主机')
    return
  }
  if (!formData.value.local_port) {
    message.warning('请输入本地端口')
    return
  }

  submitting.value = true
  try {
    // Fix group_id empty string
    if (formData.value.group_id === '') {
      formData.value.group_id = null
    }

    if (isEditing.value) {
      await tunnelStore.updateTunnel(formData.value)
      message.success('隧道已更新')
    } else {
      await tunnelStore.createTunnel(formData.value)
      message.success('隧道已创建')
    }
    uiStore.closeTunnelForm()
  } catch (e) {
    message.error(`保存失败: ${e}`)
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <NDrawer
    :show="uiStore.showTunnelForm"
    :width="520"
    placement="right"
    @update:show="(v: boolean) => { if (!v) uiStore.closeTunnelForm() }"
  >
    <NDrawerContent :title="title" closable>
      <NForm label-placement="left" label-width="100" :model="formData">
        <!-- Basic info -->
        <NFormItem label="名称" required>
          <NInput v-model:value="formData.name" placeholder="例: 公司内网跳转" />
        </NFormItem>

        <NFormItem label="分组">
          <NSelect
            v-model:value="formData.group_id"
            :options="groupOptions"
            placeholder="选择分组"
            clearable
          />
        </NFormItem>

        <NDivider title-placement="left" style="margin: 12px 0;">
          <NText depth="3" style="font-size: 13px;">SSH 连接</NText>
        </NDivider>

        <NFormItem label="SSH 主机" required>
          <NInput v-model:value="formData.ssh_host" placeholder="例: 192.168.1.100" />
        </NFormItem>

        <NFormItem label="SSH 端口">
          <NInputNumber v-model:value="formData.ssh_port" :min="1" :max="65535" style="width: 100%;" />
        </NFormItem>

        <NFormItem label="SSH 用户名">
          <NInput v-model:value="formData.ssh_user" placeholder="例: root" />
        </NFormItem>

        <NFormItem label="认证方式">
          <NSelect v-model:value="authType" :options="authTypeOptions" />
        </NFormItem>

        <NFormItem v-if="authType === 'password'" label="密码">
          <NInput
            v-model:value="(formData.auth as any).password"
            type="password"
            show-password-on="click"
            placeholder="SSH 密码"
          />
        </NFormItem>

        <template v-if="authType === 'keyfile'">
          <NFormItem label="密钥路径">
            <NSpace style="width: 100%;" :wrap="false">
              <NInput
                v-model:value="(formData.auth as any).path"
                placeholder="例: C:\Users\user\.ssh\id_rsa"
                style="flex: 1;"
              />
              <NButton @click="pickKeyFile" size="medium">
                <template #icon><NIcon :component="FolderOpenOutline" /></template>
              </NButton>
            </NSpace>
          </NFormItem>
          <NFormItem label="密钥密码">
            <NInput
              v-model:value="(formData.auth as any).passphrase"
              type="password"
              show-password-on="click"
              placeholder="(可选) 密钥解锁密码"
            />
          </NFormItem>
        </template>

        <NDivider title-placement="left" style="margin: 12px 0;">
          <NText depth="3" style="font-size: 13px;">端口转发</NText>
        </NDivider>

        <NFormItem label="转发模式" required>
          <NSelect v-model:value="formData.forward_mode" :options="forwardModeOptions" />
        </NFormItem>

        <NFormItem label="本地端口" required>
          <NInputNumber v-model:value="formData.local_port" :min="1" :max="65535" style="width: 100%;" placeholder="本地监听端口" />
        </NFormItem>

        <template v-if="needsRemoteTarget">
          <NFormItem label="远程主机">
            <NInput v-model:value="formData.remote_host" placeholder="例: 127.0.0.1" />
          </NFormItem>

          <NFormItem label="远程端口">
            <NInputNumber v-model:value="formData.remote_port" :min="1" :max="65535" style="width: 100%;" />
          </NFormItem>
        </template>

        <NDivider title-placement="left" style="margin: 12px 0;">
          <NText depth="3" style="font-size: 13px;">跳板机 (可选)</NText>
        </NDivider>

        <JumpHostList v-model:jumpHosts="formData.jump_hosts" />

        <NDivider title-placement="left" style="margin: 12px 0;">
          <NText depth="3" style="font-size: 13px;">高级选项</NText>
        </NDivider>

        <NFormItem label="自动重连">
          <NSwitch v-model:value="formData.auto_reconnect" />
        </NFormItem>

        <NFormItem label="额外参数">
          <NInput v-model:value="formData.extra_args" placeholder="额外 SSH 参数 (可选)" />
        </NFormItem>

        <!-- SSH Command Preview -->
        <template v-if="sshCommandPreview">
          <NDivider title-placement="left" style="margin: 12px 0;">
            <NText depth="3" style="font-size: 13px;">SSH 命令预览</NText>
          </NDivider>
          <div style="padding: 8px 12px; border-radius: 4px; background: var(--n-color-embedded, rgba(0,0,0,0.04)); word-break: break-all; font-family: 'Fira Code', monospace; font-size: 12px; line-height: 1.6; color: var(--n-text-color-2, #666);">
            {{ sshCommandPreview }}
          </div>
        </template>
      </NForm>

      <template #footer>
        <NSpace justify="end">
          <NButton @click="uiStore.closeTunnelForm()">取消</NButton>
          <NButton type="primary" :loading="submitting" @click="handleSubmit">
            {{ isEditing ? '保存' : '创建' }}
          </NButton>
        </NSpace>
      </template>
    </NDrawerContent>
  </NDrawer>
</template>
