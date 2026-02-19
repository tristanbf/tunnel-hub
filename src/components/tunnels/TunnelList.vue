<script setup lang="ts">
import { h } from 'vue'
import {
  NDataTable, NButton, NIcon, NSpace, NText, NTag,
  NPopconfirm, NEmpty, NTooltip, useMessage,
} from 'naive-ui'
import type { DataTableColumns } from 'naive-ui'
import {
  PlayOutline, StopOutline, CreateOutline, TrashOutline,
  CopyOutline, TerminalOutline,
} from '@vicons/ionicons5'
import type { TunnelConfig } from '../../types'
import { getForwardingModeLabel, getForwardingModeTag } from '../../types'
import StatusBadge from './StatusBadge.vue'
import { useTunnelStore } from '../../stores/tunnelStore'
import { useUiStore } from '../../stores/uiStore'
import { useGroupStore } from '../../stores/groupStore'

defineProps<{
  tunnels: TunnelConfig[]
}>()

const tunnelStore = useTunnelStore()
const uiStore = useUiStore()
const groupStore = useGroupStore()
const message = useMessage()

async function handleStart(id: string) {
  try {
    await tunnelStore.startTunnel(id)
  } catch (e) {
    message.error(`启动失败: ${e}`)
  }
}

async function handleStop(id: string) {
  try {
    await tunnelStore.stopTunnel(id)
  } catch (e) {
    message.error(`停止失败: ${e}`)
  }
}

async function handleDelete(id: string) {
  try {
    await tunnelStore.deleteTunnel(id)
    message.success('隧道已删除')
  } catch (e) {
    message.error(`删除失败: ${e}`)
  }
}

async function handleDuplicate(id: string) {
  try {
    await tunnelStore.duplicateTunnel(id)
    message.success('隧道已复制')
  } catch (e) {
    message.error(`复制失败: ${e}`)
  }
}

function getGroupName(groupId: string | null): string {
  if (!groupId) return '-'
  const group = groupStore.getGroupById(groupId)
  return group?.name ?? '-'
}

const columns: DataTableColumns<TunnelConfig> = [
  {
    title: '状态',
    key: 'status',
    width: 60,
    align: 'center',
    render(row) {
      const status = tunnelStore.getStatus(row.id)
      return h(StatusBadge, { status })
    },
  },
  {
    title: '名称',
    key: 'name',
    width: 140,
    ellipsis: { tooltip: true },
    render(row) {
      return h(NText, { strong: true }, { default: () => row.name || '(未命名)' })
    },
  },
  {
    title: '类型',
    key: 'forward_mode',
    width: 50,
    align: 'center',
    render(row) {
      const mode = row.forward_mode ?? 'local'
      const tagType = mode === 'local' ? 'info' : mode === 'remote' ? 'warning' : 'success'
      return h(NTooltip, null, {
        trigger: () => h(NTag, { size: 'small', type: tagType, bordered: false, round: true }, {
          default: () => getForwardingModeTag(mode),
        }),
        default: () => getForwardingModeLabel(mode),
      })
    },
  },
  {
    title: '本地端口',
    key: 'local_port',
    width: 90,
    render(row) {
      return h(NTag, { size: 'small', type: 'info', bordered: false }, {
        default: () => `:${row.local_port}`,
      })
    },
  },
  {
    title: '目标地址',
    key: 'target',
    width: 200,
    ellipsis: { tooltip: true },
    render(row) {
      const mode = row.forward_mode ?? 'local'
      const target = `${row.ssh_user}@${row.ssh_host}:${row.ssh_port}`
      if (mode === 'dynamic') {
        return h('div', [
          h(NText, { depth: 2, style: 'font-size: 12px' }, { default: () => target }),
          h('br'),
          h(NText, { depth: 3, style: 'font-size: 11px' }, { default: () => `SOCKS5 :${row.local_port}` }),
        ])
      }
      const arrow = mode === 'remote' ? '←' : '→'
      const forward = `${arrow} ${row.remote_host}:${row.remote_port}`
      return h('div', [
        h(NText, { depth: 2, style: 'font-size: 12px' }, { default: () => target }),
        h('br'),
        h(NText, { depth: 3, style: 'font-size: 11px' }, { default: () => forward }),
      ])
    },
  },
  {
    title: '跳板',
    key: 'jumps',
    width: 60,
    align: 'center',
    render(row) {
      if (row.jump_hosts.length === 0) return h(NText, { depth: 3 }, { default: () => '-' })
      return h(NTag, { size: 'small', type: 'warning', round: true, bordered: false }, {
        default: () => `${row.jump_hosts.length}`,
      })
    },
  },
  {
    title: '分组',
    key: 'group',
    width: 100,
    ellipsis: { tooltip: true },
    render(row) {
      const name = getGroupName(row.group_id)
      return h(NText, { depth: 3 }, { default: () => name })
    },
  },
  {
    title: '操作',
    key: 'actions',
    width: 200,
    fixed: 'right',
    render(row) {
      const status = tunnelStore.getStatus(row.id)
      const isRunning = status.status === 'running' || status.status === 'starting'

      return h(NSpace, { size: 4 }, {
        default: () => [
          // Start / Stop
          isRunning
            ? h(NTooltip, null, {
                trigger: () => h(NButton, {
                  size: 'small',
                  type: 'warning',
                  quaternary: true,
                  onClick: () => handleStop(row.id),
                }, {
                  icon: () => h(NIcon, { component: StopOutline }),
                }),
                default: () => '停止隧道',
              })
            : h(NTooltip, null, {
                trigger: () => h(NButton, {
                  size: 'small',
                  type: 'success',
                  quaternary: true,
                  onClick: () => handleStart(row.id),
                }, {
                  icon: () => h(NIcon, { component: PlayOutline }),
                }),
                default: () => '启动隧道',
              }),
          // Log
          h(NTooltip, null, {
            trigger: () => h(NButton, {
              size: 'small',
              quaternary: true,
              onClick: () => uiStore.openLogPanel(row.id),
            }, {
              icon: () => h(NIcon, { component: TerminalOutline }),
            }),
            default: () => '查看日志',
          }),
          // Edit
          h(NTooltip, null, {
            trigger: () => h(NButton, {
              size: 'small',
              quaternary: true,
              onClick: () => uiStore.openEditTunnel(row.id),
            }, {
              icon: () => h(NIcon, { component: CreateOutline }),
            }),
            default: () => '编辑隧道',
          }),
          // Duplicate
          h(NTooltip, null, {
            trigger: () => h(NButton, {
              size: 'small',
              quaternary: true,
              onClick: () => handleDuplicate(row.id),
            }, {
              icon: () => h(NIcon, { component: CopyOutline }),
            }),
            default: () => '复制隧道',
          }),
          // Delete
          h(NPopconfirm, {
            onPositiveClick: () => handleDelete(row.id),
          }, {
            trigger: () => h(NTooltip, null, {
              trigger: () => h(NButton, {
                size: 'small',
                type: 'error',
                quaternary: true,
              }, {
                icon: () => h(NIcon, { component: TrashOutline }),
              }),
              default: () => '删除隧道',
            }),
            default: () => '确定删除这个隧道？',
          }),
        ],
      })
    },
  },
]
</script>

<template>
  <div class="tunnel-list-container">
    <div class="list-header">
      <NText strong style="font-size: 15px;">
        {{ groupStore.selectedGroupId ? `${groupStore.getGroupById(groupStore.selectedGroupId)?.name ?? ''} - 隧道列表` : '全部隧道' }}
      </NText>
      <NButton type="primary" size="small" @click="uiStore.openCreateTunnel()">
        <template #icon><NIcon :component="PlayOutline" /></template>
        新建隧道
      </NButton>
    </div>

    <NDataTable
      v-if="tunnels.length > 0"
      :columns="columns"
      :data="tunnels"
      :row-key="(row: TunnelConfig) => row.id"
      :bordered="false"
      :single-line="false"
      striped
      size="small"
      :scroll-x="800"
      style="margin-top: 12px;"
    />

    <NEmpty
      v-else
      description="暂无隧道"
      style="margin-top: 80px;"
    >
      <template #extra>
        <NButton size="small" type="primary" @click="uiStore.openCreateTunnel()">
          创建第一个隧道
        </NButton>
      </template>
    </NEmpty>
  </div>
</template>

<style scoped>
.tunnel-list-container {
  height: 100%;
}

.list-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}
</style>
