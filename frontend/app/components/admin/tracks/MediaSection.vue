<script setup lang="ts">
import type { AttachableRoleRule, AttachedImage, UploadedAsset } from '~/types/media'
import type { I18nText } from '~/types/shared'
import type { TrackDocumentDraft, TrackPageDraft } from '~/utils/track-page'

/**
 * Photo du bandeau et documents de la page d'un fil, par le module Médias
 * (`event.programme_tracks`, rôles `cover` et `document`). Le dépôt part tout
 * de suite ; le rattachement attend l'enregistrement de la page.
 *
 * `MediaImageField` émet l'identifiant puis l'image : on ne suit que l'image,
 * qui porte les deux, sans quoi la seconde émission écraserait la première.
 */

const props = defineProps<{
  modelValue: TrackPageDraft
  rules: AttachableRoleRule[]
  trackId: string
  disabled?: boolean
}>()
const emit = defineEmits<{ 'update:modelValue': [value: TrackPageDraft] }>()

const { t } = useI18n()

const owner = computed(() => trackMediaOwner(props.trackId))
const ruleOf = (role: string) => props.rules.find((rule) => rule.role === role) ?? null
const documentRule = computed(() => ruleOf('document'))

function set<K extends keyof TrackPageDraft>(key: K, value: TrackPageDraft[K]): void {
  emit('update:modelValue', { ...props.modelValue, [key]: value })
}

function setDocuments(next: TrackDocumentDraft[]): void {
  set('documents', next)
}

function addDocument(asset: UploadedAsset): void {
  const filename = asset.original_filename ?? t('admin.event.track.media.untitled')
  setDocuments([
    ...props.modelValue.documents,
    { asset_id: asset.id, filename, url: null, caption: { fr: filename.replace(/\.pdf$/i, '') } },
  ])
}

function setCaption(index: number, caption: I18nText | null): void {
  setDocuments(props.modelValue.documents.map((doc, i) => (i === index ? { ...doc, caption } : doc)))
}
</script>

<template>
  <div class="grid items-start gap-6 lg:grid-cols-2">
    <MediaImageField
      role="cover"
      :label="t('admin.event.track.media.cover')"
      :use="t('admin.event.track.media.coverUse')"
      :rule="ruleOf('cover')"
      :image="props.modelValue.cover"
      :asset-id="props.modelValue.cover_asset_id"
      :owner="owner"
      :disabled="props.disabled"
      @update:image="(next: AttachedImage | null) => emit('update:modelValue', { ...props.modelValue, cover: next, cover_asset_id: next?.asset_id ?? null })"
    />

    <div class="space-y-3">
      <h3 class="text-sm font-semibold">{{ t('admin.event.track.media.documents') }}</h3>
      <p class="text-xs text-text-muted">{{ t('admin.event.track.media.documentsHint') }}</p>

      <ol v-if="props.modelValue.documents.length" class="space-y-3">
        <li
          v-for="(doc, index) in props.modelValue.documents"
          :key="doc.asset_id"
          class="rounded-md border border-border-subtle bg-surface p-3"
        >
          <div class="flex flex-wrap items-center gap-2">
            <UiIcon name="document" class="shrink-0 text-text-muted" />
            <a
              v-if="doc.url"
              :href="doc.url"
              target="_blank"
              rel="noopener noreferrer"
              class="min-w-0 flex-1 truncate text-sm text-accent underline-offset-2 hover:underline"
            >{{ doc.filename }}</a>
            <span v-else class="min-w-0 flex-1 truncate text-sm">{{ doc.filename }}</span>
            <UiMoveButtons
              :item="doc.filename"
              :first="index === 0"
              :last="index === props.modelValue.documents.length - 1"
              :disabled="props.disabled"
              @up="setDocuments(moved(props.modelValue.documents, index, -1))"
              @down="setDocuments(moved(props.modelValue.documents, index, 1))"
            />
            <UiButton
              variant="ghost"
              icon="trash"
              icon-only
              :label="t('admin.event.track.media.removeDocument', { name: doc.filename })"
              :disabled="props.disabled"
              @click="setDocuments(props.modelValue.documents.filter((_, i) => i !== index))"
            />
          </div>
          <AdminEventsI18nField
            class="mt-3"
            :model-value="doc.caption"
            :label="t('admin.event.track.media.caption')"
            :hint="t('admin.event.track.media.captionHint')"
            :maxlength="80"
            :disabled="props.disabled"
            @update:model-value="(next: I18nText | null) => setCaption(index, next)"
          />
        </li>
      </ol>

      <MediaFileField
        :label="t('admin.event.track.media.addDocument')"
        :types="documentRule?.allowed_mime_prefixes.length ? documentRule.allowed_mime_prefixes : ['application/pdf']"
        :asset-id="null"
        :owner="owner"
        :max-byte-size="documentRule?.max_byte_size ?? null"
        :disabled="props.disabled"
        @uploaded="addDocument"
      />
    </div>
  </div>
</template>
