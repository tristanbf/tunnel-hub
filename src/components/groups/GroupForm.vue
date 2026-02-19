<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import {
  NModal, NCard, NForm, NFormItem, NInput, NSelect, NButton, NSpace,
  useMessage,
} from 'naive-ui'
import { useGroupStore } from '../../stores/groupStore'
import { useUiStore } from '../../stores/uiStore'

const groupStore = useGroupStore()
const uiStore = useUiStore()
const message = useMessage()

const name = ref('')
const parentId = ref<string | null>(null)
const submitting = ref(false)

const isEditing = computed(() => !!uiStore.editingGroupId)
const title = computed(() => isEditing.value ? '编辑分组' : '新建分组')

// Parent group options (exclude self and descendants when editing)
const parentOptions = computed(() => {
  let available = groupStore.groups
  if (uiStore.editingGroupId) {
    const excluded = groupStore.getDescendantIds(uiStore.editingGroupId)
    available = available.filter(g => !excluded.includes(g.id))
  }
  return [
    { label: '(根级分组)', value: '' },
    ...available.map(g => ({
      label: groupStore.getGroupPath(g.id).join(' / '),
      value: g.id,
    })),
  ]
})

// Populate form when editing
watch(() => uiStore.showGroupForm, (show) => {
  if (show) {
    if (uiStore.editingGroupId) {
      const group = groupStore.getGroupById(uiStore.editingGroupId)
      if (group) {
        name.value = group.name
        parentId.value = group.parent_id
      }
    } else {
      name.value = ''
      parentId.value = uiStore.newGroupParentId
    }
  }
})

async function handleSubmit() {
  if (!name.value.trim()) {
    message.warning('请输入分组名称')
    return
  }

  submitting.value = true
  try {
    const pid = parentId.value === '' ? null : parentId.value

    if (isEditing.value) {
      await groupStore.updateGroup({
        id: uiStore.editingGroupId!,
        name: name.value.trim(),
        parent_id: pid,
      })
      message.success('分组已更新')
    } else {
      await groupStore.createGroup(name.value.trim(), pid)
      message.success('分组已创建')
    }
    uiStore.closeGroupForm()
  } catch (e) {
    message.error(`保存失败: ${e}`)
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <NModal
    :show="uiStore.showGroupForm"
    @update:show="(v: boolean) => { if (!v) uiStore.closeGroupForm() }"
  >
    <NCard
      :title="title"
      style="width: 420px;"
      :bordered="false"
      size="medium"
      closable
      @close="uiStore.closeGroupForm()"
    >
      <NForm label-placement="left" label-width="80">
        <NFormItem label="分组名称" required>
          <NInput v-model:value="name" placeholder="例: 远程桌面" />
        </NFormItem>

        <NFormItem label="父级分组">
          <NSelect
            v-model:value="parentId"
            :options="parentOptions"
            placeholder="选择父级分组"
            clearable
          />
        </NFormItem>
      </NForm>

      <template #footer>
        <NSpace justify="end">
          <NButton @click="uiStore.closeGroupForm()">取消</NButton>
          <NButton type="primary" :loading="submitting" @click="handleSubmit">
            {{ isEditing ? '保存' : '创建' }}
          </NButton>
        </NSpace>
      </template>
    </NCard>
  </NModal>
</template>
