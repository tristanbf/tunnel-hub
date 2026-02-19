<script setup lang="ts">
import { computed, h } from 'vue'
import {
  NTree, NButton, NIcon, NSpace, NText, NDivider, NDropdown,
  useMessage, useDialog,
} from 'naive-ui'
import {
  FolderOutline, AddOutline, PlayOutline, StopOutline,
  ListOutline, SwapHorizontalOutline,
} from '@vicons/ionicons5'
import type { TreeOption } from 'naive-ui'
import { useGroupStore } from '../../stores/groupStore'
import { useUiStore } from '../../stores/uiStore'
import { useTunnelStore } from '../../stores/tunnelStore'

const groupStore = useGroupStore()
const uiStore = useUiStore()
const tunnelStore = useTunnelStore()
const message = useMessage()
const dialog = useDialog()

/** Count tunnels in this group AND all descendant groups */
function getTunnelCountRecursive(groupId: string): number {
  const descendantIds = groupStore.getDescendantIds(groupId)
  return tunnelStore.tunnels.filter(t => t.group_id !== null && descendantIds.includes(t.group_id)).length
}

// Build tree options with tunnel children
const treeOptions = computed((): TreeOption[] => {
  const buildNodes = (parentId: string | null): TreeOption[] => {
    return groupStore.groups
      .filter(g => g.parent_id === parentId)
      .map(g => {
        const tunnelCount = getTunnelCountRecursive(g.id)
        const subGroups = buildNodes(g.id)
        // Add tunnels belonging directly to this group as leaf nodes
        const tunnelLeaves: TreeOption[] = tunnelStore.tunnels
          .filter(t => t.group_id === g.id)
          .map(t => ({
            key: `tunnel:${t.id}`,
            label: t.name || '(未命名)',
            isLeaf: true,
            prefix: () => h(NIcon, { component: SwapHorizontalOutline, size: 14 }),
            suffix: () => {
              const status = tunnelStore.getStatus(t.id)
              const color = status.status === 'running' ? '#18a058'
                : status.status === 'error' ? '#d03050'
                : '#909399'
              return h('span', {
                style: `display:inline-block;width:6px;height:6px;border-radius:50%;background:${color};margin-left:4px;`,
              })
            },
          }))
        const children = [...subGroups, ...tunnelLeaves]
        return {
          key: g.id,
          label: `${g.name} (${tunnelCount})`,
          prefix: () => h(NIcon, { component: FolderOutline }),
          children: children.length > 0 ? children : undefined,
        }
      })
  }
  return buildNodes(null)
})

const selectedKeys = computed(() => {
  return groupStore.selectedGroupId ? [groupStore.selectedGroupId] : []
})

function handleSelect(keys: string[]) {
  if (keys.length === 0) return
  const key = keys[0]
  // Tunnel leaf nodes start with "tunnel:" — don't select them as groups
  if (key.startsWith('tunnel:')) return
  groupStore.selectGroup(key)
}

function handleShowAll() {
  groupStore.selectGroup(null)
}

function handleContextMenu(key: string) {
  return [
    {
      label: '编辑',
      key: 'edit',
      props: { onClick: () => uiStore.openEditGroup(key) },
    },
    {
      label: '添加子分组',
      key: 'add-child',
      props: { onClick: () => uiStore.openCreateGroup(key) },
    },
    { type: 'divider' as const, key: 'divider1' },
    {
      label: '启动全部',
      key: 'start-all',
      props: {
        onClick: async () => {
          try {
            await groupStore.startGroup(key)
            message.success('分组隧道已启动')
          } catch (e) {
            message.error(`启动失败: ${e}`)
          }
        },
      },
    },
    {
      label: '停止全部',
      key: 'stop-all',
      props: {
        onClick: async () => {
          try {
            await groupStore.stopGroup(key)
            message.success('分组隧道已停止')
          } catch (e) {
            message.error(`停止失败: ${e}`)
          }
        },
      },
    },
    { type: 'divider' as const, key: 'divider2' },
    {
      label: '删除分组',
      key: 'delete',
      props: {
        onClick: () => {
          dialog.warning({
            title: '删除分组',
            content: '确定删除此分组及其所有子分组？隧道不会被删除，但会被移出分组。',
            positiveText: '删除',
            negativeText: '取消',
            onPositiveClick: async () => {
              try {
                await groupStore.deleteGroup(key)
                message.success('分组已删除')
              } catch (e) {
                message.error(`删除失败: ${e}`)
              }
            },
          })
        },
      },
    },
  ]
}

const contextMenuOptions = computed(() => {
  const gid = groupStore.selectedGroupId
  if (!gid) return []
  return handleContextMenu(gid)
})

function getNodeProps(info: { option: TreeOption }) {
  return {
    onContextmenu: (e: MouseEvent) => {
      e.preventDefault()
      groupStore.selectGroup(info.option.key as string)
    },
  }
}
</script>

<template>
  <div class="sidebar-container">
    <div class="sidebar-header">
      <NText strong style="font-size: 14px;">分组</NText>
      <NButton
        size="tiny"
        quaternary
        @click="uiStore.openCreateGroup(null)"
      >
        <template #icon><NIcon :component="AddOutline" :size="16" /></template>
      </NButton>
    </div>

    <div
      class="all-item"
      :class="{ active: !groupStore.selectedGroupId }"
      @click="handleShowAll"
    >
      <NIcon :component="ListOutline" :size="16" style="margin-right: 8px;" />
      <NText>全部隧道 ({{ tunnelStore.totalCount }})</NText>
    </div>

    <NDivider style="margin: 8px 0;" />

    <NTree
      :data="treeOptions"
      block-line
      :selected-keys="selectedKeys"
      selectable
      expand-on-click
      :node-props="getNodeProps"
      @update:selected-keys="handleSelect"
    />

    <!-- Context menu for selected group -->
    <NDropdown
      v-if="groupStore.selectedGroupId"
      trigger="manual"
      :options="contextMenuOptions"
      :show="false"
    />

    <!-- Bottom action for selected group -->
    <div v-if="groupStore.selectedGroupId" class="group-actions">
      <NDivider style="margin: 8px 0;" />
      <NSpace vertical :size="4">
        <NButton
          size="small"
          block
          type="success"
          ghost
          @click="groupStore.startGroup(groupStore.selectedGroupId!).catch(() => {})"
        >
          <template #icon><NIcon :component="PlayOutline" /></template>
          启动分组
        </NButton>
        <NButton
          size="small"
          block
          type="warning"
          ghost
          @click="groupStore.stopGroup(groupStore.selectedGroupId!).catch(() => {})"
        >
          <template #icon><NIcon :component="StopOutline" /></template>
          停止分组
        </NButton>
        <NButton
          size="small"
          block
          quaternary
          @click="uiStore.openEditGroup(groupStore.selectedGroupId!)"
        >
          编辑分组
        </NButton>
      </NSpace>
    </div>
  </div>
</template>

<style scoped>
.sidebar-container {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.all-item {
  display: flex;
  align-items: center;
  padding: 6px 8px;
  border-radius: 4px;
  cursor: pointer;
  transition: background-color 0.2s;
}

.all-item:hover {
  background: rgba(128, 128, 128, 0.1);
}

.all-item.active {
  background: rgba(24, 160, 88, 0.1);
}

.group-actions {
  margin-top: auto;
}
</style>
