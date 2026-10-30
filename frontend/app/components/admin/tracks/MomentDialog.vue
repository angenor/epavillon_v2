<script setup lang="ts">
import type { TrackMoment, TrackMomentPayload, TrackPart } from '~/types/event/track-page'
import type { I18nText, IsoDate, TimeZoneName } from '~/types/shared'
import type { SelectOption } from '~/types/ui'

/**
 * Un moment sans activité : accueil, déjeuner, pause, clôture. Ni inscription,
 * ni conflit, ni programmation publique. Heures saisies dans le fuseau de
 * l'édition ; la fin est facultative (la clôture n'a qu'un début).
 */

const props = defineProps<{
  open: boolean
  /** Nul : ajout. */
  moment: TrackMoment | null
  parts: TrackPart[]
  timezone: TimeZoneName
  zoneLabel: string
  /** Jour proposé à l'ajout : le début annoncé du fil. */
  defaultDay: IsoDate
  busy?: boolean
  error?: string | null
}>()
const emit = defineEmits<{ close: []; save: [payload: TrackMomentPayload] }>()

const { t } = useI18n()
const { tr } = useI18nText()

const start = ref('')
const end = ref('')
const label = ref<I18nText | null>(null)
const description = ref<I18nText | null>(null)
const partId = ref('')
const tried = ref(false)

const wallClock = (value: string | null): string => (value ? wallClockInZone(value, props.timezone).replace(' ', 'T') : '')

watch(
  () => [props.open, props.moment] as const,
  ([open, moment]) => {
    if (!open) return
    start.value = moment ? wallClock(moment.starts_at) : `${props.defaultDay}T12:30`
    end.value = moment ? wallClock(moment.ends_at) : ''
    label.value = moment?.label ?? null
    description.value = moment?.description ?? null
    partId.value = moment?.part_id ?? ''
    tried.value = false
  },
  { immediate: true },
)

const partOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.event.track.agenda.noPart') },
  ...props.parts.map((part, index) => ({
    value: part.id,
    label: partHeading(part, index),
  })),
])

function partHeading(part: TrackPart, index: number): string {
  return part.label ? tr(part.label) : part.title ? tr(part.title) : t('admin.event.track.parts.numbered', { n: index + 1 })
}

const startsAt = computed(() => instantFromWallClock(start.value, props.timezone))
const endsAt = computed(() => instantFromWallClock(end.value, props.timezone))

const endError = computed(() =>
  startsAt.value && endsAt.value && Date.parse(endsAt.value) <= Date.parse(startsAt.value)
    ? t('admin.event.track.moments.endBeforeStart')
    : undefined,
)

function submit(): void {
  tried.value = true
  if (!startsAt.value || !label.value || endError.value) return
  emit('save', {
    part_id: partId.value || null,
    starts_at: startsAt.value,
    ends_at: endsAt.value,
    label: label.value,
    description: description.value,
  })
}
</script>

<template>
  <UiModal
    :open="props.open"
    :title="props.moment ? t('admin.event.track.moments.editTitle') : t('admin.event.track.moments.createTitle')"
    :description="t('admin.event.track.moments.dialogHint')"
    size="lg"
    fullscreen-mobile
    @update:open="(next: boolean) => { if (!next) emit('close') }"
  >
    <form id="track-moment-form" class="space-y-5" novalidate @submit.prevent="submit">
      <UiAlert v-if="props.error" intent="danger" live :message="props.error" />

      <AdminEventsI18nField
        v-model="label"
        :label="t('admin.event.track.moments.label')"
        :hint="t('admin.event.track.moments.labelHint')"
        :error="tried && !label ? t('admin.event.track.moments.labelMissing') : undefined"
        :maxlength="80"
        required
      />

      <div class="grid gap-4 sm:grid-cols-2">
        <UiDatePicker
          v-model="start"
          with-time
          :label="t('admin.event.track.moments.start')"
          :timezone-label="props.zoneLabel"
          :error="tried && !startsAt ? t('admin.event.track.moments.startMissing') : undefined"
          required
        />
        <UiDatePicker
          v-model="end"
          with-time
          :label="t('admin.event.track.moments.end')"
          :hint="t('admin.event.track.moments.endHint')"
          :timezone-label="props.zoneLabel"
          :min="start || undefined"
          :error="endError"
        />
      </div>

      <AdminEventsI18nField
        v-model="description"
        :label="t('admin.event.track.moments.description')"
        :hint="t('admin.event.track.moments.descriptionHint')"
        multiline
        :rows="3"
        :maxlength="400"
      />

      <UiSelect
        v-if="props.parts.length"
        v-model="partId"
        :options="partOptions"
        :label="t('admin.event.track.moments.part')"
        hide-optional
      />
    </form>

    <template #footer>
      <UiButton variant="ghost" :disabled="props.busy" @click="emit('close')">
        {{ t('common.actions.cancel') }}
      </UiButton>
      <UiButton type="submit" form="track-moment-form" :loading="props.busy">
        {{ t('common.actions.save') }}
      </UiButton>
    </template>
  </UiModal>
</template>
