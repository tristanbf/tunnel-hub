<script setup lang="ts">
import {
  NCard, NInput, NInputNumber, NSelect, NButton, NSpace,
  NIcon, NText, NFormItem, NForm,
} from 'naive-ui'
import { AddOutline, TrashOutline, ChevronDownOutline, ChevronUpOutline } from '@vicons/ionicons5'
import type { JumpHost } from '../../types'
import { createDefaultJumpHost } from '../../types'

const props = defineProps<{
  jumpHosts: JumpHost[]
}>()

const emit = defineEmits<{
  'update:jumpHosts': [hosts: JumpHost[]]
}>()

function addJumpHost() {
  emit('update:jumpHosts', [...props.jumpHosts, createDefaultJumpHost()])
}

function removeJumpHost(index: number) {
  const updated = [...props.jumpHosts]
  updated.splice(index, 1)
  emit('update:jumpHosts', updated)
}

function moveUp(index: number) {
  if (index <= 0) return
  const updated = [...props.jumpHosts]
  ;[updated[index - 1], updated[index]] = [updated[index], updated[index - 1]]
  emit('update:jumpHosts', updated)
}

function moveDown(index: number) {
  if (index >= props.jumpHosts.length - 1) return
  const updated = [...props.jumpHosts]
  ;[updated[index], updated[index + 1]] = [updated[index + 1], updated[index]]
  emit('update:jumpHosts', updated)
}

function updateHost(index: number, field: keyof JumpHost, value: any) {
  const updated = [...props.jumpHosts]
  updated[index] = { ...updated[index], [field]: value }
  emit('update:jumpHosts', updated)
}

function updateAuthType(index: number, type: string) {
  const updated = [...props.jumpHosts]
  switch (type) {
    case 'password':
      updated[index] = { ...updated[index], auth: { type: 'password', password: '' } }
      break
    case 'keyfile':
      updated[index] = { ...updated[index], auth: { type: 'keyfile', path: '' } }
      break
    case 'agent':
      updated[index] = { ...updated[index], auth: { type: 'agent' } }
      break
  }
  emit('update:jumpHosts', updated)
}

const authTypeOptions = [
  { label: 'SSH Agent', value: 'agent' },
  { label: '密码', value: 'password' },
  { label: '密钥文件', value: 'keyfile' },
]
</script>

<template>
  <div class="jump-host-list">
    <NCard
      v-for="(jump, index) in jumpHosts"
      :key="index"
      size="small"
      :bordered="true"
      style="margin-bottom: 8px;"
    >
      <template #header>
        <NSpace align="center" justify="space-between" style="width: 100%;">
          <NText depth="2" style="font-size: 13px;">跳板机 {{ index + 1 }}</NText>
          <NSpace :size="4">
            <NButton size="tiny" quaternary :disabled="index === 0" @click="moveUp(index)">
              <template #icon><NIcon :component="ChevronUpOutline" :size="14" /></template>
            </NButton>
            <NButton size="tiny" quaternary :disabled="index === jumpHosts.length - 1" @click="moveDown(index)">
              <template #icon><NIcon :component="ChevronDownOutline" :size="14" /></template>
            </NButton>
            <NButton size="tiny" quaternary type="error" @click="removeJumpHost(index)">
              <template #icon><NIcon :component="TrashOutline" :size="14" /></template>
            </NButton>
          </NSpace>
        </NSpace>
      </template>

      <NForm label-placement="left" label-width="70" size="small">
        <NFormItem label="主机">
          <NInput
            :value="jump.host"
            placeholder="跳板机地址"
            @update:value="(v: string) => updateHost(index, 'host', v)"
          />
        </NFormItem>

        <NFormItem label="端口">
          <NInputNumber
            :value="jump.port"
            :min="1"
            :max="65535"
            style="width: 100%;"
            @update:value="(v: number | null) => updateHost(index, 'port', v ?? 22)"
          />
        </NFormItem>

        <NFormItem label="用户名">
          <NInput
            :value="jump.user"
            placeholder="SSH 用户名"
            @update:value="(v: string) => updateHost(index, 'user', v)"
          />
        </NFormItem>

        <NFormItem label="认证">
          <NSelect
            :value="jump.auth.type"
            :options="authTypeOptions"
            @update:value="(v: string) => updateAuthType(index, v)"
          />
        </NFormItem>

        <NFormItem v-if="jump.auth.type === 'password'" label="密码">
          <NInput
            :value="(jump.auth as any).password"
            type="password"
            show-password-on="click"
            @update:value="(v: string) => {
              const updated = [...jumpHosts]
              updated[index] = { ...updated[index], auth: { type: 'password', password: v } }
              emit('update:jumpHosts', updated)
            }"
          />
        </NFormItem>

        <NFormItem v-if="jump.auth.type === 'keyfile'" label="密钥">
          <NInput
            :value="(jump.auth as any).path"
            placeholder="密钥文件路径"
            @update:value="(v: string) => {
              const updated = [...jumpHosts]
              updated[index] = { ...updated[index], auth: { type: 'keyfile', path: v } }
              emit('update:jumpHosts', updated)
            }"
          />
        </NFormItem>
      </NForm>
    </NCard>

    <NButton
      dashed
      block
      size="small"
      @click="addJumpHost"
    >
      <template #icon><NIcon :component="AddOutline" /></template>
      添加跳板机
    </NButton>
  </div>
</template>

<style scoped>
.jump-host-list {
  width: 100%;
}
</style>
