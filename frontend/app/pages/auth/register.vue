<script setup lang="ts">
import type { Country } from '~/types/reference'
import type { SelectOption } from '~/types/ui'
import { MIN_PASSWORD_LENGTH, evaluatePassword } from '~/utils/password-strength'

/**
 * Création de compte — `/inscription`, `/en/register`.
 *
 * LE STRICT MINIMUM, ET C'EST UNE DÉCISION DE PRODUIT. Cinq champs : prénom,
 * nom, adresse, pays, mot de passe. Rien sur l'organisation — le rattachement
 * est une étape à part entière (prompt A2), et pour une bonne raison : c'est
 * l'écran dont dépend la qualité de tout le référentiel. Le noyer ici, dans un
 * formulaire qu'on remplit vite, c'est fabriquer les doublons que la v1 a payés
 * cher. `people.primary_organization_id` reste donc nul à la création.
 *
 * DEUX COLONNES REMPLIES SANS ÊTRE DEMANDÉES. `people.preferred_locale` prend la
 * langue de l'interface au moment de l'inscription, `people.timezone` celui que
 * déclare le navigateur — toutes deux sont `NOT NULL` dans le modèle, toutes
 * deux se corrigent depuis le profil. Deux champs de formulaire en moins.
 *
 * AUCUNE ANNONCE DE DOUBLON. Le formulaire ne dit jamais qu'une adresse est déjà
 * prise : l'API répond la même chose dans les deux cas et envoie un courriel
 * différent. Sans cela, ce formulaire serait le moyen le plus simple de savoir
 * qui, dans une délégation, a un compte sur la plateforme.
 *
 * ÉTATS — le référentiel des pays est la seule donnée chargée : il porte donc le
 * chargement (squelette), l'erreur (reprise) et le vide (référentiel introuvable,
 * cas dans lequel le formulaire ne peut pas être rempli honnêtement).
 */

definePageMeta({ layout: 'auth', middleware: 'guest' })
defineI18nRoute({ paths: { fr: '/inscription', en: '/register' } })

const { t, locale } = useI18n()
const { tr } = useI18nText()
const localePath = useLocalePath()
const router = useRouter()
const api = useApi()
const auth = useAuthStore()

useHead(() => ({ title: t('auth.register.title') }))

/**
 * `lazy` et sans `await` : la navigation ne se bloque pas sur le référentiel, et
 * l'écran montre son squelette pendant le chargement. Attendre ici afficherait
 * la page toute faite après un temps mort — l'état de chargement existerait dans
 * le code sans jamais se voir.
 */
const {
  data: countries,
  status: countriesStatus,
  error: countriesError,
  refresh: reloadCountries,
} = useAsyncData<Country[]>('reference-countries', () => api.reference.countries(), { lazy: true })

/**
 * Les pays sont triés dans la LANGUE AFFICHÉE, pas dans l'ordre de la base :
 * « Allemagne » et « Germany » ne se rangent pas au même endroit. `localeCompare`
 * place aussi les accents là où on les cherche — « Côte d'Ivoire » après
 * « Congo », pas à la fin de la liste.
 */
const countryOptions = computed<SelectOption[]>(() =>
  (countries.value ?? [])
    .filter((country) => country.is_active)
    .map((country) => ({ value: country.id, label: tr(country.name) }))
    .sort((a, b) => a.label.localeCompare(b.label, locale.value)),
)

const form = reactive({
  first_name: '',
  last_name: '',
  email: '',
  country_id: '',
  password: '',
})

const fieldErrors = ref<Record<string, string>>({})
const isSubmitting = ref(false)
const submitError = ref<Error | null>(null)

const strength = computed(() => evaluatePassword(form.password))

function validate(): boolean {
  const errors: Record<string, string> = {}

  if (form.first_name.trim().length === 0) errors.first_name = t('validation.required')
  if (form.last_name.trim().length === 0) errors.last_name = t('validation.required')

  if (form.email.trim().length === 0) errors.email = t('validation.required')
  else if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(form.email.trim())) errors.email = t('validation.email')

  if (form.country_id.length === 0) errors.country_id = t('validation.required')

  // Les trois exigences — longueur, majuscule, minuscule — et elles seules. Le
  // score de l'indicateur ne conditionne rien, et le message est celui, partagé,
  // de `_validation.json` : une règle, une phrase, partout la même.
  if (!strength.value.meetsRequirements) {
    errors.password = t('validation.passwordTooWeak', { min: MIN_PASSWORD_LENGTH })
  }

  fieldErrors.value = errors
  return Object.keys(errors).length === 0
}

async function submit(): Promise<void> {
  submitError.value = null
  if (!validate()) return

  isSubmitting.value = true
  try {
    await auth.register({
      first_name: form.first_name.trim(),
      last_name: form.last_name.trim(),
      email: form.email.trim(),
      country_id: form.country_id,
      password: form.password,
      preferred_locale: locale.value,
      // `Intl` donne le fuseau du système ; `people.timezone` est `NOT NULL` et
      // sert ensuite à afficher les créneaux dans l'heure de la personne.
      timezone: Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC',
    })
    await router.push(localePath('auth-verify-email'))
  } catch (error) {
    submitError.value = error instanceof Error ? error : new Error(String(error))
  } finally {
    isSubmitting.value = false
  }
}
</script>

<template>
  <AuthScreen>
    <!-- `idle` autant que `pending` : au rendu serveur, un chargement paresseux
         n'a pas encore démarré et le statut vaut `idle`. Ne traiter que
         `pending` afficherait un formulaire vide pendant ce temps-là. -->
    <UiLoadingState
      v-if="countriesStatus === 'pending' || countriesStatus === 'idle'"
      variant="form"
      :lines="5"
      :label="t('common.states.loading.label')"
    />

    <UiErrorState
      v-else-if="countriesError"
      :title="t('common.states.error.title')"
      :description="t('auth.register.states.referenceError')"
      :detail="countriesError.message"
      :retry-label="t('common.actions.retry')"
      @retry="reloadCountries()"
    />

    <!-- Référentiel vide : le formulaire ne peut pas demander un pays qu'il ne
         connaît pas, et `people.country_id` n'est pas une case à cocher. On le
         dit plutôt que d'afficher une liste déroulante à zéro entrée. -->
    <UiEmptyState
      v-else-if="countryOptions.length === 0"
      icon="globe"
      :title="t('auth.register.states.emptyTitle')"
      :description="t('auth.register.states.emptyDescription')"
      :action-label="t('common.actions.retry')"
      @action="reloadCountries()"
    />

    <AuthCard
      v-else
      :title="t('auth.register.title')"
      :description="t('auth.register.description')"
    >
      <UiAlert
        v-if="submitError"
        class="mb-5"
        intent="danger"
        :title="t('validation.server.generic')"
        :message="submitError.message"
        live
      />

      <form class="grid gap-4" novalidate @submit.prevent="submit">
        <div class="grid gap-4 sm:grid-cols-2">
          <UiInput
            v-model="form.first_name"
            autocomplete="given-name"
            :label="t('auth.register.fields.firstName')"
            :error="fieldErrors.first_name"
            :disabled="isSubmitting"
            required
          />
          <UiInput
            v-model="form.last_name"
            autocomplete="family-name"
            :label="t('auth.register.fields.lastName')"
            :error="fieldErrors.last_name"
            :disabled="isSubmitting"
            required
          />
        </div>

        <UiInput
          v-model="form.email"
          type="email"
          autocomplete="email"
          inputmode="email"
          icon="mail"
          :label="t('auth.register.fields.email')"
          :hint="t('auth.register.fields.emailHint')"
          :error="fieldErrors.email"
          :disabled="isSubmitting"
          required
        />

        <UiSelect
          v-model="form.country_id"
          :options="countryOptions"
          :label="t('auth.register.fields.country')"
          :hint="t('auth.register.fields.countryHint')"
          :placeholder="t('auth.register.fields.countryPlaceholder')"
          :error="fieldErrors.country_id"
          :disabled="isSubmitting"
          required
        />

        <div>
          <AuthPasswordField
            v-model="form.password"
            autocomplete="new-password"
            :label="t('auth.register.fields.password')"
            :hint="t('auth.register.fields.passwordHint')"
            :error="fieldErrors.password"
            :disabled="isSubmitting"
            required
          />
          <AuthPasswordStrength :password="form.password" />
        </div>

        <!-- Consentement aux conditions : donné par l'acte d'inscription, sans
             case à cocher. `identity.consents` est écrit côté API, qui seule
             connaît la version en vigueur de la politique. -->
        <p class="text-xs text-text-muted">
          <i18n-t keypath="auth.register.legal.text" tag="span" scope="global">
            <template #terms>
              <NuxtLink :to="localePath('/conditions-utilisation')" class="text-text-link">
                {{ t('auth.register.legal.terms') }}
              </NuxtLink>
            </template>
            <template #privacy>
              <NuxtLink :to="localePath('/confidentialite')" class="text-text-link">
                {{ t('auth.register.legal.privacy') }}
              </NuxtLink>
            </template>
          </i18n-t>
        </p>

        <UiButton
          type="submit"
          variant="primary"
          block
          size="lg"
          :loading="isSubmitting"
          :label="t('auth.register.submit')"
        />
      </form>

      <!-- L'étape suivante est annoncée AVANT d'être atteinte : personne
           n'aime découvrir qu'un formulaire en cachait un second. -->
      <p class="mt-5 flex items-start gap-2 rounded-md bg-surface-sunken px-3 py-2.5 text-xs text-text-muted">
        <UiIcon name="building" size="1rem" class="mt-0.5 shrink-0" />
        <span>{{ t('auth.register.nextStep') }}</span>
      </p>

      <template #footer>
        <p>
          {{ t('auth.register.haveAccount') }}
          <NuxtLink :to="localePath('auth-login')" class="ml-1 font-bold text-text-link">
            {{ t('auth.register.signIn') }}
          </NuxtLink>
        </p>
      </template>
    </AuthCard>
  </AuthScreen>
</template>
