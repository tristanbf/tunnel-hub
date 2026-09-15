<script setup lang="ts">
import { computed, h, onUnmounted, ref } from 'vue'
import {
  NTree, NButton, NIcon, NSpace, NText, NDivider, NDropdown,
  useMessage, useDialog,
} from 'naive-ui'
import {
  FolderOutline, AddOutline, PlayOutline, StopOutline,
  ListOutline, SwapHorizontalOutline,
} from '@vicons/ionicons5'
import type { TreeOption } from 'naive-ui'
import type { Group, GroupReorderItem } from '../../types'
import { useGroupStore } from '../../stores/groupStore'
import { useUiStore } from '../../stores/uiStore'
import { useTunnelStore } from '../../stores/tunnelStore'

const DRAG_THRESHOLD = 5

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

const expandedKeys = ref<string[]>([])
const treeWrapRef = ref<HTMLElement | null>(null)
const draggingId = ref<string | null>(null)
const dropTarget = ref<{ id: string; after: boolean } | null>(null)

let pointerId: number | null = null
let startY = 0
let startX = 0
let startId: string | null = null
let dragActivated = false

function isTunnelKey(key: unknown): boolean {
  return String(key).startsWith('tunnel:')
}

function isInteractiveTarget(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false
  return !!target.closest('button, a, .n-button, .n-tree-node-switcher')
}

function detachPointerListeners() {
  window.removeEventListener('pointermove', handlePointerMove)
  window.removeEventListener('pointerup', handlePointerUp)
  window.removeEventListener('pointercancel', handlePointerUp)
}

function syncDragClasses() {
  const root = treeWrapRef.value
  if (!root) return
  const target = dropTarget.value
  for (const el of root.querySelectorAll<HTMLElement>('.n-tree-node[data-group-id]')) {
    const id = el.dataset.groupId
    el.classList.toggle('group-node-dragging', draggingId.value === id)
    el.classList.toggle('group-node-drop-before', !!target && target.id === id && !target.after)
    el.classList.toggle('group-node-drop-after', !!target && target.id === id && target.after)
  }
}

function handlePointerDown(e: PointerEvent, groupId: string) {
  if (e.button !== 0) return
  if (isInteractiveTarget(e.target)) return

  pointerId = e.pointerId
  startX = e.clientX
  startY = e.clientY
  startId = groupId
  dragActivated = false

  window.addEventListener('pointermove', handlePointerMove)
  window.addEventListener('pointerup', handlePointerUp)
  window.addEventListener('pointercancel', handlePointerUp)
}

function updateDropTarget(x: number, y: number) {
  if (!startId) {
    dropTarget.value = null
    return
  }
  const el = document.elementFromPoint(x, y)
  const node = el?.closest('.n-tree-node[data-group-id]') as HTMLElement | null
  const overId = node?.dataset.groupId
  if (overId) {
    if (overId === startId || groupStore.getDescendantIds(startId).includes(overId)) {
      dropTarget.value = null
      return
    }
    const rect = node.getBoundingClientRect()
    dropTarget.value = { id: overId, after: y > rect.top + rect.height / 2 }
    return
  }

  const wrap = treeWrapRef.value
  const dragId = startId
  if (!wrap || !dragId) {
    dropTarget.value = null
    return
  }
  if (!wrap.contains(el) && el !== wrap) {
    dropTarget.value = null
    return
  }

  const nodes = [...wrap.querySelectorAll<HTMLElement>('.n-tree-node[data-group-id]')]
  const last = [...nodes].reverse().find(n => {
    const id = n.dataset.groupId
    return !!id && id !== dragId && !groupStore.getDescendantIds(dragId).includes(id)
  })
  dropTarget.value = last?.dataset.groupId
    ? { id: last.dataset.groupId, after: true }
    : null
}

function handlePointerMove(e: PointerEvent) {
  if (pointerId !== e.pointerId || !startId) return

  if (!dragActivated) {
    if (Math.hypot(e.clientX - startX, e.clientY - startY) < DRAG_THRESHOLD) return
    dragActivated = true
    draggingId.value = startId
    document.body.classList.add('group-tree-reordering')
  }

  e.preventDefault()
  updateDropTarget(e.clientX, e.clientY)
  syncDragClasses()
}

async function handlePointerUp(e: PointerEvent) {
  if (pointerId !== e.pointerId) return

  const fromId = startId
  const target = dropTarget.value
  const shouldReorder = dragActivated && !!fromId && !!target

  if (dragActivated) {
    const stopClick = (ev: MouseEvent) => {
      ev.preventDefault()
      ev.stopPropagation()
      window.removeEventListener('click', stopClick, true)
    }
    window.addEventListener('click', stopClick, true)
  }

  detachPointerListeners()
  document.body.classList.remove('group-tree-reordering')
  pointerId = null
  startId = null
  dragActivated = false
  draggingId.value = null
  dropTarget.value = null
  syncDragClasses()

  if (!shouldReorder || !fromId || !target) return

  try {
    await applyGroupReorder(fromId, target.id, target.after)
  } catch (err) {
    message.error(`调整顺序失败: ${err}`)
  }
}

async function applyGroupReorder(fromId: string, toId: string, placeAfter: boolean) {
  const targetGroup = groupStore.getGroupById(toId)
  if (!targetGroup) return

  const newParentId = targetGroup.parent_id
  if (newParentId === fromId) return
  if (groupStore.getDescendantIds(fromId).includes(toId)) return

  const siblings = groupStore.groups
    .filter(g => g.id !== fromId && g.parent_id === newParentId)
    .map(g => g.id)
  const idx = siblings.indexOf(toId)
  if (idx < 0) return

  const orderedSiblings = placeAfter
    ? [...siblings.slice(0, idx + 1), fromId, ...siblings.slice(idx + 1)]
    : [...siblings.slice(0, idx), fromId, ...siblings.slice(idx)]

  const items = flattenMovedGroups(groupStore.groups, fromId, newParentId, orderedSiblings)
  if (items.length !== groupStore.groups.length) return
  const unchanged = items.every((item, i) => {
    const g = groupStore.groups[i]
    return g.id === item.id && g.parent_id === item.parent_id
  })
  if (unchanged) return

  await groupStore.reorderGroups(items)
}

onUnmounted(() => {
  detachPointerListeners()
  document.body.classList.remove('group-tree-reordering')
})

function flattenMovedGroups(
  groups: Group[],
  movedId: string,
  newParentId: string | null,
  siblingOrder: string[],
): GroupReorderItem[] {
  const updated = groups.map(g => (
    g.id === movedId ? { ...g, parent_id: newParentId } : g
  ))

  const byParent = new Map<string | null, Group[]>()
  for (const g of updated) {
    const siblings = byParent.get(g.parent_id)
    if (siblings) siblings.push(g)
    else byParent.set(g.parent_id, [g])
  }

  const orderIdx = new Map(siblingOrder.map((id, i) => [id, i]))
  const movedSiblings = byParent.get(newParentId)
  if (movedSiblings) {
    movedSiblings.sort((a, b) => (orderIdx.get(a.id) ?? 0) - (orderIdx.get(b.id) ?? 0))
  }

  const result: GroupReorderItem[] = []
  const walk = (parentId: string | null) => {
    for (const g of byParent.get(parentId) ?? []) {
      result.push({ id: g.id, parent_id: g.parent_id })
      walk(g.id)
    }
  }
  walk(null)
  return result
}

function handleExpandedKeys(keys: string[]) {
  expandedKeys.value = keys
}

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
  const key = String(info.option.key)
  const isTunnel = isTunnelKey(key)
  return {
    class: isTunnel ? 'tree-tunnel-node' : 'tree-group-node',
    'data-group-id': isTunnel ? undefined : key,
    onPointerdown: isTunnel ? undefined : (e: PointerEvent) => handlePointerDown(e, key),
    onContextmenu: (e: MouseEvent) => {
      e.preventDefault()
      if (!isTunnel) {
        groupStore.selectGroup(key)
      }
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

    <div
      ref="treeWrapRef"
      class="group-tree"
      :class="{ 'is-reordering': !!draggingId }"
    >
      <NTree
        :data="treeOptions"
        block-line
        :selected-keys="selectedKeys"
        :expanded-keys="expandedKeys"
        selectable
        expand-on-click
        :node-props="getNodeProps"
        @update:selected-keys="handleSelect"
        @update:expanded-keys="handleExpandedKeys"
      />
    </div>

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

.group-tree {
  flex: 1;
  min-height: 0;
  overflow: auto;
}

:deep(.tree-group-node) {
  cursor: grab;
  user-select: none;
  touch-action: none;
}

:deep(.tree-group-node .n-tree-node-content) {
  cursor: grab;
}

:deep(.tree-tunnel-node),
:deep(.tree-tunnel-node .n-tree-node-content) {
  cursor: default;
}

:deep(.group-node-dragging) {
  opacity: 0.45;
}

:deep(.group-node-drop-before) {
  box-shadow: inset 0 2px 0 0 #18a058;
}

:deep(.group-node-drop-after) {
  box-shadow: inset 0 -2px 0 0 #18a058;
}

.is-reordering :deep(*) {
  cursor: grabbing !important;
}
</style>

<style>
body.group-tree-reordering {
  cursor: grabbing;
  user-select: none;
}
</style>
