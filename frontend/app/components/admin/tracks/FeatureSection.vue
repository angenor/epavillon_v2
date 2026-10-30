<script setup lang="ts">
import type { AttachableRoleRule, AttachedImage } from '~/types/media'
import type { I18nText } from '~/types/shared'
import type { TrackPageDraft } from '~/utils/track-page'

/**
 * L'encart mis en avant : le Guide des négociations pour l'atelier. Un seul par
 * fil. Le titre porte l'encart (sans lui, rien ne s'affiche) ; le lien se
 * donne avec son libellé ou pas du tout (`ck_programme_tracks_feature_link`).
 * Son visuel est le rôle `thumbnail` du fil, sans forme imposée.
 */

const props = defineProps<{
  modelValue: TrackPageDraft
  rules: AttachableRoleRule[]
  trackId: string
  disabled?: boolean
}>()
const emit = defineEmits<{ 'update:modelValue': [value: TrackPageDraft] }>()

const { t } = useI18n()

function set<K extends keyof TrackPageDraft>(key: K, value: TrackPageDraft[K]): void {
  emit('update:modelValue', { ...props.modelValue, [key]: value })
}

const titleMissing = computed(() => {
  const d = props.modelValue
  return !d.feature_title && Boolean(d.feature_kicker || d.feature_body || d.feature_link_url.trim())
})

const urlError = computed(() => {
  const url = props.modelValue.feature_link_url.trim()
  if (!url) return props.modelValue.feature_link_label ? t('admin.event.track.feature.urlMissing') : undefined
  return /^(https?:\/\/|\/)\S*$/.test(url) ? undefined : t('admin.event.track.feature.urlInvalid')
})

const labelError = computed(() =>
  props.modelValue.feature_link_url.trim() && !props.modelValue.feature_link_label
    ? t('admin.event.track.feature.labelMissing')
    : undefined,
)
</script>

<template>
  <div class="grid items-start gap-6 lg:grid-cols-[minmax(0,1fr)_18rem]">
    <div class="space-y-5">
      <AdminEventsI18nField
        :model-value="props.modelValue.feature_kicker"
        :label="t('admin.event.track.feature.kicker')"
        :hint="t('admin.event.track.feature.kickerHint')"
        :maxlength="80"
        :disabled="props.disabled"
        @update:model-value="(next: I18nText | null) => set('feature_kicker', next)"
      />
      <AdminEventsI18nField
        :model-value="props.modelValue.feature_title"
        :label="t('admin.event.track.feature.title')"
        :error="titleMissing ? t('admin.event.track.feature.titleMissing') : undefined"
        :maxlength="160"
        :disabled="props.disabled"
        @update:model-value="(next: I18nText | null) => set('feature_title', next)"
      />
      <AdminEventsI18nField
        :model-value="props.modelValue.feature_body"
        :label="t('admin.event.track.feature.body')"
        multiline
        :rows="4"
        :disabled="props.disabled"
        @update:model-value="(next: I18nText | null) => set('feature_body', next)"
      />
      <div class="grid gap-4 sm:grid-cols-2">
        <AdminEventsI18nField
          :model-value="props.modelValue.feature_link_label"
          :label="t('admin.event.track.feature.linkLabel')"
          :error="labelError"
          :maxlength="60"
          :disabled="props.disabled"
          @update:model-value="(next: I18nText | null) => set('feature_link_label', next)"
        />
        <!-- Aligné sur le champ voisin, qui porte ses onglets de langue au-dessus. -->
        <UiInput
          class="sm:mt-12"
          :model-value="props.modelValue.feature_link_url"
          type="url"
          :label="t('admin.event.track.feature.linkUrl')"
          :hint="t('admin.event.track.feature.linkUrlHint')"
          :error="urlError"
          :maxlength="2048"
          :disabled="props.disabled"
          @update:model-value="(next: string) => set('feature_link_url', next)"
        />
      </div>
    </div>

    <MediaImageField
      role="thumbnail"
      :label="t('admin.event.track.feature.visual')"
      :use="t('admin.event.track.feature.visualUse')"
      :rule="props.rules.find((rule) => rule.role === 'thumbnail') ?? null"
      :image="props.modelValue.visual"
      :asset-id="props.modelValue.visual_asset_id"
      :owner="trackMediaOwner(props.trackId)"
      :disabled="props.disabled"
      @update:image="(next: AttachedImage | null) => emit('update:modelValue', { ...props.modelValue, visual: next, visual_asset_id: next?.asset_id ?? null })"
    />
  </div>
</template>
