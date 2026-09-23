<script setup lang="ts">
import type { AdminDocument, AdminDocumentInput } from '~/types/admin-negotiation-documents'
import type { I18nText, Uuid } from '~/types/shared'
import type { SelectOption } from '~/types/ui'
import type { PickableTheme } from '~/components/admin/ThemePicker.vue'

/**
 * La fiche d'un document de Guide Négo, à la création comme à la modification.
 *
 * Le vocabulaire (types, thématiques, langues) et les COP viennent de l'API ; le
 * formulaire envoie des codes, jamais des libellés. Le PDF se dépose avec le
 * brouillon pour propriétaire : à la création, il n'a pas encore d'existence.
 */

export interface DocumentFormFailure {
  message: string
  /** Le champ que l'API nomme, s'il en nomme un. */
  field: string | null
}

const props = withDefaults(
  defineProps<{
    /** Nul à la création. */
    document: AdminDocument | null
    readonly?: boolean
    submitting?: boolean
    attaching?: boolean
    failure?: DocumentFormFailure | null
    /** Incrémenté par l'écran après un enregistrement : le formulaire se recale sur la réponse. */
    revision?: number
  }>(),
  { readonly: false, submitting: false, attaching: false, failure: null, revision: 0 },
)

const emit = defineEmits<{
  submit: [input: AdminDocumentInput]
  attach: [assetId: Uuid]
}>()

const { t, locale } = useI18n()
const api = useApi()

const { data: vocabulaire, error: vocabulaireEnEchec, refresh: relireLeVocabulaire } = useAsyncData(
  'admin-negotiation-document-form-vocabulary',
  async () => {
    const [types, themes, langues, editions, documents] = await Promise.all([
      api.reference.terms('document_type'),
      api.reference.terms('negotiation_theme'),
      api.reference.locales(),
      api.events.publicList(),
      api.adminNegotiationDocuments.documents(),
    ])
    return { types, themes, langues, editions, documents: documents.documents }
  },
  { lazy: true, default: () => null },
)

interface Etat {
  titleFr: string
  titleEn: string
  summaryFr: string
  summaryEn: string
  type: string
  themes: string[]
  cop: string
  version: string
  issuedOn: string
  publisher: string
  locale: string
  externalUrl: string
  supersedesId: string
  restricted: boolean
  ragEligible: boolean
}

function depuis(d: AdminDocument | null): Etat {
  return {
    titleFr: d?.title.fr ?? '',
    titleEn: d?.title.en ?? '',
    summaryFr: d?.summary?.fr ?? '',
    summaryEn: d?.summary?.en ?? '',
    type: d?.type ?? '',
    themes: [...(d?.themes ?? [])],
    cop: d?.cop ?? '',
    version: d?.version ?? '',
    issuedOn: d?.issued_on ?? '',
    publisher: d?.publisher ?? '',
    locale: d?.locale ?? 'fr',
    externalUrl: d?.external_url ?? '',
    supersedesId: d?.supersedes?.id ?? '',
    // Le défaut de la base : un document naît réservé.
    restricted: d?.restricted ?? true,
    ragEligible: d?.rag_eligible ?? false,
  }
}

const etat = ref<Etat>(depuis(props.document))
const erreurLocale = ref<DocumentFormFailure | null>(null)

// Une relecture du même document (l'extraction qui avance) n'efface pas la saisie en cours.
watch([() => props.document?.id, () => props.revision], () => {
  etat.value = depuis(props.document)
  erreurLocale.value = null
})

watch(
  () => props.failure,
  () => {
    erreurLocale.value = null
  },
)

const VALEUR_DU_CHAMP: Record<string, (e: Etat) => string> = {
  title: (e) => e.titleFr,
  type: (e) => e.type,
  version: (e) => e.version,
}

// L'erreur locale tombe dès que le champ qu'elle nomme change.
watch(
  () => {
    const champ = erreurLocale.value?.field ?? null
    return [champ, champ ? VALEUR_DU_CHAMP[champ]?.(etat.value) : undefined] as const
  },
  ([champ, valeur], [champAvant, valeurAvant]) => {
    if (champ && champ === champAvant && valeur !== valeurAvant) erreurLocale.value = null
  },
)

const echec = computed(() => erreurLocale.value ?? props.failure)
const erreurDe = (champ: string): string | undefined =>
  echec.value?.field === champ ? echec.value.message : undefined

const libelle = (texte: I18nText | null | undefined): string => resolveI18nText(texte, locale.value)

const optionsDeType = computed<SelectOption[]>(() =>
  (vocabulaire.value?.types ?? []).map((terme) => ({ value: terme.code, label: libelle(terme.label) })),
)

const themesChoisissables = computed<PickableTheme[]>(() =>
  (vocabulaire.value?.themes ?? []).map((terme) => ({
    code: terme.code,
    label: terme.label,
    color: terme.color_hex,
  })),
)

const optionsDeLangue = computed<SelectOption[]>(() =>
  (vocabulaire.value?.langues ?? [])
    .filter((langue) => langue.is_active)
    .sort((a, b) => a.sort_order - b.sort_order)
    .map((langue) => ({ value: langue.code, label: langue.native_label })),
)

const optionsDeCop = computed<SelectOption[]>(() => {
  const editions = (vocabulaire.value?.editions ?? [])
    .filter((edition) => edition.series_kind?.startsWith('cop_'))
    .map((edition) => ({
    value: edition.id,
    label: edition.edition_label ?? libelle(edition.title),
  }))
  const cop = etat.value.cop
  const absente = cop && vocabulaire.value && !editions.some((option) => option.value === cop)
  return [
    { value: '', label: t('admin.negociations.documents.form.cop.none') },
    ...editions,
    ...(absente ? [{ value: cop, label: t('admin.negociations.documents.form.cop.unknown') }] : []),
  ]
})

const optionsDeRemplacement = computed<SelectOption[]>(() => {
  const moi = props.document?.id
  const publies = (vocabulaire.value?.documents ?? [])
    .filter((d) => d.state === 'published' && d.id !== moi && (!d.superseded_by || d.superseded_by.id === moi))
    .map((d) => ({
      value: d.id,
      label: d.title,
      description: t('admin.negociations.documents.form.supersedes.version', { version: d.version }),
    }))
  const actuel = props.document?.supersedes
  const manquant = actuel && !publies.some((option) => option.value === actuel.id)
  return [
    { value: '', label: t('admin.negociations.documents.form.supersedes.none') },
    ...(manquant
      ? [
          {
            value: actuel.id,
            label: actuel.title,
            description: t('admin.negociations.documents.form.supersedes.version', { version: actuel.version }),
          },
        ]
      : []),
    ...publies,
  ]
})

const optionsDAcces = computed<SelectOption[]>(() => [
  {
    value: 'restricted',
    label: t('admin.negociations.documents.form.access.restricted'),
    description: t('admin.negociations.documents.form.access.restrictedHint'),
  },
  {
    value: 'public',
    label: t('admin.negociations.documents.form.access.public'),
    description: t('admin.negociations.documents.form.access.publicHint'),
  },
])

const fichierFige = computed(() => props.document?.file_locked ?? false)
const fichierDepose = computed(() => Boolean(props.document?.asset_id))
const lienRempli = computed(() => etat.value.externalUrl.trim().length > 0)

/** Pourquoi le dépôt est fermé, s'il l'est — la phrase que lit l'utilisateur. */
const fichierFerme = computed<string | null>(() => {
  if (!props.document) return t('admin.negociations.documents.form.file.afterCreate')
  if (fichierFige.value) return t('admin.negociations.documents.form.file.locked')
  if (props.document.external_url) return t('admin.negociations.documents.form.file.linkSaved')
  if (lienRempli.value && !fichierDepose.value) return t('admin.negociations.documents.form.file.linkFilled')
  return null
})

const lienFerme = computed<string | null>(() => {
  if (fichierFige.value) return t('admin.negociations.documents.form.link.locked')
  if (fichierDepose.value) return t('admin.negociations.documents.form.link.fileAttached')
  return null
})

const fichierActuel = computed(() => {
  const fichier = props.document?.file
  if (!fichier) return null
  return {
    filename: fichier.filename ?? t('admin.negociations.documents.form.file.unnamed'),
    byteSize: fichier.byte_size,
  }
})

const proprietaire = computed(() =>
  props.document ? { schema: 'negotiation', table: 'documents', id: props.document.id } : null,
)

function surDepot(assetId: Uuid | null): void {
  // « Retirer » n'a pas de route : un fichier attaché se remplace, il ne se détache pas.
  if (assetId) emit('attach', assetId)
}

function texte(fr: string, en: string): I18nText {
  const valeur: I18nText = { fr: fr.trim() }
  if (en.trim()) valeur.en = en.trim()
  return valeur
}

function versLEntree(): AdminDocumentInput {
  const e = etat.value
  const entree: AdminDocumentInput = {
    title: texte(e.titleFr, e.titleEn),
    summary: e.summaryFr.trim() || e.summaryEn.trim() ? texte(e.summaryFr, e.summaryEn) : null,
    type: e.type,
    themes: [...e.themes],
    cop: e.cop || null,
    issued_on: e.issuedOn || null,
    publisher: e.publisher.trim() || null,
    locale: e.locale,
    supersedes_id: e.supersedesId || null,
    restricted: e.restricted,
    rag_eligible: e.ragEligible,
  }
  if (e.version.trim()) entree.version = e.version.trim()
  // L'API refuse le champ du lien, même inchangé, sur un document déjà publié.
  if (!fichierFige.value) entree.external_url = e.externalUrl.trim() || null
  return entree
}

function soumettre(): void {
  if (props.readonly || props.submitting) return
  erreurLocale.value = null
  if (!etat.value.titleFr.trim()) {
    erreurLocale.value = { message: t('admin.negociations.documents.form.error.title'), field: 'title' }
    return
  }
  if (!etat.value.type) {
    erreurLocale.value = { message: t('admin.negociations.documents.form.error.type'), field: 'type' }
    return
  }
  // L'API ignore une version vide : sans ce refus, l'effacer ne changerait rien, en silence.
  if (props.document && !etat.value.version.trim()) {
    erreurLocale.value = { message: t('admin.negociations.documents.form.error.version'), field: 'version' }
    return
  }
  emit('submit', versLEntree())
}
</script>

<template>
  <form class="space-y-8" novalidate @submit.prevent="soumettre">
    <UiAlert
      v-if="props.readonly"
      intent="info"
      compact
      :message="t('admin.negociations.documents.form.readonly')"
    />

    <UiAlert
      v-if="vocabulaireEnEchec"
      intent="warning"
      :message="t('admin.negociations.documents.form.vocabularyError')"
    >
      <template #actions>
        <UiButton variant="secondary" size="sm" icon="refresh" @click="relireLeVocabulaire()">
          {{ t('common.actions.retry') }}
        </UiButton>
      </template>
    </UiAlert>

    <section class="space-y-5">
      <h2 class="text-xl font-semibold">{{ t('admin.negociations.documents.form.sections.description') }}</h2>

      <div class="grid gap-5 sm:grid-cols-2">
        <UiInput
          v-model="etat.titleFr"
          :label="t('admin.negociations.documents.form.title.fr')"
          :error="erreurDe('title')"
          :readonly="props.readonly"
          :maxlength="300"
          required
        />
        <UiInput
          v-model="etat.titleEn"
          :label="t('admin.negociations.documents.form.title.en')"
          :hint="t('admin.negociations.documents.form.title.enHint')"
          :readonly="props.readonly"
          :maxlength="300"
        />
        <UiTextarea
          v-model="etat.summaryFr"
          :label="t('admin.negociations.documents.form.summary.fr')"
          :error="erreurDe('summary')"
          :readonly="props.readonly"
          :rows="4"
          auto-grow
        />
        <UiTextarea
          v-model="etat.summaryEn"
          :label="t('admin.negociations.documents.form.summary.en')"
          :readonly="props.readonly"
          :rows="4"
          auto-grow
        />
      </div>
    </section>

    <section class="space-y-5 border-t border-border pt-8">
      <h2 class="text-xl font-semibold">{{ t('admin.negociations.documents.form.sections.classification') }}</h2>

      <div class="grid gap-5 sm:grid-cols-2">
        <UiSelect
          v-model="etat.type"
          :label="t('admin.negociations.documents.form.type.label')"
          :placeholder="t('admin.negociations.documents.form.type.placeholder')"
          :options="optionsDeType"
          :error="erreurDe('type')"
          :readonly="props.readonly"
          required
        />
        <UiSelect
          v-model="etat.cop"
          :label="t('admin.negociations.documents.form.cop.label')"
          :hint="t('admin.negociations.documents.form.cop.hint')"
          :options="optionsDeCop"
          :error="erreurDe('cop')"
          :readonly="props.readonly"
        />
      </div>

      <AdminThemePicker
        v-model="etat.themes"
        :themes="themesChoisissables"
        :label="t('admin.negociations.documents.form.themes.label')"
        :hint="t('admin.negociations.documents.form.themes.hint')"
        :error="erreurDe('themes')"
        :readonly="props.readonly"
      />

      <div class="grid gap-5 sm:grid-cols-2">
        <UiInput
          v-model="etat.version"
          :label="t('admin.negociations.documents.form.version.label')"
          :hint="t('admin.negociations.documents.form.version.hint')"
          :error="erreurDe('version')"
          :readonly="props.readonly"
          :maxlength="60"
          :required="props.document !== null"
        />
        <UiDatePicker
          v-model="etat.issuedOn"
          :label="t('admin.negociations.documents.form.issuedOn.label')"
          :hint="t('admin.negociations.documents.form.issuedOn.hint')"
          :error="erreurDe('issued_on')"
          :readonly="props.readonly"
        />
        <UiInput
          v-model="etat.publisher"
          :label="t('admin.negociations.documents.form.publisher.label')"
          :placeholder="t('admin.negociations.documents.form.publisher.placeholder')"
          :error="erreurDe('publisher')"
          :readonly="props.readonly"
          :maxlength="200"
        />
        <UiSelect
          v-model="etat.locale"
          :label="t('admin.negociations.documents.form.locale.label')"
          :hint="t('admin.negociations.documents.form.locale.hint')"
          :options="optionsDeLangue"
          :error="erreurDe('locale')"
          :readonly="props.readonly"
          required
        />
      </div>
    </section>

    <section class="space-y-5 border-t border-border pt-8">
      <header>
        <h2 class="text-xl font-semibold">{{ t('admin.negociations.documents.form.sections.source') }}</h2>
        <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
          {{ t('admin.negociations.documents.form.sections.sourceHint') }}
        </p>
      </header>

      <div class="grid gap-5 lg:grid-cols-2">
        <div class="space-y-2">
          <MediaFileField
            :label="t('admin.negociations.documents.form.file.label')"
            :types="['application/pdf']"
            :asset-id="props.document?.asset_id ?? null"
            :current="fichierActuel"
            :owner="proprietaire"
            :removable="false"
            :disabled="props.readonly || fichierFerme !== null || props.attaching"
            @update:asset-id="surDepot"
          />
          <p v-if="props.attaching" class="flex items-center gap-2 text-sm text-text-muted" aria-live="polite">
            <UiSpinner />
            {{ t('admin.negociations.documents.form.file.attaching') }}
          </p>
          <p v-else-if="fichierFerme && !props.readonly" class="text-sm text-text-muted">{{ fichierFerme }}</p>
          <p v-if="erreurDe('asset_id')" role="alert" class="text-sm font-bold text-danger">
            {{ erreurDe('asset_id') }}
          </p>
        </div>

        <UiInput
          v-model="etat.externalUrl"
          type="url"
          inputmode="url"
          :label="t('admin.negociations.documents.form.link.label')"
          :placeholder="t('admin.negociations.documents.form.link.placeholder')"
          :hint="props.readonly ? undefined : (lienFerme ?? t('admin.negociations.documents.form.link.hint'))"
          :error="erreurDe('external_url')"
          :readonly="props.readonly"
          :disabled="!props.readonly && lienFerme !== null"
        />
      </div>
    </section>

    <section class="space-y-5 border-t border-border pt-8">
      <h2 class="text-xl font-semibold">{{ t('admin.negociations.documents.form.sections.publication') }}</h2>

      <UiCombobox
        v-model="etat.supersedesId"
        :label="t('admin.negociations.documents.form.supersedes.label')"
        :hint="t('admin.negociations.documents.form.supersedes.hint')"
        :placeholder="t('admin.negociations.documents.form.supersedes.placeholder')"
        :options="optionsDeRemplacement"
        :error="erreurDe('supersedes_id')"
        :readonly="props.readonly"
      />

      <UiRadio
        :model-value="etat.restricted ? 'restricted' : 'public'"
        :label="t('admin.negociations.documents.form.access.label')"
        :options="optionsDAcces"
        :error="erreurDe('restricted')"
        :readonly="props.readonly"
        @update:model-value="(valeur: string) => (etat.restricted = valeur === 'restricted')"
      />

      <UiCheckbox
        v-model="etat.ragEligible"
        :label="t('admin.negociations.documents.form.rag.label')"
        :hint="t('admin.negociations.documents.form.rag.hint')"
        :readonly="props.readonly"
      />
    </section>

    <template v-if="!props.readonly">
      <UiAlert v-if="echec" intent="danger" live :message="echec.message" />

      <div class="flex flex-wrap gap-3">
        <UiButton type="submit" :loading="props.submitting">
          {{
            props.document
              ? t('admin.negociations.documents.form.submit.save')
              : t('admin.negociations.documents.form.submit.create')
          }}
        </UiButton>
        <slot name="actions" />
      </div>
    </template>
  </form>
</template>
