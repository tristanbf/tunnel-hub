<script setup lang="ts">
import { computed } from 'vue'
import { NLayout, NLayoutSider, NLayoutContent, NLayoutHeader } from 'naive-ui'
import AppHeader from '../components/layout/AppHeader.vue'
import AppSidebar from '../components/layout/AppSidebar.vue'
import TunnelList from '../components/tunnels/TunnelList.vue'
import TunnelForm from '../components/tunnels/TunnelForm.vue'
import GroupForm from '../components/groups/GroupForm.vue'
import TunnelLogPanel from '../components/logs/TunnelLogPanel.vue'
import SshImportModal from '../components/tunnels/SshImportModal.vue'
import { useTunnelStore } from '../stores/tunnelStore'
import { useGroupStore } from '../stores/groupStore'

const tunnelStore = useTunnelStore()
const groupStore = useGroupStore()

const filteredTunnels = computed(() => {
  const gid = groupStore.selectedGroupId
  if (gid === null) return tunnelStore.tunnels

  // Include tunnels from selected group and all descendant groups
  const groupIds = groupStore.getDescendantIds(gid)
  return tunnelStore.tunnels.filter(t =>
    t.group_id !== null && groupIds.includes(t.group_id)
  )
})
</script>

<template>
  <NLayout class="app-layout" position="absolute">
    <NLayoutHeader bordered class="app-header">
      <AppHeader />
    </NLayoutHeader>

    <NLayout has-sider position="absolute" style="top: 56px;">
      <NLayoutSider
        bordered
        :width="260"
        :collapsed-width="0"
        show-trigger="bar"
        collapse-mode="width"
        content-style="padding: 12px;"
      >
        <AppSidebar />
      </NLayoutSider>

      <NLayoutContent content-style="padding: 16px;" :native-scrollbar="false">
        <TunnelList :tunnels="filteredTunnels" />
      </NLayoutContent>
    </NLayout>

    <!-- Drawers -->
    <TunnelForm />
    <GroupForm />
    <TunnelLogPanel />
    <SshImportModal />
  </NLayout>
</template>

<style scoped>
.app-layout {
  height: 100vh;
}

.app-header {
  height: 56px;
  display: flex;
  align-items: center;
}
</style>
