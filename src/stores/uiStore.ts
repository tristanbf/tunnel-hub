import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useUiStore = defineStore('ui', () => {
  // Dark mode
  const darkMode = ref(loadDarkMode())

  function toggleDarkMode() {
    darkMode.value = !darkMode.value
    localStorage.setItem('tunnelhub-dark-mode', String(darkMode.value))
  }

  function loadDarkMode(): boolean {
    const saved = localStorage.getItem('tunnelhub-dark-mode')
    if (saved !== null) return saved === 'true'
    return window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? false
  }

  // Tunnel form drawer
  const showTunnelForm = ref(false)
  const editingTunnelId = ref<string | null>(null)

  function openCreateTunnel() {
    editingTunnelId.value = null
    showTunnelForm.value = true
  }

  function openEditTunnel(id: string) {
    editingTunnelId.value = id
    showTunnelForm.value = true
  }

  function closeTunnelForm() {
    showTunnelForm.value = false
    editingTunnelId.value = null
  }

  // Group form
  const showGroupForm = ref(false)
  const editingGroupId = ref<string | null>(null)
  const newGroupParentId = ref<string | null>(null)

  function openCreateGroup(parentId: string | null = null) {
    editingGroupId.value = null
    newGroupParentId.value = parentId
    showGroupForm.value = true
  }

  function openEditGroup(id: string) {
    editingGroupId.value = id
    showGroupForm.value = true
  }

  function closeGroupForm() {
    showGroupForm.value = false
    editingGroupId.value = null
    newGroupParentId.value = null
  }

  // Log panel
  const showLogPanel = ref(false)
  const logPanelTunnelId = ref<string | null>(null)

  function openLogPanel(tunnelId: string) {
    logPanelTunnelId.value = tunnelId
    showLogPanel.value = true
  }

  function closeLogPanel() {
    showLogPanel.value = false
    logPanelTunnelId.value = null
  }

  return {
    darkMode,
    toggleDarkMode,
    showTunnelForm,
    editingTunnelId,
    openCreateTunnel,
    openEditTunnel,
    closeTunnelForm,
    showGroupForm,
    editingGroupId,
    newGroupParentId,
    openCreateGroup,
    openEditGroup,
    closeGroupForm,
    showLogPanel,
    logPanelTunnelId,
    openLogPanel,
    closeLogPanel,
  }
})
