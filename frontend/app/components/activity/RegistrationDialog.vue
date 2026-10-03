<script setup lang="ts">
import type { Country } from '~/types/reference'
import type {
  RegistrationForm,
  RegistrationFormField,
  RegistrationGuest,
  RegistrationResult,
} from '~/types/programme/registration'
import type { Intent, SelectOption } from '~/types/ui'
import type { TimeZoneName } from '~/types/shared'
import { ApiRequestError, apiErrorMessage, incidentReference, isForbiddenError } from '~/utils/api-error'
import { timeZoneCountryIso2 } from '~/utils/timezone-country'
import {
  champsAffiches,
  champsSansReponse,
  consentementRequis,
  reponsesDeLaSaisie,
  saisieInitiale,
  type SaisieDuFormulaire,
} from '~/utils/guide-nego/pavillon'

interface Props {
  sessionId: string
  sessionTitle: string
  open: boolean
  /** Fuseau de l'édition, pour dater une ouverture ou une clôture ; à défaut, celui du navigateur. */
  timezone?: TimeZoneName
  timezoneLabel?: string
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:open': [value: boolean]; done: [result: RegistrationResult] }>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { dateTime, zoneLabel } = useDateTime()
const api = useApi()
const auth = useAuthStore()
const route = useRoute()
const localePath = useLocalePath()

const k = (key: string, params: Record<string, unknown> = {}) => t(`registration-dialog.${key}`, params)

type Phase = 'loading' | 'error' | 'forbidden' | 'login' | 'form' | 'result'

const phase = ref<Phase>('loading')
const loadError = ref<string | null>(null)
const loadRequestId = ref<string | null>(null)
const form = ref<RegistrationForm | null>(null)
const fields = ref<RegistrationFormField[]>([])
const countries = ref<Country[]>([])
const entries = ref<SaisieDuFormulaire>({})
const guest = reactive<RegistrationGuest>({ first_name: '', last_name: '', email: '' })
const consent = ref(false)
const countryGuessed = ref(false)
const fieldErrors = ref<Record<string, string>>({})
const submitError = ref<string | null>(null)
const submitting = ref(false)
const result = ref<RegistrationResult | null>(null)

const formId = useId()
const CONSENT = '__consent'
const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/

const visibleFields = computed(() => champsAffiches(fields.value))
const isGuest = computed(() => !auth.isAuthenticated)
const loginTo = computed(
  () => `${localePath('auth-login')}?redirect=${encodeURIComponent(route.fullPath)}`,
)
const answers = computed(() => reponsesDeLaSaisie(fields.value, entries.value))
const needsConsent = computed(() => consentementRequis(fields.value, answers.value))

const countryOptions = computed<SelectOption[]>(() =>
  countries.value
    .filter((c) => c.is_active)
    .map((c) => ({ value: c.iso2, label: tr(c.name) }))
    .sort((a, b) => a.label.localeCompare(b.label, locale.value)),
)

async function load(): Promise<void> {
  phase.value = 'loading'
  loadError.value = null
  loadRequestId.value = null
  try {
    const [applicable] = await Promise.all([api.registrations.form(props.sessionId), auth.ensureLoaded()])
    if (!applicable) throw new ApiRequestError({ code: 'NOT_FOUND', message: k('notFound') }, 404)
    form.value = applicable.form
    fields.value = applicable.fields
    if (applicable.fields.some((f) => f.field_type === 'country')) await loadCountries()
    reset()
    phase.value = isGuest.value && !applicable.form.allows_anonymous ? 'login' : 'form'
  } catch (error) {
    if (isForbiddenError(error)) {
      phase.value = 'forbidden'
      return
    }
    loadError.value = apiErrorMessage(error, t)
    loadRequestId.value = incidentReference(error)
    phase.value = 'error'
  }
}

async function loadCountries(): Promise<void> {
  if (countries.value.length) return
  try {
    countries.value = await api.reference.countries()
  } catch {
    // Sans référentiel, le pays se saisit par son code à deux lettres.
  }
}

function initialCountry(): string | null {
  const ofPerson = countries.value.find((c) => c.id === auth.person?.country_id)?.iso2
  countryGuessed.value = false
  if (ofPerson) return ofPerson
  const browserZone = Intl.DateTimeFormat().resolvedOptions().timeZone
  const guessed = browserZone ? timeZoneCountryIso2(browserZone) : null
  if (!guessed || !countryOptions.value.some((o) => o.value === guessed)) return null
  countryGuessed.value = true
  return guessed
}

function reset(): void {
  entries.value = saisieInitiale(fields.value, initialCountry())
  const person = auth.person
  Object.assign(
    guest,
    person
      ? { first_name: person.first_name, last_name: person.last_name, email: person.primary_email }
      : { first_name: '', last_name: '', email: '' },
  )
  consent.value = false
  fieldErrors.value = {}
  submitError.value = null
  result.value = null
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen && !submitting.value) void load()
  },
  { immediate: true },
)

function close(): void {
  emit('update:open', false)
}

function clearError(code: string): void {
  if (!(code in fieldErrors.value)) return
  const { [code]: _cleared, ...rest } = fieldErrors.value
  fieldErrors.value = rest
}

function setEntry(code: string, value: string | string[] | boolean): void {
  entries.value = { ...entries.value, [code]: value }
  if (fields.value.some((f) => f.code === code && f.field_type === 'country')) countryGuessed.value = false
  clearError(code)
}

function toggleChoice(code: string, value: string, checked: boolean): void {
  const others = checkedValues(code).filter((v) => v !== value)
  setEntry(code, checked ? [...others, value] : others)
}

const textOf = (code: string) => {
  const v = entries.value[code]
  return typeof v === 'string' ? v : ''
}
const checkedValues = (code: string) => {
  const v = entries.value[code]
  return Array.isArray(v) ? v : []
}

type Kind = 'input' | 'textarea' | 'date' | 'select' | 'radio' | 'checkboxes' | 'checkbox' | 'country' | 'country-code'

const RADIO_MAX = 5

function kindOf(field: RegistrationFormField): Kind {
  switch (field.field_type) {
    case 'long_text':
      return 'textarea'
    case 'date':
      return 'date'
    case 'boolean':
      return 'checkbox'
    case 'multiple_choice':
      return 'checkboxes'
    case 'country':
      return countryOptions.value.length ? 'country' : 'country-code'
    case 'taxonomy_term':
      return 'select'
    case 'single_choice':
      return optionsOf(field).length <= RADIO_MAX ? 'radio' : 'select'
    default:
      return 'input'
  }
}

const INPUT_TYPES = { email: 'email', phone: 'tel', number: 'number' } as const
const inputType = (field: RegistrationFormField) =>
  INPUT_TYPES[field.field_type as keyof typeof INPUT_TYPES] ?? 'text'
const AUTOCOMPLETE: Partial<Record<RegistrationFormField['field_type'], string>> = { email: 'email', phone: 'tel' }

function optionsOf(field: RegistrationFormField): SelectOption[] {
  const values = 'values' in field.options && Array.isArray(field.options.values) ? field.options.values : []
  return values.map((o) => ({ value: o.value, label: tr(o.label) }))
}

function maxLengthOf(field: RegistrationFormField): number | undefined {
  const max = field.validation.maxLength
  return typeof max === 'number' && max > 0 ? max : undefined
}

const hintOf = (field: RegistrationFormField) => {
  if (field.help_text) return tr(field.help_text)
  return field.field_type === 'country' && countryGuessed.value ? k('countryGuessed') : undefined
}

function validate(): boolean {
  const errors: Record<string, string> = Object.fromEntries(
    champsSansReponse(fields.value, answers.value).map((code) => [code, t('validation.required')]),
  )
  for (const field of visibleFields.value) {
    const value = answers.value[field.code]
    if (field.field_type === 'email' && typeof value === 'string' && !EMAIL_PATTERN.test(value)) {
      errors[field.code] = t('validation.email')
    }
  }
  if (isGuest.value) {
    if (!guest.first_name.trim()) errors['guest.first_name'] = t('validation.required')
    if (!guest.last_name.trim()) errors['guest.last_name'] = t('validation.required')
    if (!guest.email.trim()) errors['guest.email'] = t('validation.required')
    else if (!EMAIL_PATTERN.test(guest.email.trim())) errors['guest.email'] = t('validation.email')
  }
  if (needsConsent.value && !consent.value) errors[CONSENT] = k('consent.required')
  fieldErrors.value = errors
  return Object.keys(errors).length === 0
}

async function submit(): Promise<void> {
  if (submitting.value) return
  submitError.value = null
  if (!validate()) return

  submitting.value = true
  try {
    const outcome = await api.pavillon.inscrire(props.sessionId, {
      answers: answers.value,
      locale: locale.value,
      sensitive_data_consent: needsConsent.value ? consent.value : false,
      guest: isGuest.value
        ? { first_name: guest.first_name.trim(), last_name: guest.last_name.trim(), email: guest.email.trim() }
        : null,
    })
    result.value = outcome
    phase.value = 'result'
    emit('done', outcome)
  } catch (error) {
    showSubmitError(error)
  } finally {
    submitting.value = false
  }
}

function showSubmitError(error: unknown): void {
  const message = apiErrorMessage(error, t)
  if (!(error instanceof ApiRequestError)) {
    submitError.value = message
    return
  }
  if (error.code === 'REGISTRATION_ACCOUNT_REQUIRED') {
    phase.value = 'login'
    return
  }
  if (error.code === 'REGISTRATION_CONSENT_REQUIRED' && needsConsent.value) {
    fieldErrors.value = { ...fieldErrors.value, [CONSENT]: message }
    return
  }
  if (error.field && visibleFields.value.some((f) => f.code === error.field)) {
    fieldErrors.value = { ...fieldErrors.value, [error.field]: message }
    return
  }
  submitError.value = message
}

const zone = computed<TimeZoneName>(
  () => props.timezone ?? (Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC'),
)
const when = (value: string) =>
  k('dateWithZone', { date: dateTime(value, zone.value), zone: zoneLabel(zone.value, props.timezoneLabel) })

const outcome = computed<{ intent: Intent; title: string; message: string } | null>(() => {
  const r = result.value
  if (!r) return null
  const title = props.sessionTitle
  switch (r.status) {
    case 'registered':
      return { intent: 'success', title: k('outcome.registered.title'), message: k('outcome.registered.message', { title }) }
    case 'waitlisted':
      return {
        intent: 'warning',
        title: k('outcome.waitlisted.title'),
        message: k('outcome.waitlisted.message', { position: r.position }),
      }
    case 'already_registered':
      return r.registration.status === 'waitlisted'
        ? {
            intent: 'warning',
            title: k('outcome.already.title'),
            message: k('outcome.already.waitlisted', { position: r.registration.waitlist_position ?? '—' }),
          }
        : { intent: 'info', title: k('outcome.already.title'), message: k('outcome.already.registered', { title }) }
    case 'full':
      return { intent: 'neutral', title: k('outcome.full.title'), message: k('outcome.full.message', { capacity: r.capacity }) }
    case 'closed':
      return { intent: 'neutral', title: k('outcome.closed.title'), message: k('outcome.closed.message', { date: when(r.closed_at) }) }
    case 'not_open_yet':
      return { intent: 'info', title: k('outcome.notOpen.title'), message: k('outcome.notOpen.message', { date: when(r.opens_at) }) }
  }
  return null
})
</script>

<template>
  <UiModal
    :open="open"
    :title="k('title')"
    :description="sessionTitle"
    :description-lines="2"
    :dismissible="!submitting"
    @update:open="emit('update:open', $event)"
  >
    <UiLoadingState v-if="phase === 'loading'" variant="form" :lines="4" :label="k('loading')" />

    <UiErrorState
      v-else-if="phase === 'error'"
      compact
      :title="k('loadError')"
      :description="loadError ?? undefined"
      :request-id="loadRequestId ?? undefined"
      @retry="load"
    />

    <UiForbiddenState v-else-if="phase === 'forbidden'" compact />

    <UiAlert v-else-if="phase === 'login'" intent="info" :title="k('login.title')" :message="k('login.message')">
      <template #actions>
        <UiButton :to="loginTo">{{ k('login.action') }}</UiButton>
      </template>
    </UiAlert>

    <div v-else-if="phase === 'result' && outcome" aria-live="polite">
      <UiAlert :intent="outcome.intent" :title="outcome.title" :message="outcome.message" />
    </div>

    <form
      v-else-if="phase === 'form'"
      :id="formId"
      class="grid gap-5"
      novalidate
      @submit.prevent="submit"
    >
      <p v-if="isGuest" class="max-w-(--measure) text-sm text-text-muted">
        {{ k('guest.intro') }}
        <NuxtLink :to="loginTo" class="font-bold text-text-link underline underline-offset-4">
          {{ k('guest.signIn') }}
        </NuxtLink>
      </p>
      <p v-else class="max-w-(--measure) text-sm text-text-muted">{{ k('account.intro') }}</p>

      <div class="grid gap-4 sm:grid-cols-2">
        <UiInput
          v-model="guest.last_name"
          :label="k('guest.lastName')"
          autocomplete="family-name"
          :readonly="!isGuest"
          :error="fieldErrors['guest.last_name']"
          required
          @update:model-value="clearError('guest.last_name')"
        />
        <UiInput
          v-model="guest.first_name"
          :label="k('guest.firstName')"
          autocomplete="given-name"
          :readonly="!isGuest"
          :error="fieldErrors['guest.first_name']"
          required
          @update:model-value="clearError('guest.first_name')"
        />
      </div>
      <UiInput
        v-model="guest.email"
        type="email"
        :label="k('guest.email')"
        :hint="isGuest ? k('guest.emailHint') : undefined"
        autocomplete="email"
        :readonly="!isGuest"
        :error="fieldErrors['guest.email']"
        required
        @update:model-value="clearError('guest.email')"
      />

      <template v-for="field in visibleFields" :key="field.code">
        <UiTextarea
          v-if="kindOf(field) === 'textarea'"
          :model-value="textOf(field.code)"
          :label="tr(field.label)"
          :hint="hintOf(field)"
          :error="fieldErrors[field.code]"
          :required="field.is_required"
          :maxlength="maxLengthOf(field)"
          :rows="3"
          auto-grow
          @update:model-value="setEntry(field.code, $event)"
        />

        <UiDatePicker
          v-else-if="kindOf(field) === 'date'"
          :model-value="textOf(field.code)"
          :label="tr(field.label)"
          :hint="hintOf(field)"
          :error="fieldErrors[field.code]"
          :required="field.is_required"
          @update:model-value="setEntry(field.code, $event)"
        />

        <UiSelect
          v-else-if="kindOf(field) === 'select' || kindOf(field) === 'country'"
          :model-value="textOf(field.code)"
          :options="kindOf(field) === 'country' ? countryOptions : optionsOf(field)"
          :label="tr(field.label)"
          :hint="hintOf(field)"
          :placeholder="k('choose')"
          :error="fieldErrors[field.code]"
          :required="field.is_required"
          @update:model-value="setEntry(field.code, $event)"
        />

        <UiRadio
          v-else-if="kindOf(field) === 'radio'"
          :model-value="textOf(field.code) || null"
          :options="optionsOf(field)"
          :label="tr(field.label)"
          :hint="hintOf(field)"
          :error="fieldErrors[field.code]"
          :required="field.is_required"
          @update:model-value="setEntry(field.code, $event)"
        />

        <UiCheckbox
          v-else-if="kindOf(field) === 'checkbox'"
          :model-value="entries[field.code] === true"
          :label="tr(field.label)"
          :hint="hintOf(field)"
          :error="fieldErrors[field.code]"
          :required="field.is_required"
          @update:model-value="setEntry(field.code, $event)"
        />

        <fieldset
          v-else-if="kindOf(field) === 'checkboxes'"
          :aria-invalid="fieldErrors[field.code] ? true : undefined"
          :aria-required="field.is_required ? true : undefined"
        >
          <legend class="mb-1 max-w-(--measure) text-sm font-bold text-text">
            {{ tr(field.label) }}
            <span v-if="field.is_required" class="ml-0.5 text-danger" aria-hidden="true">*</span>
            <span v-if="field.is_required" class="sr-only"> — {{ t('form.required') }}</span>
          </legend>
          <p v-if="hintOf(field)" class="max-w-(--measure) text-sm text-text-muted">{{ hintOf(field) }}</p>
          <UiCheckbox
            v-for="option in optionsOf(field)"
            :key="option.value"
            :model-value="checkedValues(field.code).includes(option.value)"
            :label="option.label"
            @update:model-value="toggleChoice(field.code, option.value, $event)"
          />
          <p v-if="fieldErrors[field.code]" role="alert" class="mt-1.5 flex items-start gap-1.5 text-sm font-bold text-danger">
            <UiIcon name="error" size="1.05em" class="mt-0.5" />
            <span>{{ fieldErrors[field.code] }}</span>
          </p>
        </fieldset>

        <UiInput
          v-else-if="kindOf(field) === 'country-code'"
          :model-value="textOf(field.code)"
          :label="tr(field.label)"
          :hint="k('countryCode')"
          :error="fieldErrors[field.code]"
          :required="field.is_required"
          :maxlength="2"
          autocomplete="country"
          @update:model-value="setEntry(field.code, $event)"
        />

        <UiInput
          v-else
          :model-value="textOf(field.code)"
          :type="inputType(field)"
          :label="tr(field.label)"
          :hint="hintOf(field)"
          :error="fieldErrors[field.code]"
          :required="field.is_required"
          :maxlength="maxLengthOf(field)"
          :autocomplete="AUTOCOMPLETE[field.field_type] ?? 'off'"
          :inputmode="field.field_type === 'number' ? 'decimal' : undefined"
          @update:model-value="setEntry(field.code, $event)"
        />
      </template>

      <UiCheckbox
        v-if="needsConsent"
        v-model="consent"
        :label="k('consent.label')"
        :hint="k('consent.hint')"
        :error="fieldErrors[CONSENT]"
        required
        @update:model-value="clearError(CONSENT)"
      />

      <UiAlert v-if="submitError" intent="danger" :message="submitError" live />
    </form>

    <template #footer>
      <template v-if="phase === 'form'">
        <UiButton variant="ghost" :disabled="submitting" @click="close">{{ t('common.actions.cancel') }}</UiButton>
        <UiButton type="submit" :form="formId" :loading="submitting">{{ k('submit') }}</UiButton>
      </template>
      <UiButton v-else variant="secondary" :disabled="submitting" @click="close">{{ t('common.actions.close') }}</UiButton>
    </template>
  </UiModal>
</template>
