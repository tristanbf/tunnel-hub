import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Group, GroupTreeNode } from '@/types'
import * as api from '@/composables/useTauri'

export const useGroupStore = defineStore('group', () => {
  // ─── State ─────────────────────────────────────────────────

  const groups = ref<Group[]>([])
  const selectedGroupId = ref<string | null>(null) // null = show all
  const loading = ref(false)

  // ─── Getters ───────────────────────────────────────────────

  /** Build a tree structure for NTree from the flat groups list */
  const groupTree = computed((): GroupTreeNode[] => {
    const buildChildren = (parentId: string | null): GroupTreeNode[] => {
      return groups.value
        .filter(g => g.parent_id === parentId)
        .map(g => ({
          key: g.id,
          label: g.name,
          isGroup: true,
          children: buildChildren(g.id),
        }))
    }
    return buildChildren(null)
  })

  /** Get all descendant group IDs (including the given id) */
  function getDescendantIds(groupId: string): string[] {
    const result = [groupId]
    for (const g of groups.value) {
      if (g.parent_id === groupId) {
        result.push(...getDescendantIds(g.id))
      }
    }
    return result
  }

  function getGroupById(id: string): Group | undefined {
    return groups.value.find(g => g.id === id)
  }

  /** Get the names of ancestor groups (for breadcrumb) */
  function getGroupPath(groupId: string): string[] {
    const path: string[] = []
    let current = groups.value.find(g => g.id === groupId)
    while (current) {
      path.unshift(current.name)
      current = current.parent_id
        ? groups.value.find(g => g.id === current!.parent_id)
        : undefined
    }
    return path
  }

  // ─── Actions ───────────────────────────────────────────────

  async function fetchGroups() {
    loading.value = true
    try {
      groups.value = await api.listGroups()
    } finally {
      loading.value = false
    }
  }

  async function createGroup(name: string, parentId?: string | null) {
    const group = await api.createGroup(name, parentId)
    groups.value.push(group)
    return group
  }

  async function updateGroup(group: Group) {
    await api.updateGroup(group)
    const idx = groups.value.findIndex(g => g.id === group.id)
    if (idx >= 0) {
      groups.value[idx] = group
    }
  }

  async function deleteGroup(id: string) {
    await api.deleteGroup(id)
    // Remove the group and all descendants from local state
    const toRemove = getDescendantIds(id)
    groups.value = groups.value.filter(g => !toRemove.includes(g.id))
    // If the selected group was deleted, reset selection
    if (selectedGroupId.value && toRemove.includes(selectedGroupId.value)) {
      selectedGroupId.value = null
    }
  }

  async function startGroup(groupId: string) {
    await api.startGroup(groupId)
  }

  async function stopGroup(groupId: string) {
    await api.stopGroup(groupId)
  }

  function selectGroup(id: string | null) {
    selectedGroupId.value = id
  }

  return {
    // State
    groups,
    selectedGroupId,
    loading,
    // Getters
    groupTree,
    getDescendantIds,
    getGroupById,
    getGroupPath,
    // Actions
    fetchGroups,
    createGroup,
    updateGroup,
    deleteGroup,
    startGroup,
    stopGroup,
    selectGroup,
  }
})
