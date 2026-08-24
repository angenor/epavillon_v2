<script setup lang="ts">
import type { SimilarOrganization } from '~/types/org'
import type { CreateOrganizationPayload } from '~/types/organization-join'
import type { SelectOption } from '~/types/ui'

/**
 * Formulaire de création d'une organisation — les sept champs du prompt.
 *
 * IL CONTINUE DE CHERCHER PENDANT QU'ON ÉCRIT, et c'est le point qui compte. La
 * recherche n'est pas une étape franchie une fois pour toutes : quelqu'un qui a
 * cherché « OSED » sans succès et tape maintenant « Observatoire du Sahel pour
 * l'énergie durable » doit voir la fiche apparaître SOUS SES DOIGTS, avant
 * d'avoir fini de remplir. C'est là que le doublon se rattrape — après l'envoi,
 * il est déjà en base. Le site web relance la recherche pour la même raison : un
 * domaine partagé vaut quarante points, et c'est le signal le plus fiable du
 * modèle.
 *
 * `description` est en français seulement à la saisie. La colonne est un
 * `platform.i18n_text` et accepte plusieurs langues ; personne ne rédige deux
 * versions dans un formulaire d'inscription, et la traduction se gère au
 * back-office. On écrit donc `{ fr: … }`, jamais une chaîne nue.
 */

interface Props {
  /** Correspondances trouvées pendant la saisie, tous scores confondus. */
  matches: SimilarOrganization[]
  /** Correspondances FORTES seulement — celles qui déclencheront l'avertissement. */
  strongMatches: SimilarOrganization[]
  countryOptions: SelectOption[]
  typeOptions: SelectOption[]
  searching?: boolean
  submitting?: boolean
  busyJoinId?: string | null
  countryNameOf: (countryId: string | null) => string | null
  typeLabelOf: (code: string) => string | null
  /** Pays du profil : préchargé, mais l'organisation peut être ailleurs. */
  defaultCountryId?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  /** Le nom ou le site a changé : l'appelant relance la recherche. */
  probe: [value: { name: string; website: string; countryId: string }]
  submit: [payload: Omit<CreateOrganizationPayload, 'acknowledged_match_ids'>]
  join: [match: SimilarOrganization]
  cancel: []
}>()

const { t } = useI18n()

const form = reactive({
  legal_name: '',
  acronym: '',
  organization_type_code: '',
  country_id: props.defaultCountryId ?? '',
  city: '',
  website: '',
  description: '',
  job_title: '',
})

const fieldErrors = ref<Record<string, string>>({})

/**
 * Anti-rebond de 300 ms, comme le prescrit le § 5 du modèle (« frappe au
 * clavier, debounce 300 ms »). Sans lui, chaque touche partirait en requête.
 */
let probeTimer: ReturnType<typeof setTimeout> | null = null
watch(
  () => [form.legal_name, form.website, form.country_id],
  () => {
    if (probeTimer) clearTimeout(probeTimer)
    probeTimer = setTimeout(() => {
      emit('probe', {
        name: form.legal_name.trim(),
        website: form.website.trim(),
        countryId: form.country_id,
      })
    }, 300)
  },
)
onBeforeUnmount(() => {
  if (probeTimer) clearTimeout(probeTimer)
})

function validate(): boolean {
  const errors: Record<string, string> = {}
  // Les obligations sont celles de la base : `legal_name` d'au moins deux signes
  // (`ck` du § 1), le type, le pays — et la FONCTION, car le créateur devient
  // référent actif d'emblée et une adhésion active en porte toujours une
  // (`ck_memberships_job_title`). Tout le reste se complète depuis la fiche,
  // plus tard, sans bloquer une inscription.
  if (form.legal_name.trim().length < 2) errors.legal_name = t('organization.join.create.errors.nameTooShort')
  if (form.organization_type_code.length === 0) errors.organization_type_code = t('validation.required')
  if (form.country_id.length === 0) errors.country_id = t('validation.required')
  // Le créateur devient référent ACTIF d'emblée : sa fonction est exigée, comme
  // pour toute adhésion active (`ck_memberships_job_title`).
  if (form.job_title.trim().length === 0) errors.job_title = t('validation.required')
  // `acronym` : 2 à 32 caractères quand il est renseigné — `ck_` du § 1.
  const acronym = form.acronym.trim()
  if (acronym.length > 0 && (acronym.length < 2 || acronym.length > 32)) {
    errors.acronym = t('organization.join.create.errors.acronymLength')
  }
  if (form.website.trim().length > 0 && !/^https?:\/\/\S+\.\S+/.test(form.website.trim())) {
    errors.website = t('validation.url')
  }
  fieldErrors.value = errors
  return Object.keys(errors).length === 0
}

function submit(): void {
  if (!validate()) return
  const description = form.description.trim()
  emit('submit', {
    legal_name: form.legal_name.trim(),
    acronym: form.acronym.trim() || null,
    organization_type_code: form.organization_type_code,
    country_id: form.country_id,
    city: form.city.trim() || null,
    website: form.website.trim() || null,
    description: description ? { fr: description } : null,
    job_title: form.job_title.trim(),
  })
}

/** Ce que l'écran d'avertissement montrera : exposé pour que la page le compose. */
defineExpose({ form })
</script>

<template>
  <section class="grid gap-5">
    <div>
      <h2 class="font-display text-xl font-bold text-text">{{ t('organization.join.create.title') }}</h2>
      <p class="mt-1 text-sm text-text-muted">{{ t('organization.join.create.description') }}</p>
    </div>

    <!-- L'avertissement vit AU-DESSUS du formulaire, pas dans un dialogue à la
         soumission : le but est qu'il soit lu pendant qu'on hésite encore. -->
    <UiAlert
      v-if="props.strongMatches.length > 0"
      intent="warning"
      :title="t('organization.join.create.probeWarning.title')"
      :message="t('organization.join.create.probeWarning.description')"
    />

    <form class="grid gap-4" novalidate @submit.prevent="submit">
      <UiInput
        v-model="form.legal_name"
        icon="building"
        :label="t('organization.join.create.fields.legalName')"
        :hint="t('organization.join.create.fields.legalNameHint')"
        :error="fieldErrors.legal_name"
        :disabled="props.submitting"
        required
      />

      <div class="grid gap-4 sm:grid-cols-2">
        <UiInput
          v-model="form.acronym"
          :label="t('organization.join.create.fields.acronym')"
          :hint="t('organization.join.create.fields.acronymHint')"
          :error="fieldErrors.acronym"
          :disabled="props.submitting"
          :maxlength="32"
        />
        <UiSelect
          v-model="form.organization_type_code"
          :options="props.typeOptions"
          :label="t('organization.join.create.fields.type')"
          :placeholder="t('organization.join.create.fields.typePlaceholder')"
          :error="fieldErrors.organization_type_code"
          :disabled="props.submitting"
          required
        />
      </div>

      <div class="grid gap-4 sm:grid-cols-2">
        <UiSelect
          v-model="form.country_id"
          :options="props.countryOptions"
          :label="t('organization.join.create.fields.country')"
          :placeholder="t('organization.join.create.fields.countryPlaceholder')"
          :error="fieldErrors.country_id"
          :disabled="props.submitting"
          required
        />
        <UiInput
          v-model="form.city"
          :label="t('organization.join.create.fields.city')"
          :disabled="props.submitting"
        />
      </div>

      <UiInput
        v-model="form.website"
        type="url"
        inputmode="url"
        placeholder="https://"
        :label="t('organization.join.create.fields.website')"
        :hint="t('organization.join.create.fields.websiteHint')"
        :error="fieldErrors.website"
        :disabled="props.submitting"
      />

      <UiTextarea
        v-model="form.description"
        :rows="4"
        :maxlength="600"
        :label="t('organization.join.create.fields.description')"
        :hint="t('organization.join.create.fields.descriptionHint')"
        :disabled="props.submitting"
      />

      <UiInput
        v-model="form.job_title"
        :label="t('organization.join.create.fields.jobTitle')"
        :hint="t('organization.join.create.fields.jobTitleHint')"
        :disabled="props.submitting"
        :error="fieldErrors.job_title"
        required
        :maxlength="120"
      />

      <div class="flex flex-col gap-3 sm:flex-row-reverse sm:justify-start">
        <UiButton
          type="submit"
          variant="primary"
          size="lg"
          :loading="props.submitting"
          :label="t('organization.join.create.submit')"
        />
        <UiButton
          variant="ghost"
          icon="chevron-left"
          :label="t('organization.join.create.backToSearch')"
          @click="emit('cancel')"
        />
      </div>
    </form>

    <!-- Les correspondances trouvées en cours de saisie, TOUTES et non les seules
         fortes : c'est le moment où elles servent encore à quelque chose. -->
    <section v-if="props.matches.length > 0" class="grid gap-3 border-t border-border pt-5">
      <h3 class="flex items-center gap-2 font-display text-sm text-text-secondary">
        {{ t('organization.join.create.probeResults') }}
        <UiSpinner v-if="props.searching" size="0.9rem" />
      </h3>
      <OrganizationMatchCard
        v-for="match in props.matches"
        :key="match.organization_id"
        :match="match"
        :country-name="props.countryNameOf(match.country_id)"
        :type-label="props.typeLabelOf(match.organization_type_code)"
        :busy="props.busyJoinId === match.organization_id"
        @join="emit('join', $event)"
      />
    </section>
  </section>
</template>
