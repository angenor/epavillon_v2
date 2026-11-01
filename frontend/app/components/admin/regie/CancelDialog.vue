<script setup lang="ts">
import type { RegieReason, RegieSession } from '~/types/admin-regie'
import type { TimeZoneName } from '~/types/shared'
import type { SelectOption } from '~/types/ui'
import type { RegieCancelChoice } from '~/utils/regie'

interface Props {
  open: boolean
  session: RegieSession | null
  reasons: RegieReason[]
  /** Raison cochée à l'ouverture (« pas venus » depuis le bouton Absent). */
  initialReason: string | null
  timezone: TimeZoneName
  canPublishIncident: boolean
  submitting: boolean
  error: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:open': [value: boolean]; submit: [choice: RegieCancelChoice] }>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const dt = useDateTime()

const reason = ref<string | null>(null)
const note = ref('')
const publishIncident = ref(true)
const notify = ref(true)

watch(
  () => props.open,
  (open) => {
    if (!open) return
    reason.value = props.initialReason ?? null
    note.value = ''
    publishIncident.value = props.canPublishIncident
    notify.value = true
  },
)

const time = computed(() => (props.session ? dt.time(props.session.starts_at, props.timezone) : ''))
const description = computed(() => {
  const session = props.session
  if (!session) return undefined
  const orgs = regieOrganizationsLabel(session.organizations, locale.value)
  return orgs ? t('admin.regie.dialogs.cancel.lead', { title: tr(session.title), orgs }) : tr(session.title)
})

const options = computed<SelectOption[]>(() =>
  props.reasons.map((entry) => ({ value: entry.code, label: tr(entry.label), description: tr(entry.description) || undefined })),
)
const registrants = computed(() => props.session?.registrants ?? 0)

function submit(): void {
  if (!reason.value) return
  emit('submit', {
    reason_code: reason.value,
    note: reason.value === 'other' && note.value.trim() ? note.value.trim() : null,
    notify_registrants: registrants.value > 0 && notify.value,
    publish_incident: props.canPublishIncident && publishIncident.value,
  })
}
</script>

<template>
  <UiModal
    :open="props.open"
    :title="t('admin.regie.dialogs.cancel.title', { time })"
    :description="description"
    @update:open="emit('update:open', $event)"
  >
    <form id="regie-cancel" class="grid gap-4" @submit.prevent="submit">
      <UiRadio v-model="reason" :options="options" :label="t('admin.regie.dialogs.cancel.reason')" required />
      <UiTextarea
        v-if="reason === 'other'"
        v-model="note"
        :label="t('admin.regie.dialogs.cancel.note')"
        :hint="t('admin.regie.dialogs.cancel.noteHint')"
        :rows="2"
        block
      />
      <UiCheckbox
        v-if="props.canPublishIncident"
        v-model="publishIncident"
        :label="t('admin.regie.dialogs.cancel.publish', { time })"
      />
      <UiCheckbox
        v-if="registrants > 0"
        v-model="notify"
        :label="t('admin.regie.dialogs.cancel.notify', { count: registrants }, registrants)"
        :hint="t('admin.regie.dialogs.cancel.notifyHint')"
      />
      <UiAlert v-if="props.error" intent="danger" live compact :message="props.error" />
    </form>
    <template #footer>
      <UiButton variant="ghost" @click="emit('update:open', false)">{{ t('admin.regie.dialogs.cancel.keep') }}</UiButton>
      <UiButton type="submit" form="regie-cancel" variant="danger" :disabled="!reason" :loading="props.submitting">
        {{ t('admin.regie.dialogs.cancel.confirm') }}
      </UiButton>
    </template>
  </UiModal>
</template>
