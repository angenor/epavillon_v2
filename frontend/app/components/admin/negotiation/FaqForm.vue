<script setup lang="ts">
import type { AdminFaqEntry, AdminFaqInput, AdminKnowledgeSourceInput } from '~/types/admin-negotiation-savoir'
import type { I18nText, Uuid } from '~/types/shared'
import type { SelectOption } from '~/types/ui'
import type { DocumentFormFailure } from '~/components/admin/negotiation/DocumentForm.vue'

const props = withDefaults(
  defineProps<{
    /** Nulle à la création. */
    entry: AdminFaqEntry | null
    readonly?: boolean
    submitting?: boolean
    failure?: DocumentFormFailure | null
    /** Incrémenté par l'écran après un enregistrement : le formulaire se recale sur la réponse. */
    revision?: number
  }>(),
  { readonly: false, submitting: false, failure: null, revision: 0 },
)

const emit = defineEmits<{ submit: [input: AdminFaqInput] }>()

const { t, locale } = useI18n()
const api = useApi()

const { data: vocabulaire, error: vocabulaireEnEchec, refresh: relire } = useAsyncData(
  'admin-negotiation-faq-form-vocabulary',
  async () => {
    const [rubriques, liste] = await Promise.all([
      api.reference.terms('faq_section'),
      api.adminNegotiationSavoir.faq(),
    ])
    return { rubriques, entrees: liste.entries }
  },
  { lazy: true, default: () => null },
)

interface Etat {
  section: string
  questionFr: string
  questionEn: string
  answerFr: string
  answerEn: string
  rank: string
  sources: AdminKnowledgeSourceInput[]
  related: Uuid[]
}

function depuis(e: AdminFaqEntry | null): Etat {
  return {
    section: e?.section_code ?? '',
    questionFr: e?.question.fr ?? '',
    questionEn: e?.question.en ?? '',
    answerFr: e?.answer?.fr ?? '',
    answerEn: e?.answer?.en ?? '',
    rank: e?.editorial_rank?.toString() ?? '',
    sources: (e?.sources ?? []).map(({ document_title: _, ...s }) => s),
    related: (e?.related ?? []).map((r) => r.id),
  }
}

const etat = ref<Etat>(depuis(props.entry))
const erreurLocale = ref<DocumentFormFailure | null>(null)

watch([() => props.entry?.id, () => props.revision], () => {
  etat.value = depuis(props.entry)
  erreurLocale.value = null
})
watch(
  () => props.failure,
  () => (erreurLocale.value = null),
)

const echec = computed(() => erreurLocale.value ?? props.failure)
const erreurDe = (...champs: string[]): string | undefined =>
  echec.value?.field && champs.includes(echec.value.field) ? echec.value.message : undefined

const optionsDeRubrique = computed<SelectOption[]>(() =>
  (vocabulaire.value?.rubriques ?? []).map((r) => ({ value: r.code, label: resolveI18nText(r.label, locale.value) })),
)

const optionsDeLiee = computed<SelectOption[]>(() =>
  (vocabulaire.value?.entrees ?? [])
    .filter((e) => e.id !== props.entry?.id)
    .map((e) => ({
      value: e.id,
      label: e.question,
      description: t(`admin.negociations.faq.state.${e.status}`),
    })),
)

const titresConnus = computed(() =>
  Object.fromEntries(
    (props.entry?.sources ?? []).flatMap((s) => (s.document_id && s.document_title ? [[s.document_id, s.document_title]] : [])),
  ),
)

function texte(fr: string, en: string): I18nText {
  const valeur: I18nText = { fr: fr.trim() }
  if (en.trim()) valeur.en = en.trim()
  return valeur
}

function soumettre(): void {
  if (props.readonly || props.submitting) return
  const e = etat.value
  erreurLocale.value = null
  const rang = e.rank.trim() ? Number.parseInt(e.rank, 10) : null
  const refus = (field: string, cle: string) => (erreurLocale.value = { field, message: t(cle) })
  if (!e.section) return void refus('section_code', 'admin.negociations.faq.form.error.section')
  if (!e.questionFr.trim()) return void refus('question', 'admin.negociations.faq.form.error.question')
  if (!e.answerFr.trim() && e.answerEn.trim()) return void refus('answer', 'admin.negociations.faq.form.error.answer')
  if (rang !== null && (!Number.isInteger(rang) || rang < 1)) {
    return void refus('editorial_rank', 'admin.negociations.faq.form.error.rank')
  }
  if (e.sources.some((s) => !s.document_id && !s.external_title?.trim())) {
    return void refus('sources', 'admin.negociations.faq.form.error.source')
  }
  emit('submit', {
    section_code: e.section,
    question: texte(e.questionFr, e.questionEn),
    answer: e.answerFr.trim() ? texte(e.answerFr, e.answerEn) : null,
    editorial_rank: rang,
    sources: e.sources.map((s) => ({ ...s, external_title: s.external_title?.trim() ?? null })),
    related_ids: [...e.related],
  })
}
</script>

<template>
  <form class="space-y-8" novalidate @submit.prevent="soumettre">
    <UiAlert v-if="props.readonly" intent="info" compact :message="t('admin.negociations.faq.form.readonly')" />

    <UiAlert v-if="vocabulaireEnEchec" intent="warning" :message="t('admin.negociations.faq.form.vocabularyError')">
      <template #actions>
        <UiButton variant="secondary" size="sm" icon="refresh" @click="relire()">
          {{ t('common.actions.retry') }}
        </UiButton>
      </template>
    </UiAlert>

    <section class="space-y-5">
      <h2 class="text-xl font-semibold">{{ t('admin.negociations.faq.form.sections.question') }}</h2>

      <UiSelect
        v-model="etat.section"
        class="max-w-md"
        :label="t('admin.negociations.faq.form.section.label')"
        :placeholder="t('admin.negociations.faq.form.section.placeholder')"
        :options="optionsDeRubrique"
        :error="erreurDe('section_code')"
        :readonly="props.readonly"
        required
      />

      <div class="grid gap-5 sm:grid-cols-2">
        <UiInput
          v-model="etat.questionFr"
          :label="t('admin.negociations.faq.form.question.fr')"
          :error="erreurDe('question')"
          :readonly="props.readonly"
          :maxlength="300"
          required
        />
        <UiInput
          v-model="etat.questionEn"
          :label="t('admin.negociations.faq.form.question.en')"
          :hint="t('admin.negociations.faq.form.enHint')"
          :readonly="props.readonly"
          :maxlength="300"
        />
        <UiTextarea
          v-model="etat.answerFr"
          :label="t('admin.negociations.faq.form.answer.fr')"
          :hint="t('admin.negociations.faq.form.answer.hint')"
          :error="erreurDe('answer')"
          :readonly="props.readonly"
          :rows="8"
          auto-grow
        />
        <UiTextarea
          v-model="etat.answerEn"
          :label="t('admin.negociations.faq.form.answer.en')"
          :readonly="props.readonly"
          :rows="8"
          auto-grow
        />
      </div>

      <UiInput
        v-model="etat.rank"
        class="max-w-xs"
        type="number"
        inputmode="numeric"
        :min="1"
        :label="t('admin.negociations.faq.form.rank.label')"
        :hint="t('admin.negociations.faq.form.rank.hint')"
        :error="erreurDe('editorial_rank')"
        :readonly="props.readonly"
      />
    </section>

    <section class="space-y-5 border-t border-border pt-8">
      <AdminNegotiationSourcesField
        v-model="etat.sources"
        :known-titles="titresConnus"
        :readonly="props.readonly"
        :error="erreurDe('sources')"
      />
    </section>

    <section class="space-y-5 border-t border-border pt-8">
      <AdminNegotiationRelatedField
        v-model="etat.related"
        :options="optionsDeLiee"
        :known="props.entry?.related ?? []"
        :label="t('admin.negociations.faq.form.related.label')"
        :hint="t('admin.negociations.faq.form.related.hint')"
        :placeholder="t('admin.negociations.faq.form.related.placeholder')"
        :readonly="props.readonly"
        :error="erreurDe('related_ids')"
      />
    </section>

    <template v-if="!props.readonly">
      <UiAlert v-if="echec" intent="danger" live :message="echec.message" />
      <div class="flex flex-wrap gap-3">
        <UiButton type="submit" :loading="props.submitting">
          {{ props.entry ? t('admin.negociations.faq.form.save') : t('admin.negociations.faq.form.create') }}
        </UiButton>
        <slot name="actions" />
      </div>
    </template>
  </form>
</template>
