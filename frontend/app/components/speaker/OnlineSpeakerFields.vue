<script setup lang="ts">
import type { Civility } from '~/types/identity'
import type { OnlineSpeakerInput } from '~/types/programme/online-speaker'
import type { Country } from '~/types/reference'
import type { SelectOption } from '~/types/ui'
import type { OnlineSpeakerField } from '~/utils/speaker-links'

/** Les champs d'un intervenant en ligne, communs à la régie et au lien d'inscription. */

interface Props {
  idPrefix: string
  errors?: Partial<Record<OnlineSpeakerField, string>>
  disabled?: boolean
}

const props = withDefaults(defineProps<Props>(), { errors: () => ({}) })
const model = defineModel<OnlineSpeakerInput>({ required: true })

const { t, locale } = useI18n()
const { tr } = useI18nText()
const api = useApi()

const k = (key: string) => t(`online-speaker-fields.${key}`)
const CIVILITIES: Civility[] = ['mme', 'm', 'dr', 'pr', 'other']

const civilityOptions = computed<SelectOption[]>(() =>
  CIVILITIES.map((code) => ({ value: code, label: t(`proposal.form.step-speakers.civilities.${code}`) })),
)

const countries = ref<Country[]>([])
const countryOptions = computed<SelectOption[]>(() =>
  countries.value
    .filter((c) => c.is_active)
    .map((c) => ({ value: c.iso2, label: tr(c.name) }))
    .sort((a, b) => a.label.localeCompare(b.label, locale.value)),
)

onMounted(async () => {
  try {
    countries.value = await api.reference.countries()
  } catch {
    // Le pays est facultatif : sans la liste, le champ reste vide.
  }
})

function set<K extends keyof OnlineSpeakerInput>(key: K, value: OnlineSpeakerInput[K]): void {
  model.value = { ...model.value, [key]: value }
}

const id = (name: string) => `${props.idPrefix}-${name}`
</script>

<template>
  <div class="grid gap-4">
    <div class="grid gap-4 sm:grid-cols-[8rem_minmax(0,1fr)_minmax(0,1fr)]">
      <UiSelect
        :id="id('civility')"
        :model-value="model.civility"
        :options="civilityOptions"
        :label="k('civility')"
        :disabled="props.disabled"
        @update:model-value="set('civility', ($event || null) as Civility | null)"
      />
      <UiInput
        :id="id('first-name')"
        :model-value="model.first_name"
        :label="k('firstName')"
        autocomplete="given-name"
        required
        :maxlength="120"
        :error="props.errors.first_name"
        :disabled="props.disabled"
        @update:model-value="set('first_name', $event)"
      />
      <UiInput
        :id="id('last-name')"
        :model-value="model.last_name"
        :label="k('lastName')"
        autocomplete="family-name"
        required
        :maxlength="120"
        :error="props.errors.last_name"
        :disabled="props.disabled"
        @update:model-value="set('last_name', $event)"
      />
    </div>
    <div class="grid gap-4 sm:grid-cols-2">
      <UiInput
        :id="id('job')"
        :model-value="model.job_title"
        :label="k('jobTitle')"
        autocomplete="organization-title"
        required
        :maxlength="200"
        :error="props.errors.job_title"
        :disabled="props.disabled"
        @update:model-value="set('job_title', $event)"
      />
      <UiInput
        :id="id('organization')"
        :model-value="model.organization"
        :label="k('organization')"
        autocomplete="organization"
        required
        :maxlength="200"
        :error="props.errors.organization"
        :disabled="props.disabled"
        @update:model-value="set('organization', $event)"
      />
    </div>
    <div class="grid gap-4 sm:grid-cols-2">
      <UiInput
        :id="id('email')"
        :model-value="model.email"
        type="email"
        inputmode="email"
        autocomplete="email"
        :label="k('email')"
        required
        :maxlength="254"
        :error="props.errors.email"
        :disabled="props.disabled"
        @update:model-value="set('email', $event)"
      />
      <UiCombobox
        :id="id('country')"
        :model-value="model.country_iso2"
        :options="countryOptions"
        :label="k('country')"
        :disabled="props.disabled"
        @update:model-value="set('country_iso2', $event || null)"
      />
    </div>
    <UiTextarea
      :id="id('bio')"
      :model-value="model.bio"
      :label="k('bio')"
      :hint="k('bioHint')"
      :rows="3"
      :maxlength="2000"
      :disabled="props.disabled"
      @update:model-value="set('bio', $event)"
    />
  </div>
</template>
