<script setup lang="ts">
import type { AdminCorrectionNote, AdminCorrectionNoteList, AdminPreviewPage } from '~/types/admin-negotiation-documents'
import type { TimeZoneName, Uuid } from '~/types/shared'

/**
 * Les notes de correction d'un document, à côté de sa forme lisible : l'expert choisit
 * la page en feuilletant, et le passage en le sélectionnant dans le texte. Une note ne
 * s'efface jamais : retirée, elle reste ici avec qui l'a posée et qui l'a retirée.
 */
interface Props {
  documentId: Uuid
  page: AdminPreviewPage
  labels: Map<number, string>
  /** Le texte de la page, où l'extrait se sélectionne. */
  source: HTMLElement | null
}

const props = defineProps<Props>()
defineEmits<{ go: [index: number] }>()

const { t } = useI18n()
const api = useApi()
const { dateTime, zoneLabel } = useDateTime()

const timezone = computed<TimeZoneName>(
  () => (Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC') as TimeZoneName,
)
const zoned = (instant: string): string =>
  t('admin.negociations.documents.preview.notes.zoned', {
    date: dateTime(instant, timezone.value),
    zone: zoneLabel(timezone.value),
  })

// Sans `await` : le panneau paraît après l'aperçu, hors de la suspension de la page.
const { data: list, error: listError, status, refresh } = useAsyncData<AdminCorrectionNoteList | null>(
  () => `admin-negotiation-document-notes-${props.documentId}`,
  () => api.adminNegotiationDocuments.notes(props.documentId),
  { default: () => null, lazy: true },
)

const notes = computed<AdminCorrectionNote[]>(() => list.value?.notes ?? [])
const here = computed(() => notes.value.filter((note) => note.page_index === props.page.index))
const elsewhere = computed(() => notes.value.filter((note) => note.page_index !== props.page.index))
const canPost = computed(() => list.value?.can_post ?? false)
const canWithdraw = computed(() => list.value?.can_withdraw ?? false)

// --- L'extrait visé ---------------------------------------------------------

// Un début d'extrait suffit à retrouver le passage ; il reste une sous-chaîne du texte.
const PASSAGE_MAX = 280
const passage = ref<string | null>(null)

function shorten(text: string): string {
  if (text.length <= PASSAGE_MAX) return text
  const cut = text.lastIndexOf(' ', PASSAGE_MAX)
  return text.slice(0, cut > 0 ? cut : PASSAGE_MAX)
}

// Seule une sélection faite dans le texte de la page compte ; la perdre ne l'efface pas.
function onSelectionChange(): void {
  const selection = window.getSelection()
  if (!props.source || !selection || selection.isCollapsed || selection.rangeCount === 0) return
  if (!props.source.contains(selection.getRangeAt(0).commonAncestorContainer)) return
  const text = selection.toString().trim()
  if (text) passage.value = shorten(text)
}

onMounted(() => document.addEventListener('selectionchange', onSelectionChange))
onBeforeUnmount(() => document.removeEventListener('selectionchange', onSelectionChange))

// --- Poser ------------------------------------------------------------------

const bodyFr = ref('')
const bodyEn = ref('')
const posting = ref(false)
const postError = ref<string | null>(null)
const posted = ref(false)

watch(
  () => props.page.index,
  () => {
    passage.value = null
    postError.value = null
    posted.value = false
  },
)

async function post(): Promise<void> {
  if (posting.value) return
  postError.value = null
  posted.value = false
  if (!bodyFr.value.trim()) {
    postError.value = t('admin.negociations.documents.preview.notes.form.frRequired')
    return
  }
  posting.value = true
  try {
    const en = bodyEn.value.trim()
    await api.adminNegotiationDocuments.poserUneNote(props.documentId, {
      page_index: props.page.index,
      passage: passage.value,
      body: en ? { fr: bodyFr.value.trim(), en } : { fr: bodyFr.value.trim() },
    })
    bodyFr.value = ''
    bodyEn.value = ''
    passage.value = null
    posted.value = true
    await refresh()
  } catch (error) {
    postError.value = apiErrorMessage(error, t)
  } finally {
    posting.value = false
  }
}

// --- Retirer ----------------------------------------------------------------

const withdrawing = ref<AdminCorrectionNote | null>(null)
const withdrawOpen = computed({
  get: () => withdrawing.value !== null,
  set: (open: boolean) => {
    if (!open) withdrawing.value = null
  },
})
const withdrawBusy = ref(false)
const withdrawError = ref<string | null>(null)

function askWithdraw(note: AdminCorrectionNote): void {
  withdrawError.value = null
  withdrawing.value = note
}

async function withdraw(): Promise<void> {
  const note = withdrawing.value
  if (!note || withdrawBusy.value) return
  withdrawBusy.value = true
  withdrawError.value = null
  try {
    await api.adminNegotiationDocuments.retirerUneNote(note.id)
    withdrawing.value = null
    await refresh()
  } catch (error) {
    withdrawError.value = apiErrorMessage(error, t)
  } finally {
    withdrawBusy.value = false
  }
}

const body = (note: AdminCorrectionNote, lang: string): string => note.body[lang] ?? ''
const pageLabel = (index: number): string => props.labels.get(index) ?? String(index)
</script>

<template>
  <section class="space-y-4" :aria-labelledby="`notes-${documentId}`">
    <header>
      <h2 :id="`notes-${documentId}`" class="text-sm font-semibold tracking-caps text-text-muted uppercase">
        {{ t('admin.negociations.documents.preview.notes.title') }}
      </h2>
      <p class="mt-1 text-sm text-text-muted">
        {{ t('admin.negociations.documents.preview.notes.subtitle') }}
      </p>
    </header>

    <UiSkeletonLoader v-if="status === 'pending' && !list" variant="text" :lines="4" />

    <UiAlert v-else-if="listError" intent="danger" :title="t('admin.negociations.documents.preview.notes.error')">
      <p>{{ apiErrorMessage(listError, t) }}</p>
      <UiButton class="mt-3" variant="secondary" icon="refresh" @click="refresh()">
        {{ t('common.actions.retry') }}
      </UiButton>
    </UiAlert>

    <template v-else-if="list">
      <form
        v-if="canPost"
        class="space-y-4 rounded-lg border border-border bg-surface-raised p-4"
        novalidate
        @submit.prevent="post"
      >
        <h3 class="font-semibold">
          {{ t('admin.negociations.documents.preview.notes.form.title', { index: page.index, label: page.label }) }}
        </h3>

        <div class="rounded-md border border-dashed border-border-strong bg-surface-sunken p-3 text-sm">
          <p class="font-semibold">{{ t('admin.negociations.documents.preview.notes.form.passage') }}</p>
          <template v-if="passage">
            <blockquote class="mt-1 border-s-2 border-danger ps-3 italic break-words">
              {{ t('admin.negociations.documents.preview.notes.quote', { text: passage }) }}
            </blockquote>
            <UiButton class="mt-2" variant="ghost" size="sm" icon="close" @click="passage = null">
              {{ t('admin.negociations.documents.preview.notes.form.clearPassage') }}
            </UiButton>
          </template>
          <p v-else class="mt-1 text-text-muted">
            {{ t('admin.negociations.documents.preview.notes.form.noPassage') }}
          </p>
        </div>

        <UiFormField :label="t('admin.negociations.documents.preview.notes.form.bodyFr')" required>
          <template #default="{ control }">
            <UiTextarea v-bind="control" v-model="bodyFr" :rows="3" auto-grow />
          </template>
        </UiFormField>
        <UiFormField
          :label="t('admin.negociations.documents.preview.notes.form.bodyEn')"
          :hint="t('admin.negociations.documents.preview.notes.form.bodyEnHint')"
        >
          <template #default="{ control }">
            <UiTextarea v-bind="control" v-model="bodyEn" :rows="2" auto-grow />
          </template>
        </UiFormField>

        <UiAlert v-if="postError" intent="danger" live :message="postError" />
        <UiAlert v-else-if="posted" intent="success" live :message="t('admin.negociations.documents.preview.notes.form.posted')" />

        <div class="flex justify-end">
          <UiButton type="submit" icon="plus" :loading="posting">
            {{ t('admin.negociations.documents.preview.notes.form.submit') }}
          </UiButton>
        </div>
      </form>
      <p v-else class="text-sm text-text-muted">
        {{ t('admin.negociations.documents.preview.notes.readOnly') }}
      </p>

      <div v-for="group in [{ key: 'here', items: here }, { key: 'elsewhere', items: elsewhere }]" :key="group.key">
        <h3 class="mb-2 text-sm font-semibold">
          {{ t(`admin.negociations.documents.preview.notes.${group.key}`, { count: group.items.length }) }}
        </h3>
        <p v-if="!group.items.length" class="text-sm text-text-muted">
          {{ t(`admin.negociations.documents.preview.notes.${group.key}Empty`) }}
        </p>
        <ul v-else class="divide-y divide-border rounded-lg border border-border">
          <li
            v-for="note in group.items"
            :key="note.id"
            class="space-y-2 p-3 text-sm"
            :class="{ 'bg-surface-sunken text-text-muted': note.withdrawn_at }"
          >
            <div class="flex flex-wrap items-center gap-2">
              <UiButton
                v-if="note.page_index !== page.index"
                variant="ghost"
                size="sm"
                icon="arrow-right"
                @click="$emit('go', note.page_index)"
              >
                {{ t('admin.negociations.documents.preview.notes.goTo', { label: pageLabel(note.page_index) }) }}
              </UiButton>
              <UiBadge
                v-if="note.withdrawn_at"
                size="sm"
                :label="t('admin.negociations.documents.preview.notes.withdrawn')"
              />
            </div>
            <blockquote v-if="note.passage" class="border-s-2 border-danger ps-3 italic break-words">
              {{ t('admin.negociations.documents.preview.notes.quote', { text: note.passage }) }}
            </blockquote>
            <p v-else class="text-text-muted">{{ t('admin.negociations.documents.preview.notes.pageTop') }}</p>
            <p class="whitespace-pre-line break-words" lang="fr">{{ body(note, 'fr') }}</p>
            <p v-if="body(note, 'en')" class="whitespace-pre-line break-words text-text-secondary" lang="en">
              <span class="me-1 text-xs font-semibold tracking-caps uppercase">{{ t('admin.negociations.documents.preview.notes.en') }}</span>{{ body(note, 'en') }}
            </p>
            <p class="text-xs text-text-muted">
              {{ t('admin.negociations.documents.preview.notes.postedBy', { name: note.author.name, date: zoned(note.posted_at) }) }}
            </p>
            <p v-if="note.withdrawn_at" class="text-xs text-text-muted">
              {{
                t('admin.negociations.documents.preview.notes.withdrawnBy', {
                  name: note.withdrawn_by?.name ?? '—',
                  date: zoned(note.withdrawn_at),
                })
              }}
            </p>
            <UiButton
              v-else-if="canWithdraw"
              variant="ghost"
              size="sm"
              icon="close"
              @click="askWithdraw(note)"
            >
              {{ t('admin.negociations.documents.preview.notes.withdraw') }}
            </UiButton>
          </li>
        </ul>
      </div>
    </template>

    <UiModal
      v-model:open="withdrawOpen"
      :title="t('admin.negociations.documents.preview.notes.confirm.title')"
      :description="t('admin.negociations.documents.preview.notes.confirm.description')"
    >
      <UiAlert v-if="withdrawError" intent="danger" live :message="withdrawError" />
      <div class="mt-5 flex flex-wrap justify-end gap-3">
        <UiButton variant="ghost" @click="withdrawOpen = false">
          {{ t('admin.negociations.documents.preview.notes.confirm.cancel') }}
        </UiButton>
        <UiButton variant="danger" :loading="withdrawBusy" @click="withdraw">
          {{ t('admin.negociations.documents.preview.notes.confirm.confirm') }}
        </UiButton>
      </div>
    </UiModal>
  </section>
</template>
