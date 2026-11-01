<script setup lang="ts">
import type { ManagedIncident } from '~/types/admin-incidents'
import type { RegieSession } from '~/types/admin-regie'
import type { TimeZoneName } from '~/types/shared'
import type { RegieRestoreChoice } from '~/utils/regie'

interface Props {
  open: boolean
  session: RegieSession | null
  /** Le message « annulée » encore en ligne sur cette activité. */
  incident: ManagedIncident | null
  timezone: TimeZoneName
  canPublishIncident: boolean
  submitting: boolean
  error: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:open': [value: boolean]; submit: [choice: RegieRestoreChoice] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const dt = useDateTime()

const unpublish = ref(true)
const notify = ref(true)

watch(
  () => props.open,
  (open) => {
    if (!open) return
    unpublish.value = true
    notify.value = true
  },
)

const time = computed(() => (props.session ? dt.time(props.session.starts_at, props.timezone) : ''))
const notice = computed(() => props.session?.cancellation?.notice ?? null)
const informed = computed(() => (notice.value?.state === 'sent' ? notice.value.recipients : 0))
const canUnpublish = computed(() => props.incident !== null && props.canPublishIncident)

function submit(): void {
  emit('submit', {
    unpublish_incident: canUnpublish.value && unpublish.value,
    notify_registrants: informed.value > 0 && notify.value,
  })
}
</script>

<template>
  <UiModal
    :open="props.open"
    :title="t('admin.regie.dialogs.restore.title', { time })"
    :description="props.session ? t('admin.regie.dialogs.restore.lead', { title: tr(props.session.title) }) : undefined"
    @update:open="emit('update:open', $event)"
  >
    <div class="grid gap-3">
      <UiCheckbox
        v-if="canUnpublish && props.incident"
        v-model="unpublish"
        :label="t('admin.regie.dialogs.restore.unpublish', { message: tr(props.incident.message) })"
      />
      <UiCheckbox
        v-if="informed > 0"
        v-model="notify"
        :label="t('admin.regie.dialogs.restore.notify', { count: informed }, informed)"
      />
      <p v-if="!canUnpublish && informed === 0" class="text-sm text-text-secondary">{{ t('admin.regie.dialogs.restore.nothingElse') }}</p>
      <UiAlert v-if="props.error" intent="danger" live compact :message="props.error" />
    </div>
    <template #footer>
      <UiButton variant="ghost" @click="emit('update:open', false)">{{ t('admin.regie.dialogs.restore.keep') }}</UiButton>
      <UiButton :loading="props.submitting" @click="submit">{{ t('admin.regie.dialogs.restore.confirm') }}</UiButton>
    </template>
  </UiModal>
</template>
