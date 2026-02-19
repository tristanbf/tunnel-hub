<script setup lang="ts">
import { computed } from 'vue'
import type { TunnelStatus } from '../../types'
import { getStatusColor, getStatusLabel } from '../../types'

const props = defineProps<{
  status: TunnelStatus
}>()

const color = computed(() => getStatusColor(props.status))
const label = computed(() => getStatusLabel(props.status))
const isPulsing = computed(() =>
  props.status.status === 'starting' ||
  props.status.status === 'reconnecting' ||
  props.status.status === 'stopping'
)
</script>

<template>
  <div class="status-badge" :title="label">
    <span
      class="status-dot"
      :class="{ pulsing: isPulsing }"
      :style="{ backgroundColor: color }"
    />
  </div>
</template>

<style scoped>
.status-badge {
  display: flex;
  align-items: center;
  justify-content: center;
}

.status-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  display: inline-block;
  transition: background-color 0.3s;
}

.status-dot.pulsing {
  animation: pulse 1.5s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.5; transform: scale(0.8); }
}
</style>
