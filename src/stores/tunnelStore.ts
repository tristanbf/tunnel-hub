import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { TunnelConfig, TunnelStatus, LogEntry } from '@/types'
import * as api from '@/composables/useTauri'

export const useTunnelStore = defineStore('tunnel', () => {
  // ─── State ─────────────────────────────────────────────────

  const tunnels = ref<TunnelConfig[]>([])
  const statuses = ref<Map<string, TunnelStatus>>(new Map())
  const logs = ref<Map<string, LogEntry[]>>(new Map())
  const loading = ref(false)

  // ─── Getters ───────────────────────────────────────────────

  const tunnelsByGroup = computed(() => {
    return (groupId: string | null) => {
      if (groupId === null) {
        return tunnels.value
      }
      return tunnels.value.filter(t => t.group_id === groupId)
    }
  })

  const ungroupedTunnels = computed(() => {
    return tunnels.value.filter(t => !t.group_id)
  })

  const runningCount = computed(() => {
    let count = 0
    statuses.value.forEach(s => {
      if (s.status === 'running') count++
    })
    return count
  })

  const totalCount = computed(() => tunnels.value.length)

  function getStatus(id: string): TunnelStatus {
    return statuses.value.get(id) ?? { status: 'stopped' }
  }

  function getLogs(id: string): LogEntry[] {
    return logs.value.get(id) ?? []
  }

  // ─── Actions ───────────────────────────────────────────────

  async function fetchTunnels() {
    loading.value = true
    try {
      tunnels.value = await api.listTunnels()
      const allStatuses = await api.getAllStatuses()
      const map = new Map<string, TunnelStatus>()
      for (const [id, status] of allStatuses) {
        map.set(id, status)
      }
      statuses.value = map
    } finally {
      loading.value = false
    }
  }

  async function createTunnel(config: TunnelConfig) {
    const created = await api.createTunnel(config)
    tunnels.value.push(created)
    statuses.value.set(created.id, { status: 'stopped' })
    return created
  }

  async function updateTunnel(config: TunnelConfig) {
    await api.updateTunnel(config)
    const idx = tunnels.value.findIndex(t => t.id === config.id)
    if (idx >= 0) {
      tunnels.value[idx] = config
    }
  }

  async function deleteTunnel(id: string) {
    await api.deleteTunnel(id)
    tunnels.value = tunnels.value.filter(t => t.id !== id)
    statuses.value.delete(id)
    logs.value.delete(id)
  }

  async function duplicateTunnel(id: string) {
    const copy = await api.duplicateTunnel(id)
    tunnels.value.push(copy)
    statuses.value.set(copy.id, { status: 'stopped' })
    return copy
  }

  async function startTunnel(id: string) {
    statuses.value.set(id, { status: 'starting' })
    try {
      await api.startTunnel(id)
    } catch (e) {
      statuses.value.set(id, { status: 'error', message: String(e) })
      throw e
    }
  }

  async function stopTunnel(id: string) {
    statuses.value.set(id, { status: 'stopping' })
    try {
      await api.stopTunnel(id)
      statuses.value.set(id, { status: 'stopped' })
    } catch (e) {
      throw e
    }
  }

  async function assignToGroup(tunnelId: string, groupId: string | null) {
    await api.assignTunnelToGroup(tunnelId, groupId)
    const tunnel = tunnels.value.find(t => t.id === tunnelId)
    if (tunnel) {
      tunnel.group_id = groupId
    }
  }

  // ─── Event Handlers ───────────────────────────────────────

  function handleStatusChange(event: { tunnel_id: string; status: TunnelStatus }) {
    statuses.value.set(event.tunnel_id, event.status)
  }

  function handleLogEntry(event: { tunnel_id: string; entry: LogEntry }) {
    const existing = logs.value.get(event.tunnel_id) ?? []
    existing.push(event.entry)
    // Keep last 500
    if (existing.length > 500) {
      existing.splice(0, existing.length - 500)
    }
    logs.value.set(event.tunnel_id, existing)
  }

  return {
    // State
    tunnels,
    statuses,
    logs,
    loading,
    // Getters
    tunnelsByGroup,
    ungroupedTunnels,
    runningCount,
    totalCount,
    getStatus,
    getLogs,
    // Actions
    fetchTunnels,
    createTunnel,
    updateTunnel,
    deleteTunnel,
    duplicateTunnel,
    startTunnel,
    stopTunnel,
    assignToGroup,
    handleStatusChange,
    handleLogEntry,
  }
})
