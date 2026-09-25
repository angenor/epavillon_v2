<script setup lang="ts">
import type {
  AdminGlossaryEntry,
  AdminGlossaryInput,
  AdminKnowledgeSourceInput,
} from '~/types/admin-negotiation-savoir'
import type { I18nText, Uuid } from '~/types/shared'
import type { SelectOption } from '~/types/ui'
import type { DocumentFormFailure } from '~/components/admin/negotiation/DocumentForm.vue'

const props = withDefaults(
  defineProps<{
    /** Nul à la création. */
    entry: AdminGlossaryEntry | null
    readonly?: boolean
    submitting?: boolean
    failure?: DocumentFormFailure | null
    revision?: number
  }>(),
  { readonly: false, submitting: false, failure: null, revision: 0 },
)

const emit = defineEmits<{ submit: [input: AdminGlossaryInput] }>()

const { t, locale } = useI18n()
const api = useApi()

const { data: vocabulaire, error: vocabulaireEnEchec, refresh: relire } = useAsyncData(
  'admin-negotiation-glossary-form-vocabulary',
  async () => {
    const [familles, liste] = await Promise.all([
      api.reference.terms('glossary_family'),
      api.adminNegotiationSavoir.lexique(),
    ])
    return { familles, entrees: liste.entries }
  },
  { lazy: true, default: () => null },
)

interface Etat {
  family: string
  term: string
  acronym: string
  variants: string
  translationFr: string
  translationEn: string
  definitionFr: string
  definitionEn: string
  heardInRoom: string
  sources: AdminKnowledgeSourceInput[]
  related: Uuid[]
}

function depuis(e: AdminGlossaryEntry | null): Etat {
  return {
    family: e?.family_code ?? '',
    term: e?.term ?? '',
    acronym: e?.acronym ?? '',
    variants: (e?.variants ?? []).join('\n'),
    translationFr: e?.translation.fr ?? '',
    translationEn: e?.translation.en ?? '',
    definitionFr: e?.definition.fr ?? '',
    definitionEn: e?.definition.en ?? '',
    heardInRoom: e?.heard_in_room ?? '',
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
const erreurDe = (champ: string): string | undefined =>
  echec.value?.field === champ ? echec.value.message : undefined

const optionsDeFamille = computed<SelectOption[]>(() =>
  (vocabulaire.value?.familles ?? []).map((f) => ({ value: f.code, label: resolveI18nText(f.label, locale.value) })),
)

const optionsDeLie = computed<SelectOption[]>(() =>
  (vocabulaire.value?.entrees ?? [])
    .filter((e) => e.id !== props.entry?.id)
    .map((e) => ({
      value: e.id,
      label: e.acronym ? `${e.term} (${e.acronym})` : e.term,
      description: e.translation,
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
  const refus = (field: string, cle: string) => (erreurLocale.value = { field, message: t(cle) })
  if (!e.family) return void refus('family_code', 'admin.negociations.lexique.form.error.family')
  if (!e.term.trim()) return void refus('term', 'admin.negociations.lexique.form.error.term')
  if (!e.translationFr.trim()) return void refus('translation', 'admin.negociations.lexique.form.error.translation')
  if (!e.definitionFr.trim()) return void refus('definition', 'admin.negociations.lexique.form.error.definition')
  if (e.sources.some((s) => !s.document_id && !s.external_title?.trim())) {
    return void refus('sources', 'admin.negociations.lexique.form.error.source')
  }
  emit('submit', {
    family_code: e.family,
    term: e.term.trim(),
    acronym: e.acronym.trim() || null,
    variants: [...new Set(e.variants.split('\n').map((v) => v.trim()).filter(Boolean))],
    translation: texte(e.translationFr, e.translationEn),
    definition: texte(e.definitionFr, e.definitionEn),
    heard_in_room: e.heardInRoom.trim() || null,
    sources: e.sources.map((s) => ({ ...s, external_title: s.external_title?.trim() ?? null })),
    related_ids: [...e.related],
  })
}
</script>

<template>
  <form class="space-y-8" novalidate @submit.prevent="soumettre">
    <UiAlert v-if="props.readonly" intent="info" compact :message="t('admin.negociations.lexique.form.readonly')" />

    <UiAlert
      v-if="vocabulaireEnEchec"
      intent="warning"
      :message="t('admin.negociations.lexique.form.vocabularyError')"
    >
      <template #actions>
        <UiButton variant="secondary" size="sm" icon="refresh" @click="relire()">
          {{ t('common.actions.retry') }}
        </UiButton>
      </template>
    </UiAlert>

    <section class="space-y-5">
      <h2 class="text-xl font-semibold">{{ t('admin.negociations.lexique.form.sections.term') }}</h2>

      <div class="grid gap-5 sm:grid-cols-2">
        <UiSelect
          v-model="etat.family"
          :label="t('admin.negociations.lexique.form.family.label')"
          :placeholder="t('admin.negociations.lexique.form.family.placeholder')"
          :options="optionsDeFamille"
          :error="erreurDe('family_code')"
          :readonly="props.readonly"
          required
        />
        <div class="hidden sm:block" aria-hidden="true" />
        <UiInput
          v-model="etat.term"
          :label="t('admin.negociations.lexique.form.term.label')"
          :hint="t('admin.negociations.lexique.form.term.hint')"
          :error="erreurDe('term')"
          :readonly="props.readonly"
          :maxlength="200"
          required
        />
        <UiInput
          v-model="etat.acronym"
          :label="t('admin.negociations.lexique.form.acronym.label')"
          :placeholder="t('admin.negociations.lexique.form.acronym.placeholder')"
          :error="erreurDe('acronym')"
          :readonly="props.readonly"
          :maxlength="40"
        />
      </div>

      <UiTextarea
        v-model="etat.variants"
        :label="t('admin.negociations.lexique.form.variants.label')"
        :hint="t('admin.negociations.lexique.form.variants.hint')"
        :error="erreurDe('variants')"
        :readonly="props.readonly"
        :rows="3"
        auto-grow
      />

      <div class="grid gap-5 sm:grid-cols-2">
        <UiInput
          v-model="etat.translationFr"
          :label="t('admin.negociations.lexique.form.translation.fr')"
          :error="erreurDe('translation')"
          :readonly="props.readonly"
          :maxlength="300"
          required
        />
        <UiInput
          v-model="etat.translationEn"
          :label="t('admin.negociations.lexique.form.translation.en')"
          :hint="t('admin.negociations.lexique.form.enHint')"
          :readonly="props.readonly"
          :maxlength="300"
        />
        <UiTextarea
          v-model="etat.definitionFr"
          :label="t('admin.negociations.lexique.form.definition.fr')"
          :error="erreurDe('definition')"
          :readonly="props.readonly"
          :rows="5"
          auto-grow
          required
        />
        <UiTextarea
          v-model="etat.definitionEn"
          :label="t('admin.negociations.lexique.form.definition.en')"
          :readonly="props.readonly"
          :rows="5"
          auto-grow
        />
      </div>

      <UiInput
        v-model="etat.heardInRoom"
        :label="t('admin.negociations.lexique.form.heardInRoom.label')"
        :hint="t('admin.negociations.lexique.form.heardInRoom.hint')"
        :placeholder="t('admin.negociations.lexique.form.heardInRoom.placeholder')"
        :error="erreurDe('heard_in_room')"
        :readonly="props.readonly"
        :maxlength="300"
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
        :options="optionsDeLie"
        :known="props.entry?.related ?? []"
        :label="t('admin.negociations.lexique.form.related.label')"
        :hint="t('admin.negociations.lexique.form.related.hint')"
        :placeholder="t('admin.negociations.lexique.form.related.placeholder')"
        :readonly="props.readonly"
        :error="erreurDe('related_ids')"
      />
    </section>

    <template v-if="!props.readonly">
      <UiAlert v-if="echec" intent="danger" live :message="echec.message" />
      <div class="flex flex-wrap gap-3">
        <UiButton type="submit" :loading="props.submitting">
          {{ props.entry ? t('admin.negociations.lexique.form.save') : t('admin.negociations.lexique.form.create') }}
        </UiButton>
        <slot name="actions" />
      </div>
    </template>
  </form>
</template>
