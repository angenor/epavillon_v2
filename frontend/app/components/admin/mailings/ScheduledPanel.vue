<script setup lang="ts">
import type {
  ScheduledMailBatch,
  ScheduledMailLocale,
  ScheduledMailPreview,
  ScheduledMailText,
  ScheduledMailTexts,
} from '~/types/admin-scheduled-mails'
import type { EventId } from '~/types/shared'
import type { TabItem } from '~/types/ui'

/**
 * LE PANNEAU D'UN ENVOI PROGRAMMÉ (04/11) : à gauche, l'objet et le message,
 * pour le type entier ou pour cet envoi seulement, en français ou en anglais ;
 * à droite, ce qui partira, rendu par l'API avec le code de l'envoi réel pour
 * le premier destinataire, valeurs surlignées.
 *
 * Les deux portées et les deux langues gardent chacune leur brouillon : passer
 * de l'une à l'autre ne perd rien de ce qu'on a écrit.
 */
type Scope = 'type' | 'batch'

const props = defineProps<{
  batchKey: string
  eventId: EventId
  initialScope: Scope
  /** « 06/11 à 09:00 », déjà dans le fuseau de l'édition. */
  when: string
  label: string
}>()
const emit = defineEmits<{ saved: []; close: [] }>()

const { t } = useI18n()
const api = useApi()

const detail = ref<ScheduledMailBatch | null>(null)
const loadError = ref<string | null>(null)
const loading = ref(true)

const scope = ref<Scope>(props.initialScope)
const language = ref<ScheduledMailLocale>('fr')
const drafts = reactive<Record<Scope, ScheduledMailTexts | null>>({ type: null, batch: null })

function copy(texts: ScheduledMailTexts): ScheduledMailTexts {
  return { fr: { ...texts.fr }, en: { ...texts.en } }
}

function adopt(fresh: ScheduledMailBatch, resetDrafts: boolean): void {
  detail.value = fresh
  if (resetDrafts || !drafts.type) drafts.type = copy(fresh.type_text)
  if (resetDrafts || !drafts.batch) drafts.batch = copy(fresh.own_text ?? fresh.type_text)
}

async function load(): Promise<void> {
  loading.value = true
  loadError.value = null
  try {
    const fresh = await api.adminMailings.scheduled.batch(props.batchKey)
    if (!fresh) {
      detail.value = null
      loadError.value = t('admin-mailing-scheduled-panel.gone')
      return
    }
    adopt(fresh, true)
    language.value = fresh.preview_locale
    scope.value = props.initialScope
  } catch (thrown) {
    loadError.value = apiErrorMessage(thrown, t)
  } finally {
    loading.value = false
  }
}

watch(() => [props.batchKey, props.initialScope] as const, load, { immediate: true })

const current = computed<ScheduledMailText | null>(() => drafts[scope.value]?.[language.value] ?? null)

function update(field: keyof ScheduledMailText, value: string): void {
  const texts = drafts[scope.value]
  if (texts) texts[language.value] = { ...texts[language.value], [field]: value }
}

const languageTabs = computed<TabItem[]>(() =>
  (['fr', 'en'] as const).map((value) => ({ value, label: t(`admin-mailing-scheduled-panel.language.${value}`) })),
)

/** `{{prenom}}` : les accolades écrites ainsi pour ne pas être lues par le gabarit Vue. */
function token(name: string): string {
  return `${'{'.repeat(2)}${name}${'}'.repeat(2)}`
}

const actionLabel = computed(() =>
  t(`admin-mailing-scheduled-panel.button.${detail.value?.batch.kind === 'day' ? 'day' : 'activity'}`),
)

// --- Ce qui partira -----------------------------------------------------------
const preview = ref<ScheduledMailPreview | null>(null)
const previewError = ref<string | null>(null)
let previewTimer: ReturnType<typeof setTimeout> | null = null
let previewSeq = 0

async function renderPreview(): Promise<void> {
  const text = current.value
  if (!text || !detail.value) return
  const seq = ++previewSeq
  try {
    const fresh = await api.adminMailings.scheduled.preview(props.batchKey, { locale: language.value, ...text })
    if (seq !== previewSeq) return
    preview.value = fresh
    previewError.value = null
  } catch (thrown) {
    if (seq === previewSeq) previewError.value = apiErrorMessage(thrown, t)
  }
}

watch(
  [current, language, () => detail.value?.batch.key],
  () => {
    if (previewTimer) clearTimeout(previewTimer)
    previewTimer = setTimeout(renderPreview, 400)
  },
  { deep: true, immediate: true },
)
onBeforeUnmount(() => {
  if (previewTimer) clearTimeout(previewTimer)
})

// --- Enregistrer --------------------------------------------------------------
const saving = ref(false)
const saveError = ref<string | null>(null)
const notice = ref<string | null>(null)

async function save(): Promise<void> {
  const texts = drafts[scope.value]
  if (!texts || !detail.value) return
  saving.value = true
  saveError.value = null
  notice.value = null
  try {
    if (scope.value === 'batch') {
      const fresh = await api.adminMailings.scheduled.saveBatchText(props.batchKey, texts)
      if (fresh) adopt(fresh, false)
    } else {
      await api.adminMailings.scheduled.saveTypeText(detail.value.batch.type_key, { event_id: props.eventId, ...texts })
      const fresh = await api.adminMailings.scheduled.batch(props.batchKey)
      if (fresh) adopt(fresh, false)
    }
    notice.value = t('admin-mailing-scheduled-panel.saved')
    emit('saved')
  } catch (thrown) {
    saveError.value = apiErrorMessage(thrown, t)
  } finally {
    saving.value = false
  }
}

async function resetToType(): Promise<void> {
  saving.value = true
  saveError.value = null
  notice.value = null
  try {
    const fresh = await api.adminMailings.scheduled.resetBatchText(props.batchKey)
    if (fresh) {
      detail.value = fresh
      drafts.batch = copy(fresh.type_text)
    }
    notice.value = t('admin-mailing-scheduled-panel.resetDone')
    emit('saved')
  } catch (thrown) {
    saveError.value = apiErrorMessage(thrown, t)
  } finally {
    saving.value = false
  }
}

const recipients = computed(() => detail.value?.batch.recipients ?? 0)
const typeRecipients = computed(() => detail.value?.type_recipients ?? 0)
</script>

<template>
  <section class="grid gap-6 lg:grid-cols-2" :aria-label="t('admin-mailing-scheduled-panel.region')">
    <UiCard class="min-w-0">
      <div class="flex items-start justify-between gap-3">
        <div class="min-w-0">
          <h2 class="text-lg font-semibold text-heading">{{ props.label }}</h2>
          <p class="mt-1 text-sm text-text-muted">
            {{ t('admin-mailing-scheduled-panel.context', { when: props.when, count: recipients }, recipients) }}
          </p>
        </div>
        <UiButton variant="ghost" icon="close" icon-only :label="t('admin-mailing-scheduled-panel.close')" @click="emit('close')" />
      </div>

      <UiLoadingState v-if="loading" class="mt-4" variant="text" :lines="6" />
      <UiAlert v-else-if="loadError" class="mt-4" intent="warning" :message="loadError" />

      <template v-else-if="detail && current">
        <div
          class="mt-4 flex gap-1 rounded-md bg-surface-sunken p-1"
          role="group"
          :aria-label="t('admin-mailing-scheduled-panel.scope.label')"
        >
          <button
            v-for="value in (['type', 'batch'] as const)"
            :key="value"
            type="button"
            class="min-h-(--target-min) flex-1 cursor-pointer rounded px-3 text-sm font-semibold transition-colors"
            :class="scope === value ? 'bg-surface-raised text-text shadow-xs' : 'text-text-secondary hover:text-text'"
            :aria-pressed="scope === value"
            @click="scope = value"
          >
            {{
              value === 'type'
                ? t('admin-mailing-scheduled-panel.scope.type', { count: typeRecipients }, typeRecipients)
                : t('admin-mailing-scheduled-panel.scope.batch', { count: recipients })
            }}
          </button>
        </div>
        <p class="mt-2 text-sm text-text-muted">
          <template v-if="scope === 'type'">
            {{ t(detail.type_text_is_custom ? 'admin-mailing-scheduled-panel.typeCustom' : 'admin-mailing-scheduled-panel.typeDefault') }}
            <template v-if="detail.own_text"> {{ t('admin-mailing-scheduled-panel.typeSpares') }}</template>
          </template>
          <template v-else>
            {{ t(detail.own_text ? 'admin-mailing-scheduled-panel.batchOwn' : 'admin-mailing-scheduled-panel.batchFollows') }}
          </template>
        </p>

        <UiTabs
          class="mt-4"
          :model-value="language"
          :items="languageTabs"
          :label="t('admin-mailing-scheduled-panel.language.label')"
          @update:model-value="language = $event as ScheduledMailLocale"
        />

        <div class="mt-4 grid gap-4">
          <UiInput
            :model-value="current.subject"
            :label="t('admin-mailing-scheduled-panel.subject')"
            block
            required
            :maxlength="250"
            @update:model-value="update('subject', $event)"
          />
          <UiTextarea
            :model-value="current.body"
            :label="t('admin-mailing-scheduled-panel.body')"
            :hint="t('admin-mailing-scheduled-panel.bodyHint')"
            block
            required
            :rows="9"
            auto-grow
            @update:model-value="update('body', $event)"
          />
        </div>

        <ul class="mt-3 flex flex-wrap gap-1.5" :aria-label="t('admin-mailing-scheduled-panel.variables')">
          <li
            v-for="name in detail.variables"
            :key="name"
            class="rounded-md bg-surface-sunken px-2 py-0.5 font-mono text-xs text-text-secondary"
          >
            <span v-text="token(name)" />
          </li>
          <li class="rounded-md bg-surface-sunken px-2 py-0.5 text-xs text-text-secondary">{{ actionLabel }}</li>
        </ul>

        <UiAlert v-if="saveError" class="mt-4" intent="danger" live :message="saveError" />
        <UiAlert v-else-if="notice" class="mt-4" intent="success" live :message="notice" />

        <div class="mt-4 flex flex-wrap justify-end gap-2">
          <UiButton
            v-if="scope === 'batch' && detail.own_text"
            variant="secondary"
            :disabled="saving"
            @click="resetToType"
          >
            {{ t('admin-mailing-scheduled-panel.reset') }}
          </UiButton>
          <UiButton :loading="saving" @click="save">
            {{
              scope === 'batch'
                ? t('admin-mailing-scheduled-panel.saveBatch')
                : t('admin-mailing-scheduled-panel.saveType', { count: typeRecipients }, typeRecipients)
            }}
          </UiButton>
        </div>
        <p class="mt-3 text-xs text-text-muted">{{ t('admin-mailing-scheduled-panel.help') }}</p>
      </template>
    </UiCard>

    <UiCard class="min-w-0">
      <h2 class="text-lg font-semibold text-heading">{{ t('admin-mailing-scheduled-panel.preview.title') }}</h2>
      <p class="mt-1 text-sm text-text-muted">{{ t('admin-mailing-scheduled-panel.preview.context') }}</p>
      <UiAlert v-if="previewError" class="mt-4" intent="danger" compact :message="previewError" />
      <template v-if="preview">
        <dl class="mt-4 grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 rounded-t-md border border-b-0 border-border bg-surface-sunken px-4 py-3 text-sm">
          <dt class="text-text-subtle">{{ t('admin-mailing-scheduled-panel.preview.subject') }}</dt>
          <dd class="min-w-0 font-semibold text-text wrap-anywhere">{{ preview.subject }}</dd>
          <dt class="text-text-subtle">{{ t('admin-mailing-scheduled-panel.preview.to') }}</dt>
          <dd class="min-w-0 text-text">{{ t('admin-mailing-scheduled-panel.preview.toValue', { count: recipients }, recipients) }}</dd>
        </dl>
        <iframe
          class="h-128 w-full rounded-b-md border border-border bg-white"
          :title="t('admin-mailing-scheduled-panel.preview.title')"
          sandbox=""
          :srcdoc="preview.html"
        />
      </template>
      <UiLoadingState v-else-if="!previewError" class="mt-4" variant="text" :lines="8" />
    </UiCard>
  </section>
</template>
