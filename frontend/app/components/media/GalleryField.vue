<script setup lang="ts">
import type { AssetSources, AssetStatus, AttachableRoleRule, AttachmentRole } from '~/types/media'
import type { AssetId, I18nText, Url, Uuid } from '~/types/shared'

/**
 * UNE GALERIE : plusieurs photos d'un rôle multiple, ordonnées, décrites.
 *
 * Le dépôt reprend celui de `MediaImageField` (même composable, même éditeur,
 * texte alternatif saisi à côté de l'aperçu), sans forme imposée : une galerie
 * accepte le cadrage de la photo. La légende va sur le RATTACHEMENT, jamais sur
 * l'objet : la vider l'efface vraiment.
 *
 * Rien n'est rattaché avant « Enregistrer » : l'ordre et les textes partent en
 * un lot, que l'API applique en une transaction (le rôle est vidé puis regarni).
 * Une photo déposée puis abandonnée reste un objet orphelin, que la purge reprend.
 */

interface Owner {
  schema: string
  table: string
  id: Uuid
}

interface GalleryItem {
  asset_id: AssetId
  url: Url
  width: number | null
  height: number | null
  sources: AssetSources
  status: AssetStatus
  alt_text: I18nText | null
  caption: I18nText | null
}

const props = withDefaults(
  defineProps<{
    owner: Owner
    label: string
    use?: string
    role?: AttachmentRole
    editable?: boolean
  }>(),
  { role: 'gallery', editable: true, use: undefined },
)

const { t, locale } = useI18n()
const api = useApi()

const rule = ref<AttachableRoleRule | null>(null)
const items = ref<GalleryItem[]>([])
const baseline = ref('')
const loading = ref(false)
const loadError = ref<string | null>(null)
const forbidden = ref(false)
const saving = ref(false)
const saveError = ref<string | null>(null)
const saved = ref(false)

const {
  enCours: uploading,
  echec: uploadError,
  accept,
  accepte: accepted,
  deposer,
} = useDepotMedia({ motifs: () => rule.value?.allowed_mime_prefixes ?? [], repli: 'image/*' })

function snapshot(list: GalleryItem[]): string {
  return JSON.stringify(list.map((item) => [item.asset_id, item.alt_text, item.caption]))
}

async function load(): Promise<void> {
  loading.value = true
  loadError.value = null
  forbidden.value = false
  try {
    const [rules, attached] = await Promise.all([
      api.media.roles(props.owner.schema, props.owner.table),
      api.media.attached(props.owner, props.role),
    ])
    rule.value = rules.find((entry) => entry.role === props.role) ?? null
    items.value = attached
      .filter((entry) => entry.role === props.role)
      .map((entry) => ({
        asset_id: entry.asset_id,
        url: entry.url,
        width: entry.width,
        height: entry.height,
        sources: entry.sources,
        status: entry.status,
        alt_text: entry.alt_text,
        caption: entry.caption,
      }))
    baseline.value = snapshot(items.value)
  } catch (failure) {
    items.value = []
    // L'API refuse un hors-périmètre comme un inexistant : un 404 se lit donc « pas à vous ».
    if (isForbiddenError(failure) || (failure instanceof ApiRequestError && failure.status === 404)) {
      forbidden.value = true
    } else {
      loadError.value = apiErrorMessage(failure, t)
    }
  } finally {
    loading.value = false
  }
}

watch(
  () => [props.owner.schema, props.owner.table, props.owner.id, props.role] as const,
  () => {
    saveError.value = null
    saved.value = false
    void load()
  },
  { immediate: true },
)

const isDirty = computed(() => snapshot(items.value) !== baseline.value)
const missingAlt = computed(() => items.value.filter((item) => !item.alt_text?.fr).length)
const canSave = computed(() => props.editable && isDirty.value && missingAlt.value === 0 && !uploading.value)
const maxLabel = computed(() =>
  rule.value?.max_byte_size ? formatByteSize(rule.value.max_byte_size, locale.value) : null,
)

function move(index: number, offset: -1 | 1): void {
  const target = index + offset
  if (target < 0 || target >= items.value.length) return
  const next = [...items.value]
  const [moved] = next.splice(index, 1)
  if (!moved) return
  next.splice(target, 0, moved)
  items.value = next
  saved.value = false
}

function remove(index: number): void {
  items.value = items.value.filter((_, position) => position !== index)
  saved.value = false
}

function setText(index: number, field: 'alt_text' | 'caption', value: I18nText | null): void {
  const item = items.value[index]
  if (!item) return
  items.value = items.value.map((entry, position) => (position === index ? { ...entry, [field]: value } : entry))
  saved.value = false
}

// ---------------------------------------------------------------------------
// Ajout d'une photo
// ---------------------------------------------------------------------------

const input = ref<HTMLInputElement | null>(null)
const chosen = ref<File | null>(null)
const newAlt = ref<I18nText | null>(null)
const newCaption = ref<I18nText | null>(null)

function pick(): void {
  uploadError.value = null
  input.value?.click()
}

function onFileChosen(event: Event): void {
  const target = event.target as HTMLInputElement
  const file = target.files?.[0] ?? null
  target.value = ''
  if (!file) return
  if (!accepted(file.type)) {
    uploadError.value = t('gallery-field.errors.mimeRejected')
    return
  }
  uploadError.value = null
  newAlt.value = null
  newCaption.value = null
  chosen.value = file
}

function cancelEditing(): void {
  chosen.value = null
  newAlt.value = null
  newCaption.value = null
}

async function upload(result: { blob: Blob; width: number; height: number; mimeType: string; filename: string }): Promise<void> {
  const description = newAlt.value
  if (!description?.fr) return
  const asset = await deposer({
    file: result.blob,
    filename: result.filename,
    mimeType: result.mimeType,
    altText: description,
    role: props.role,
    ownerSchema: props.owner.schema,
    ownerTable: props.owner.table,
    ownerId: props.owner.id,
  })
  if (!asset) return
  if (!items.value.some((item) => item.asset_id === asset.id)) {
    items.value = [
      ...items.value,
      {
        asset_id: asset.id,
        url: asset.url,
        width: result.width,
        height: result.height,
        sources: asset.sources,
        status: asset.status,
        alt_text: description,
        caption: newCaption.value,
      },
    ]
  }
  saved.value = false
  cancelEditing()
}

// ---------------------------------------------------------------------------
// Enregistrement
// ---------------------------------------------------------------------------

async function save(): Promise<void> {
  if (!canSave.value) return
  saving.value = true
  saveError.value = null
  try {
    await api.media.attach({
      owner_schema: props.owner.schema,
      owner_table: props.owner.table,
      owner_id: props.owner.id,
      assignments:
        items.value.length > 0
          ? items.value.map((item) => ({
              role: props.role,
              asset_id: item.asset_id,
              alt_text_override: item.alt_text,
              caption_override: item.caption,
            }))
          : [{ role: props.role, asset_id: null }],
    })
    await load()
    saved.value = true
  } catch (failure) {
    saveError.value = apiErrorMessage(failure, t)
  } finally {
    saving.value = false
  }
}

function previewOf(item: GalleryItem) {
  return {
    asset_id: item.asset_id,
    url: item.url,
    width: item.width,
    height: item.height,
    alt_text: item.alt_text ?? { fr: '' },
    caption: null,
    credit: null,
    sources: item.sources,
  }
}
</script>

<template>
  <section class="space-y-3 rounded-lg border border-border p-3" :aria-label="props.label">
    <header>
      <h4 class="text-sm font-semibold text-text">{{ props.label }}</h4>
      <p class="mt-0.5 text-xs text-text-subtle">
        <template v-if="props.use">{{ props.use }}</template>
        <template v-if="maxLabel"> · {{ t('gallery-field.max', { max: maxLabel }) }}</template>
      </p>
    </header>

    <div v-if="loading && items.length === 0" class="space-y-2" aria-busy="true">
      <UiSkeletonLoader variant="text" :lines="3" />
    </div>

    <p v-else-if="forbidden" class="text-sm text-text-muted">{{ t('gallery-field.forbidden') }}</p>

    <UiAlert v-else-if="loadError" intent="danger" compact :message="loadError">
      <template #actions>
        <UiButton variant="link" @click="load">{{ t('gallery-field.retry') }}</UiButton>
      </template>
    </UiAlert>

    <template v-else>
      <p v-if="items.length === 0" class="text-sm text-text-muted">{{ t('gallery-field.empty') }}</p>

      <ol v-else class="space-y-4">
        <li
          v-for="(item, index) in items"
          :key="item.asset_id"
          class="space-y-2 border-b border-border-subtle pb-4 last:border-b-0 last:pb-0"
        >
          <div class="flex items-start gap-3">
            <UiImage :image="previewOf(item)" ratio="4 / 3" rounded="rounded-md" class="w-28 shrink-0" sizes="7rem" />
            <div class="min-w-0 flex-1 space-y-1">
              <p class="text-xs font-medium text-text-muted">
                {{ t('gallery-field.position', { index: index + 1, total: items.length }) }}
              </p>
              <UiBadge
                v-if="item.status !== 'ready'"
                size="sm"
                intent="warning"
                :label="t('gallery-field.processing')"
              />
              <div v-if="props.editable" class="flex flex-wrap gap-1">
                <UiButton
                  variant="ghost"
                  icon="chevron-up"
                  icon-only
                  :label="t('gallery-field.actions.up')"
                  :disabled="index === 0 || saving"
                  @click="move(index, -1)"
                />
                <UiButton
                  variant="ghost"
                  icon="chevron-down"
                  icon-only
                  :label="t('gallery-field.actions.down')"
                  :disabled="index === items.length - 1 || saving"
                  @click="move(index, 1)"
                />
                <UiButton
                  variant="ghost"
                  icon="trash"
                  icon-only
                  :label="t('gallery-field.actions.remove')"
                  :disabled="saving"
                  @click="remove(index)"
                />
              </div>
            </div>
          </div>
          <AdminEventsI18nField
            :model-value="item.alt_text"
            :label="t('gallery-field.altText.label')"
            :hint="item.alt_text?.fr ? undefined : t('gallery-field.altText.required')"
            required
            :maxlength="300"
            :disabled="!props.editable || saving"
            @update:model-value="(value: I18nText | null) => setText(index, 'alt_text', value)"
          />
          <AdminEventsI18nField
            :model-value="item.caption"
            :label="t('gallery-field.caption.label')"
            :maxlength="300"
            :disabled="!props.editable || saving"
            @update:model-value="(value: I18nText | null) => setText(index, 'caption', value)"
          />
        </li>
      </ol>

      <div v-if="props.editable" class="flex flex-wrap items-center gap-2 border-t border-border-subtle pt-3">
        <UiButton variant="secondary" icon="upload" :disabled="uploading || saving" @click="pick">
          {{ t('gallery-field.actions.add') }}
        </UiButton>
        <UiButton icon="check" :loading="saving" :disabled="!canSave" @click="save">
          {{ t('gallery-field.actions.save') }}
        </UiButton>
        <p v-if="isDirty && missingAlt > 0" class="w-full text-xs text-text-muted">
          {{ t('gallery-field.missingAlt', missingAlt) }}
        </p>
        <p v-else-if="isDirty" class="w-full text-xs text-text-muted">{{ t('gallery-field.unsaved') }}</p>
        <p v-else-if="saved" class="w-full text-xs text-success" role="status">{{ t('gallery-field.saved') }}</p>
      </div>

      <UiAlert v-if="uploadError" intent="danger" live compact :message="uploadError" />
      <UiAlert v-if="saveError" intent="danger" live compact :message="saveError" />
    </template>

    <input ref="input" type="file" class="sr-only" :accept="accept" :aria-label="t('gallery-field.actions.add')" @change="onFileChosen">

    <UiImageEditor
      v-if="chosen"
      :file="chosen"
      :title="t('gallery-field.editor.title')"
      :description="t('gallery-field.editor.description')"
      :max-byte-size="rule?.max_byte_size ?? null"
      :busy="uploading"
      :apply-disabled="!newAlt?.fr"
      :apply-hint="t('gallery-field.altText.required')"
      :apply-label="t('gallery-field.editor.apply')"
      @apply="upload"
      @cancel="cancelEditing"
    >
      <template #aside>
        <AdminEventsI18nField
          v-model="newAlt"
          :label="t('gallery-field.altText.label')"
          :hint="t('gallery-field.altText.hint')"
          :disabled="uploading"
          required
          multiline
          :rows="3"
          :maxlength="300"
        />
        <AdminEventsI18nField
          v-model="newCaption"
          :label="t('gallery-field.caption.label')"
          :hint="t('gallery-field.caption.hint')"
          :disabled="uploading"
          :maxlength="300"
        />
      </template>
    </UiImageEditor>
  </section>
</template>
