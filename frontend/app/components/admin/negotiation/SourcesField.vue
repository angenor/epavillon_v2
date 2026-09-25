<script setup lang="ts">
import type { AdminKnowledgeSourceInput } from '~/types/admin-negotiation-savoir'
import type { SelectOption } from '~/types/ui'

// Une source extérieure se reconnaît à `external_title` non nul, même vide pendant la saisie.
const props = withDefaults(
  defineProps<{
    modelValue: AdminKnowledgeSourceInput[]
    /** Titres connus des documents déjà cités, pour ceux que la bibliothèque ne renvoie plus. */
    knownTitles?: Record<string, string>
    readonly?: boolean
    error?: string
  }>(),
  { knownTitles: () => ({}), readonly: false, error: undefined },
)

const emit = defineEmits<{ 'update:modelValue': [value: AdminKnowledgeSourceInput[]] }>()

const { t } = useI18n()
const api = useApi()

const { data: documents, error: documentsEnEchec, refresh } = useAsyncData(
  'admin-negotiation-savoir-documents',
  async () => (await api.adminNegotiationDocuments.documents()).documents,
  { lazy: true, default: () => [] },
)

const optionsDeDocument = computed<SelectOption[]>(() => {
  const connus = new Set(documents.value.map((d) => d.id))
  const absents = props.modelValue
    .map((s) => s.document_id)
    .filter((id): id is string => Boolean(id) && !connus.has(id as string))
    .map((id) => ({ value: id, label: props.knownTitles[id] ?? t('admin-sources-field.unknownDocument') }))
  return [
    ...absents,
    ...documents.value.map((d) => ({
      value: d.id,
      label: d.title,
      description: t('admin-sources-field.version', { version: d.version }),
    })),
  ]
})

const VIDE: AdminKnowledgeSourceInput = {
  document_id: null,
  external_title: null,
  external_url: null,
  section_label: null,
  page_from: null,
  page_to: null,
  quote: null,
}

function remplacer(liste: AdminKnowledgeSourceInput[]): void {
  emit('update:modelValue', liste)
}

function changer(index: number, patch: Partial<AdminKnowledgeSourceInput>): void {
  remplacer(props.modelValue.map((s, i) => (i === index ? { ...s, ...patch } : s)))
}

function ajouter(exterieure: boolean): void {
  remplacer([...props.modelValue, { ...VIDE, external_title: exterieure ? '' : null }])
}

function retirer(index: number): void {
  remplacer(props.modelValue.filter((_, i) => i !== index))
}

function deplacer(index: number, sens: -1 | 1): void {
  const liste = [...props.modelValue]
  const [source] = liste.splice(index, 1)
  if (!source) return
  liste.splice(index + sens, 0, source)
  remplacer(liste)
}

const texte = (valeur: string): string | null => valeur.trim() || null
const page = (valeur: string): number | null => {
  const n = Number.parseInt(valeur, 10)
  return Number.isFinite(n) && n > 0 ? n : null
}
</script>

<template>
  <fieldset class="space-y-4">
    <legend class="text-base font-semibold">{{ t('admin-sources-field.label') }}</legend>
    <p class="max-w-(--measure) text-sm text-text-muted">{{ t('admin-sources-field.hint') }}</p>

    <UiAlert v-if="documentsEnEchec" intent="warning" compact :message="t('admin-sources-field.documentsError')">
      <template #actions>
        <UiButton variant="secondary" size="sm" icon="refresh" @click="refresh()">
          {{ t('common.actions.retry') }}
        </UiButton>
      </template>
    </UiAlert>

    <p v-if="props.modelValue.length === 0" class="text-sm text-text-muted">
      {{ t('admin-sources-field.empty') }}
    </p>

    <ol class="space-y-4">
      <li
        v-for="(source, index) in props.modelValue"
        :key="index"
        class="rounded-md border border-border bg-surface p-4"
      >
        <div class="flex flex-wrap items-center justify-between gap-2">
          <p class="text-sm font-semibold">
            {{
              t(source.external_title === null ? 'admin-sources-field.document' : 'admin-sources-field.external', {
                n: index + 1,
              })
            }}
          </p>
          <div v-if="!props.readonly" class="flex flex-wrap gap-1">
            <UiButton
              variant="ghost"
              size="sm"
              icon="chevron-up"
              icon-only
              :label="t('admin-sources-field.up')"
              :disabled="index === 0"
              @click="deplacer(index, -1)"
            />
            <UiButton
              variant="ghost"
              size="sm"
              icon="chevron-down"
              icon-only
              :label="t('admin-sources-field.down')"
              :disabled="index === props.modelValue.length - 1"
              @click="deplacer(index, 1)"
            />
            <UiButton
              variant="ghost"
              size="sm"
              icon="trash"
              icon-only
              :label="t('admin-sources-field.remove')"
              @click="retirer(index)"
            />
          </div>
        </div>

        <div class="mt-3 grid gap-4 sm:grid-cols-2">
          <UiCombobox
            v-if="source.external_title === null"
            class="sm:col-span-2"
            :model-value="source.document_id ?? ''"
            :label="t('admin-sources-field.documentLabel')"
            :placeholder="t('admin-sources-field.documentPlaceholder')"
            :options="optionsDeDocument"
            :readonly="props.readonly"
            required
            @update:model-value="(v: string) => changer(index, { document_id: v || null })"
          />
          <template v-else>
            <UiInput
              :model-value="source.external_title"
              :label="t('admin-sources-field.externalTitle')"
              :placeholder="t('admin-sources-field.externalTitlePlaceholder')"
              :readonly="props.readonly"
              :maxlength="300"
              required
              @update:model-value="(v: string) => changer(index, { external_title: v })"
            />
            <UiInput
              :model-value="source.external_url ?? ''"
              type="url"
              inputmode="url"
              :label="t('admin-sources-field.externalUrl')"
              :readonly="props.readonly"
              @update:model-value="(v: string) => changer(index, { external_url: texte(v) })"
            />
          </template>

          <UiInput
            :model-value="source.section_label ?? ''"
            :label="t('admin-sources-field.section')"
            :placeholder="t('admin-sources-field.sectionPlaceholder')"
            :readonly="props.readonly"
            :maxlength="200"
            @update:model-value="(v: string) => changer(index, { section_label: texte(v) })"
          />
          <div class="grid grid-cols-2 gap-4">
            <UiInput
              :model-value="source.page_from ?? ''"
              type="number"
              inputmode="numeric"
              :min="1"
              :label="t('admin-sources-field.pageFrom')"
              :readonly="props.readonly"
              @update:model-value="(v: string) => changer(index, { page_from: page(v) })"
            />
            <UiInput
              :model-value="source.page_to ?? ''"
              type="number"
              inputmode="numeric"
              :min="1"
              :label="t('admin-sources-field.pageTo')"
              :readonly="props.readonly"
              @update:model-value="(v: string) => changer(index, { page_to: page(v) })"
            />
          </div>
          <UiTextarea
            class="sm:col-span-2"
            :model-value="source.quote ?? ''"
            :label="t('admin-sources-field.quote')"
            :hint="t('admin-sources-field.quoteHint')"
            :readonly="props.readonly"
            :rows="2"
            auto-grow
            @update:model-value="(v: string) => changer(index, { quote: texte(v) })"
          />
        </div>
      </li>
    </ol>

    <p v-if="props.error" role="alert" class="text-sm font-bold text-danger">{{ props.error }}</p>

    <div v-if="!props.readonly" class="flex flex-wrap gap-3">
      <UiButton variant="secondary" size="sm" icon="document" @click="ajouter(false)">
        {{ t('admin-sources-field.addDocument') }}
      </UiButton>
      <UiButton variant="secondary" size="sm" icon="external-link" @click="ajouter(true)">
        {{ t('admin-sources-field.addExternal') }}
      </UiButton>
    </div>
  </fieldset>
</template>
