<script setup lang="ts">
import type { TrackPart, TrackPartPayload } from '~/types/event/track-page'
import type { I18nText } from '~/types/shared'

/** Ajouter ou modifier une partie du déroulé : nom court, intitulé, résumé. */

const props = defineProps<{
  open: boolean
  /** Nulle : ajout. */
  part: TrackPart | null
  sortOrder: number
  busy?: boolean
  error?: string | null
}>()
const emit = defineEmits<{ close: []; save: [payload: TrackPartPayload] }>()

const { t } = useI18n()

const label = ref<I18nText | null>(null)
const title = ref<I18nText | null>(null)
const summary = ref<I18nText | null>(null)
const tried = ref(false)

watch(
  () => [props.open, props.part] as const,
  ([open, part]) => {
    if (!open) return
    label.value = part?.label ?? null
    title.value = part?.title ?? null
    summary.value = part?.summary ?? null
    tried.value = false
  },
  { immediate: true },
)

const headingMissing = computed(() => !label.value && !title.value)

function submit(): void {
  tried.value = true
  if (headingMissing.value) return
  emit('save', {
    label: label.value,
    title: title.value,
    summary: summary.value,
    sort_order: props.part?.sort_order ?? props.sortOrder,
  })
}
</script>

<template>
  <UiModal
    :open="props.open"
    :title="props.part ? t('admin.event.track.parts.editTitle') : t('admin.event.track.parts.createTitle')"
    :description="t('admin.event.track.parts.dialogHint')"
    size="lg"
    fullscreen-mobile
    @update:open="(next: boolean) => { if (!next) emit('close') }"
  >
    <form id="track-part-form" class="space-y-5" novalidate @submit.prevent="submit">
      <UiAlert v-if="props.error" intent="danger" live :message="props.error" />
      <AdminEventsI18nField
        v-model="label"
        :label="t('admin.event.track.parts.label')"
        :hint="t('admin.event.track.parts.labelHint')"
        :error="tried && headingMissing ? t('admin.event.track.parts.headingMissing') : undefined"
        :maxlength="60"
      />
      <AdminEventsI18nField v-model="title" :label="t('admin.event.track.parts.title')" :maxlength="200" />
      <AdminEventsI18nField
        v-model="summary"
        :label="t('admin.event.track.parts.summary')"
        :hint="t('admin.event.track.parts.summaryHint')"
        :maxlength="160"
      />
    </form>

    <template #footer>
      <UiButton variant="ghost" :disabled="props.busy" @click="emit('close')">
        {{ t('common.actions.cancel') }}
      </UiButton>
      <UiButton type="submit" form="track-part-form" :loading="props.busy">
        {{ t('common.actions.save') }}
      </UiButton>
    </template>
  </UiModal>
</template>
