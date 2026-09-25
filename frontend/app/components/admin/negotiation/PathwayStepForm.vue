<script setup lang="ts">
import type {
  AdminPathwayLinkKind,
  AdminPathwayStep,
  AdminPathwayStepInput,
} from '~/types/admin-negotiation-savoir'
import type { I18nText, Uuid } from '~/types/shared'
import type { SelectOption } from '~/types/ui'
import type { DocumentFormFailure } from '~/components/admin/negotiation/DocumentForm.vue'

const props = withDefaults(
  defineProps<{
    /** Nulle à la création. */
    step: AdminPathwayStep | null
    groupId: Uuid
    submitting?: boolean
    failure?: DocumentFormFailure | null
  }>(),
  { submitting: false, failure: null },
)

const emit = defineEmits<{ submit: [input: AdminPathwayStepInput]; cancel: [] }>()

const { t } = useI18n()
const api = useApi()

const { data: cibles, error: ciblesEnEchec, refresh } = useAsyncData(
  'admin-negotiation-pathway-targets',
  async () => {
    const [documents, faq, lexique] = await Promise.all([
      api.adminNegotiationDocuments.documents(),
      api.adminNegotiationSavoir.faq(),
      api.adminNegotiationSavoir.lexique(),
    ])
    return {
      document: documents.documents.map((d) => ({ value: d.id, label: d.title })),
      faq: faq.entries.map((e) => ({ value: e.id, label: e.question })),
      glossary: lexique.entries.map((e) => ({ value: e.id, label: e.term, description: e.translation })),
    } satisfies Record<AdminPathwayLinkKind, SelectOption[]>
  },
  { lazy: true, default: () => null },
)

const s = props.step
const lien = s?.link ?? null
const etat = ref({
  labelFr: s?.label.fr ?? '',
  labelEn: s?.label.en ?? '',
  detailFr: s?.detail?.fr ?? '',
  detailEn: s?.detail?.en ?? '',
  originFr: s?.origin_label?.fr ?? '',
  originEn: s?.origin_label?.en ?? '',
  kind: (lien?.kind ?? '') as AdminPathwayLinkKind | '',
  target: lien?.target_id ?? '',
  page: lien?.page?.toString() ?? '',
  section: lien?.section ?? '',
  linkLabelFr: lien?.label?.fr ?? '',
  linkLabelEn: lien?.label?.en ?? '',
  published: false,
})
const erreurLocale = ref<DocumentFormFailure | null>(null)
watch(
  () => props.failure,
  () => (erreurLocale.value = null),
)
const echec = computed(() => erreurLocale.value ?? props.failure)
const erreurDe = (champ: string): string | undefined =>
  echec.value?.field === champ ? echec.value.message : undefined

const optionsDeNature = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.negociations.parcours.step.link.none') },
  { value: 'document', label: t('admin.negociations.parcours.step.link.kind.document') },
  { value: 'faq', label: t('admin.negociations.parcours.step.link.kind.faq') },
  { value: 'glossary', label: t('admin.negociations.parcours.step.link.kind.glossary') },
])

const optionsDeCible = computed<SelectOption[]>(() => {
  const kind = etat.value.kind
  if (!kind) return []
  const liste = cibles.value?.[kind] ?? []
  const actuel = lien && lien.kind === kind && !liste.some((o) => o.value === lien.target_id)
  return actuel ? [{ value: lien.target_id, label: lien.target_label ?? lien.target_id }, ...liste] : liste
})

watch(
  () => etat.value.kind,
  () => (etat.value.target = ''),
)

function texte(fr: string, en: string): I18nText | null {
  if (!fr.trim()) return null
  const valeur: I18nText = { fr: fr.trim() }
  if (en.trim()) valeur.en = en.trim()
  return valeur
}

function soumettre(): void {
  if (props.submitting) return
  const e = etat.value
  erreurLocale.value = null
  const label = texte(e.labelFr, e.labelEn)
  if (!label) {
    erreurLocale.value = { field: 'label', message: t('admin.negociations.parcours.step.error.label') }
    return
  }
  if (e.kind && !e.target) {
    erreurLocale.value = { field: 'link', message: t('admin.negociations.parcours.step.error.target') }
    return
  }
  const page = Number.parseInt(e.page, 10)
  const entree: AdminPathwayStepInput = {
    label,
    detail: texte(e.detailFr, e.detailEn),
    origin_label: texte(e.originFr, e.originEn),
    link: e.kind
      ? {
          kind: e.kind,
          target_id: e.target,
          page: e.kind === 'document' && page > 0 ? page : null,
          section: e.kind === 'document' ? e.section.trim() || null : null,
          label: texte(e.linkLabelFr, e.linkLabelEn),
        }
      : null,
  }
  if (!props.step) {
    entree.group_id = props.groupId
    entree.is_published = e.published
  }
  emit('submit', entree)
}
</script>

<template>
  <form class="space-y-5" novalidate @submit.prevent="soumettre">
    <UiAlert v-if="ciblesEnEchec" intent="warning" compact :message="t('admin.negociations.parcours.step.targetsError')">
      <template #actions>
        <UiButton variant="secondary" size="sm" icon="refresh" @click="refresh()">
          {{ t('common.actions.retry') }}
        </UiButton>
      </template>
    </UiAlert>

    <div class="grid gap-4 sm:grid-cols-2">
      <UiInput
        v-model="etat.labelFr"
        :label="t('admin.negociations.parcours.step.label.fr')"
        :error="erreurDe('label')"
        :maxlength="300"
        required
      />
      <UiInput v-model="etat.labelEn" :label="t('admin.negociations.parcours.step.label.en')" :maxlength="300" />
      <UiTextarea
        v-model="etat.detailFr"
        :label="t('admin.negociations.parcours.step.detail.fr')"
        :hint="t('admin.negociations.parcours.step.detail.hint')"
        :error="erreurDe('detail')"
        :rows="2"
        auto-grow
      />
      <UiTextarea v-model="etat.detailEn" :label="t('admin.negociations.parcours.step.detail.en')" :rows="2" auto-grow />
      <UiInput
        v-model="etat.originFr"
        :label="t('admin.negociations.parcours.step.origin.fr')"
        :hint="t('admin.negociations.parcours.step.origin.hint')"
        :error="erreurDe('origin_label')"
        :maxlength="200"
      />
      <UiInput v-model="etat.originEn" :label="t('admin.negociations.parcours.step.origin.en')" :maxlength="200" />
    </div>

    <fieldset class="space-y-4 border-t border-border pt-5">
      <legend class="sr-only">{{ t('admin.negociations.parcours.step.link.legend') }}</legend>
      <div class="grid gap-4 sm:grid-cols-2">
        <UiSelect
          v-model="etat.kind"
          :label="t('admin.negociations.parcours.step.link.label')"
          :options="optionsDeNature"
          hide-optional
        />
        <UiCombobox
          v-if="etat.kind"
          v-model="etat.target"
          :label="t('admin.negociations.parcours.step.link.target')"
          :placeholder="t('admin.negociations.parcours.step.link.targetPlaceholder')"
          :options="optionsDeCible"
          :error="erreurDe('link')"
          required
        />
        <template v-if="etat.kind === 'document'">
          <UiInput
            v-model="etat.page"
            type="number"
            inputmode="numeric"
            :min="1"
            :label="t('admin.negociations.parcours.step.link.page')"
          />
          <UiInput
            v-model="etat.section"
            :label="t('admin.negociations.parcours.step.link.section')"
            :placeholder="t('admin.negociations.parcours.step.link.sectionPlaceholder')"
            :maxlength="200"
          />
        </template>
        <template v-if="etat.kind">
          <UiInput
            v-model="etat.linkLabelFr"
            :label="t('admin.negociations.parcours.step.link.textFr')"
            :hint="t('admin.negociations.parcours.step.link.textHint')"
            :maxlength="200"
          />
          <UiInput v-model="etat.linkLabelEn" :label="t('admin.negociations.parcours.step.link.textEn')" :maxlength="200" />
        </template>
      </div>
    </fieldset>

    <UiCheckbox
      v-if="!props.step"
      v-model="etat.published"
      :label="t('admin.negociations.parcours.step.published')"
      :hint="t('admin.negociations.parcours.step.publishedHint')"
    />

    <UiAlert v-if="echec" intent="danger" live :message="echec.message" />

    <div class="flex flex-wrap justify-end gap-3">
      <UiButton variant="ghost" @click="emit('cancel')">{{ t('admin.negociations.parcours.cancel') }}</UiButton>
      <UiButton type="submit" :loading="props.submitting">
        {{ props.step ? t('admin.negociations.parcours.step.save') : t('admin.negociations.parcours.step.create') }}
      </UiButton>
    </div>
  </form>
</template>
