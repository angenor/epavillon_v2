<script setup lang="ts">
import type { Civility, UpdateProfilePayload } from '~/types/identity'
import type { AttachedImage, AttachableRoleRule } from '~/types/media'
import type { Country } from '~/types/reference'
import type { SelectOption } from '~/types/ui'
import type { AssetId } from '~/types/shared'

/**
 * PROFIL — `/mon-profil`, `/en/my-profile` (30/10).
 *
 * Ce que la personne modifie d'elle-même : identité, coordonnées, langue,
 * fuseau, visibilité dans l'annuaire, photo. L'adresse principale se lit sans
 * se changer (une nouvelle adresse demande une vérification) ; le mot de passe
 * se change par le lien envoyé à cette adresse, comme un oubli.
 */

definePageMeta({ layout: 'public', middleware: ['auth'] })

defineI18nRoute({ paths: { fr: '/mon-profil', en: '/my-profile' } })

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()
const localePath = useLocalePath()
const api = useApi()
const auth = useAuthStore()
const memberships = useMembershipStore()

useHead(() => ({ title: t('account.profile.title') }))

await memberships.ensureLoaded()

const CIVILITIES: Civility[] = ['mme', 'm', 'dr', 'pr', 'other']
const LOCALES = ['fr', 'en'] as const

const civilityOptions = computed<SelectOption[]>(() =>
  CIVILITIES.map((code) => ({ value: code, label: t(`proposal.form.step-speakers.civilities.${code}`) })),
)

const localeOptions = computed<SelectOption[]>(() =>
  LOCALES.map((code) => ({ value: code, label: t(`account.profile.locales.${code}`) })),
)

const { data: countries } = useAsyncData<Country[]>('reference-countries', () => api.reference.countries(), {
  lazy: true,
})

const countryOptions = computed<SelectOption[]>(() =>
  (countries.value ?? [])
    .filter((country) => country.is_active)
    .map((country) => ({ value: country.id, label: tr(country.name) }))
    .sort((a, b) => a.label.localeCompare(b.label, locale.value)),
)

/** Les fuseaux que connaît le navigateur ; la base refuse ce qu'elle ne connaît pas. */
const timezoneOptions = computed<SelectOption[]>(() =>
  Intl.supportedValuesOf('timeZone').map((zone) => ({ value: zone, label: zone.replaceAll('_', ' ') })),
)

// ---------------------------------------------------------------------------
// Le formulaire part de la personne connectée
// ---------------------------------------------------------------------------

function formOf(): UpdateProfilePayload | null {
  const person = auth.person
  if (!person) return null
  return {
    civility: person.civility,
    first_name: person.first_name,
    last_name: person.last_name,
    job_title: person.job_title,
    phone: person.phone,
    country_id: person.country_id,
    city: person.city,
    preferred_locale: person.preferred_locale,
    timezone: person.timezone,
    is_directory_visible: person.is_directory_visible,
  }
}

const form = ref<UpdateProfilePayload | null>(formOf())
const saving = ref(false)
const message = ref<{ intent: 'success' | 'danger'; text: string } | null>(null)

async function save(): Promise<void> {
  const person = auth.person
  if (!person || !form.value) return
  saving.value = true
  message.value = null
  try {
    auth.person = { ...person, ...(await api.identity.updateProfile(person.id, form.value)) }
    form.value = formOf()
    message.value = { intent: 'success', text: t('account.profile.saved') }
  } catch (thrown) {
    message.value = { intent: 'danger', text: apiErrorMessage(thrown, (key) => t(key)) }
  } finally {
    saving.value = false
  }
}

// ---------------------------------------------------------------------------
// La photo : rôle `avatar` sur la personne, qu'elle seule rattache
// ---------------------------------------------------------------------------

const owner = computed(() => ({ schema: 'identity', table: 'people', id: auth.person?.id ?? '' }))

const { data: photo, refresh: refreshPhoto } = useAsyncData(
  'profile-avatar',
  async () => {
    if (!auth.person) return { rule: null, url: null }
    const [rules, attached] = await Promise.all([
      api.media.roles('identity', 'people'),
      api.media.attached(owner.value, 'avatar'),
    ])
    const rule: AttachableRoleRule | null = rules.find((entry) => entry.role === 'avatar') ?? null
    const current = attached[0]
    return { rule, url: current ? (current.sources.thumb?.url ?? current.url) : null }
  },
  { lazy: true, server: false },
)

const uploaded = ref<AttachedImage | null>(null)
const photoUrl = computed(() => uploaded.value?.url ?? photo.value?.url ?? null)
const photoError = ref<string | null>(null)

async function attachPhoto(assetId: AssetId | null): Promise<void> {
  if (!assetId || !auth.person) return
  photoError.value = null
  try {
    await api.media.attach({
      owner_schema: 'identity',
      owner_table: 'people',
      owner_id: auth.person.id,
      assignments: [{ role: 'avatar', asset_id: assetId }],
    })
    await refreshPhoto()
  } catch (thrown) {
    photoError.value = apiErrorMessage(thrown, (key) => t(key))
  }
}

// ---------------------------------------------------------------------------
// Adresse et mot de passe : deux liens envoyés par courriel
// ---------------------------------------------------------------------------

const mailSent = ref<'verification' | 'password' | null>(null)
const mailing = ref(false)

async function sendMail(kind: 'verification' | 'password'): Promise<void> {
  const email = auth.person?.primary_email
  if (!email) return
  mailing.value = true
  try {
    if (kind === 'verification') await api.auth.resendVerification(email)
    else await api.auth.requestPasswordReset(email)
    mailSent.value = kind
  } catch (thrown) {
    message.value = { intent: 'danger', text: apiErrorMessage(thrown, (key) => t(key)) }
  } finally {
    mailing.value = false
  }
}

const organizations = computed(() => memberships.submittable.map((entry) => entry.organization))
</script>

<template>
  <div class="mx-auto w-full max-w-4xl px-4 py-8 sm:px-6 lg:px-8">
    <UiForbiddenState v-if="!auth.person || !form" class="mt-10" />

    <template v-else>
      <header class="flex flex-wrap items-center gap-5">
        <span class="grid size-20 shrink-0 place-items-center overflow-hidden rounded-full border border-accent-border bg-accent-surface text-2xl font-bold text-accent">
          <img v-if="photoUrl" :src="photoUrl" alt="" class="size-full object-cover">
          <span v-else aria-hidden="true">{{ initialsOf(auth.person.display_name) }}</span>
        </span>
        <div class="min-w-0 flex-1 basis-48">
          <p class="text-sm text-text-subtle">{{ t('account.profile.title') }}</p>
          <h1 class="mt-1 text-3xl leading-tight font-semibold text-balance">{{ auth.person.display_name }}</h1>
          <p class="mt-1 text-sm break-all text-text-muted">{{ auth.person.primary_email }}</p>
        </div>
        <MediaImageField
          v-if="photo?.rule"
          class="w-full sm:w-auto"
          role="avatar"
          :asset-id="null"
          :label="t('account.profile.photo.label')"
          :button-label="photoUrl ? t('account.profile.photo.change') : t('account.profile.photo.add')"
          :alt-fallback="auth.person.display_name"
          :rule="photo.rule"
          :owner="owner"
          @update:image="(image: AttachedImage | null) => (uploaded = image)"
          @update:asset-id="attachPhoto"
        />
      </header>
      <UiAlert v-if="photoError" class="mt-4" intent="danger" :message="photoError" live />

      <form class="mt-10 rounded-lg border border-border bg-surface-raised p-5 sm:p-6" @submit.prevent="save()">
        <h2 class="text-xl font-semibold">{{ t('account.profile.identity.title') }}</h2>

        <div class="mt-5 grid gap-5 sm:grid-cols-2">
          <UiSelect
            id="profile-civility"
            v-model="form.civility"
            :options="civilityOptions"
            :label="t('account.profile.fields.civility')"
          />
          <div class="max-sm:hidden" />
          <UiInput id="profile-first-name" v-model="form.first_name" :label="t('account.profile.fields.firstName')" required />
          <UiInput id="profile-last-name" v-model="form.last_name" :label="t('account.profile.fields.lastName')" required />
          <UiInput id="profile-job" v-model="form.job_title" :label="t('account.profile.fields.jobTitle')" />
          <UiInput id="profile-phone" v-model="form.phone" type="tel" :label="t('account.profile.fields.phone')" />
          <UiCombobox
            id="profile-country"
            v-model="form.country_id"
            :options="countryOptions"
            :label="t('account.profile.fields.country')"
          />
          <UiInput id="profile-city" v-model="form.city" :label="t('account.profile.fields.city')" />
          <UiSelect
            id="profile-locale"
            :model-value="form.preferred_locale"
            :options="localeOptions"
            :label="t('account.profile.fields.locale')"
            :hint="t('account.profile.fields.localeHint')"
            required
            hide-optional
            @update:model-value="form.preferred_locale = $event ?? 'fr'"
          />
          <UiCombobox
            id="profile-timezone"
            :model-value="form.timezone"
            :options="timezoneOptions"
            :label="t('account.profile.fields.timezone')"
            :hint="t('account.profile.fields.timezoneHint')"
            required
            hide-optional
            @update:model-value="form.timezone = $event ?? form.timezone"
          />
        </div>

        <div class="mt-6 border-t border-border-subtle pt-5">
          <UiSwitch
            id="profile-directory"
            v-model="form.is_directory_visible"
            :label="t('account.profile.fields.directory')"
            :hint="t('account.profile.fields.directoryHint')"
          />
        </div>

        <UiAlert v-if="message" class="mt-5" :intent="message.intent" :message="message.text" live />

        <div class="mt-6 flex justify-end">
          <UiButton type="submit" :loading="saving">{{ t('account.profile.save') }}</UiButton>
        </div>
      </form>

      <section class="mt-8 rounded-lg border border-border bg-surface-raised p-5 sm:p-6" aria-labelledby="profile-access">
        <h2 id="profile-access" class="text-xl font-semibold">{{ t('account.profile.access.title') }}</h2>

        <div class="mt-5 flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div class="min-w-0">
            <p class="font-semibold text-text">{{ t('account.profile.access.email') }}</p>
            <p class="mt-1 text-sm text-text-muted">
              {{ auth.person.primary_email }} ·
              {{
                auth.person.email_verified_at
                  ? t('account.profile.access.verifiedOn', { date: date(auth.person.email_verified_at, auth.person.timezone) })
                  : t('account.profile.access.unverified')
              }}
            </p>
            <p class="mt-1 text-xs text-text-subtle">{{ t('account.profile.access.emailHint') }}</p>
          </div>
          <UiButton
            v-if="!auth.person.email_verified_at"
            variant="secondary"
            :loading="mailing"
            @click="sendMail('verification')"
          >
            {{ t('account.profile.access.verify') }}
          </UiButton>
        </div>

        <div class="mt-5 flex flex-col gap-4 border-t border-border-subtle pt-5 sm:flex-row sm:items-center sm:justify-between">
          <div class="min-w-0">
            <p class="font-semibold text-text">{{ t('account.profile.access.password') }}</p>
            <p class="mt-1 text-sm text-text-muted">{{ t('account.profile.access.passwordHint') }}</p>
          </div>
          <UiButton variant="secondary" :loading="mailing" @click="sendMail('password')">
            {{ t('account.profile.access.passwordAction') }}
          </UiButton>
        </div>

        <UiAlert
          v-if="mailSent"
          class="mt-5"
          intent="success"
          :message="t(`account.profile.access.sent.${mailSent}`, { email: auth.person.primary_email })"
          live
        />
      </section>

      <section class="mt-8 rounded-lg border border-border bg-surface-raised p-5 sm:p-6" aria-labelledby="profile-organizations">
        <h2 id="profile-organizations" class="text-xl font-semibold">{{ t('account.profile.organizations.title') }}</h2>
        <ul v-if="organizations.length > 0" class="mt-4 divide-y divide-border-subtle">
          <li v-for="organization in organizations" :key="organization.id" class="flex flex-wrap items-center justify-between gap-3 py-3">
            <span class="font-semibold text-text">
              {{ organization.legal_name }}<template v-if="organization.acronym"> ({{ organization.acronym }})</template>
            </span>
            <UiButton
              variant="ghost"
              icon-trailing="arrow-right"
              :to="localePath({ path: localePath('/mon-organisation'), query: { organisation: organization.id } })"
            >
              {{ t('account.profile.organizations.open') }}
            </UiButton>
          </li>
        </ul>
        <div v-else class="mt-4 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <p class="text-sm text-text-muted">{{ t('account.profile.organizations.none') }}</p>
          <UiButton variant="secondary" :to="localePath('/rattachement-organisation')">
            {{ t('account.profile.organizations.join') }}
          </UiButton>
        </div>
      </section>
    </template>
  </div>
</template>
