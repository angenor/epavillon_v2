<script setup lang="ts">
import type { RegieSession } from '~/types/admin-regie'

interface Props {
  open: boolean
  session: RegieSession | null
  nowMs: number
  submitting: boolean
  error: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:open': [value: boolean]; confirm: [] }>()

const { t, locale } = useI18n()

const name = computed(() => (props.session ? regieOrganizationsLabel(props.session.organizations, locale.value) : ''))
const remaining = computed(() => (props.session ? slotProgress(props.session, props.nowMs).remainingMinutes : 0))
const present = computed(() => props.session?.meeting?.present_count ?? null)

const description = computed(() => {
  const left = t('admin.regie.dialogs.end.remaining', { count: remaining.value }, remaining.value)
  if (present.value === null) return left
  return t('admin.regie.dialogs.end.withPresent', { left, count: present.value }, present.value)
})
</script>

<template>
  <UiModal
    :open="props.open"
    :title="name ? t('admin.regie.dialogs.end.title', { name }) : t('admin.regie.dialogs.end.titleBare')"
    :description="description"
    @update:open="emit('update:open', $event)"
  >
    <p class="text-sm text-text-secondary">{{ t('admin.regie.dialogs.end.resume') }}</p>
    <UiAlert v-if="props.error" class="mt-3" intent="danger" live compact :message="props.error" />
    <template #footer>
      <!-- Le geste sûr reçoit le focus : Entrée ne coupe pas un direct par mégarde. -->
      <UiButton variant="ghost" autofocus @click="emit('update:open', false)">{{ t('admin.regie.dialogs.end.keep') }}</UiButton>
      <UiButton variant="danger" :loading="props.submitting" @click="emit('confirm')">{{ t('admin.regie.dialogs.end.confirm') }}</UiButton>
    </template>
  </UiModal>
</template>
