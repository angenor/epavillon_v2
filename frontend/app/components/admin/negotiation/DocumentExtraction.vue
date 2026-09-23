<script setup lang="ts">
import type { ExtractionState } from '~/types/admin-negotiation-documents'
import type { TimeZoneName } from '~/types/shared'

const props = defineProps<{
  extraction: ExtractionState | null
  canPublish: boolean
  fileLocked: boolean
  retrying: boolean
  togglingAsIs: boolean
  /** La relecture périodique a renoncé après plusieurs échecs. */
  pollingStopped: boolean
  timezone: TimeZoneName
}>()

const emit = defineEmits<{
  retry: []
  reload: []
  'update:asIs': [value: boolean]
}>()

const { t } = useI18n()
const { dateTime, zoneLabel } = useDateTime()

const enCours = computed(() => props.extraction?.status === 'pending' || props.extraction?.status === 'extracting')
const relancable = computed(() => props.canPublish && !props.fileLocked && props.extraction !== null && !enCours.value)
</script>

<template>
  <UiCard :title="t('admin.negociations.documents.form.detail.extraction.title')" as="h2">
    <p v-if="!props.extraction" class="text-sm text-text-muted">
      {{ t('admin.negociations.documents.form.detail.extraction.none') }}
    </p>

    <div v-else class="space-y-4">
      <div class="flex flex-wrap items-center gap-3">
        <UiBadge
          :intent="EXTRACTION_STATUS_INTENT[props.extraction.status]"
          :label="t(`admin.negociations.documents.list.extraction.${props.extraction.status}`)"
        />
        <UiBadge
          v-if="props.extraction.serve_as_is"
          intent="info"
          :label="t('admin.negociations.documents.list.extraction.asIs')"
        />
        <span v-if="enCours && !props.pollingStopped" class="flex items-center gap-2 text-sm text-text-muted" aria-live="polite">
          <UiSpinner />
          {{ t('admin.negociations.documents.form.detail.extraction.polling') }}
        </span>
      </div>

      <UiAlert
        v-if="enCours && props.pollingStopped"
        intent="warning"
        live
        :message="t('admin.negociations.documents.form.detail.extraction.stopped')"
      >
        <template #actions>
          <UiButton variant="secondary" size="sm" icon="refresh" @click="emit('reload')">
            {{ t('admin.negociations.documents.form.detail.extraction.reload') }}
          </UiButton>
        </template>
      </UiAlert>

      <dl v-if="props.extraction.status === 'ready'" class="grid gap-x-6 gap-y-2 text-sm sm:grid-cols-2">
        <div v-if="props.extraction.page_count !== null">
          <dt class="text-text-muted">{{ t('admin.negociations.documents.form.detail.extraction.pages') }}</dt>
          <dd>{{ props.extraction.page_count }}</dd>
        </div>
        <div v-if="props.extraction.extracted_at">
          <dt class="text-text-muted">{{ t('admin.negociations.documents.form.detail.extraction.extractedAt') }}</dt>
          <dd>
            {{
              t('admin.negociations.documents.form.detail.zoned', {
                date: dateTime(props.extraction.extracted_at, props.timezone),
                zone: zoneLabel(props.timezone),
              })
            }}
          </dd>
        </div>
        <div v-if="props.extraction.is_reflowable !== null">
          <dt class="text-text-muted">{{ t('admin.negociations.documents.form.detail.extraction.reflowable') }}</dt>
          <dd>
            {{
              props.extraction.is_reflowable
                ? t('admin.negociations.documents.form.detail.extraction.reflowYes')
                : t('admin.negociations.documents.form.detail.extraction.reflowNo')
            }}
          </dd>
        </div>
      </dl>

      <UiAlert
        v-if="props.extraction.status === 'failed'"
        intent="danger"
        :title="t('admin.negociations.documents.form.detail.extraction.failed')"
        :message="props.extraction.failure_reason ?? undefined"
      />

      <UiSwitch
        v-if="props.canPublish"
        :model-value="props.extraction.serve_as_is"
        :label="t('admin.negociations.documents.form.detail.extraction.asIs')"
        :hint="t('admin.negociations.documents.form.detail.extraction.asIsHint')"
        :loading="props.togglingAsIs"
        @update:model-value="(valeur: boolean) => emit('update:asIs', valeur)"
      />

      <UiButton v-if="relancable" variant="secondary" icon="refresh" :loading="props.retrying" @click="emit('retry')">
        {{ t('admin.negociations.documents.form.detail.extraction.retry') }}
      </UiButton>
    </div>
  </UiCard>
</template>
