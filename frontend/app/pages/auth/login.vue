<script setup lang="ts">
import type { LoginResult } from '~/types/auth'

/**
 * Connexion — `/connexion`, `/en/login`.
 *
 * DES ERREURS PEU BAVARDES, PAR CONSTRUCTION. Cet écran ne compose aucun
 * message : il rend celui que dicte l'issue renvoyée par l'API. Et cette issue
 * ne peut pas trahir l'existence d'une adresse — voir l'ordre des contrôles dans
 * `mocks/auth.ts` : tant que le mot de passe n'est pas juste, la seule réponse
 * possible est `invalid_credentials`. Un compte verrouillé, une personne
 * suspendue, une adresse non vérifiée ne se disent qu'à quelqu'un qui vient de
 * prouver son identité.
 *
 * UN SEUL MESSAGE POUR DEUX CAS. « Adresse ou mot de passe incorrect » couvre
 * l'adresse inconnue ET le mot de passe faux. C'est volontairement moins utile à
 * la personne qui se trompe, et c'est ce qui empêche d'énumérer les comptes de
 * la plateforme — sur une plateforme qui héberge des délégations nationales, la
 * seule existence d'un compte est déjà une information.
 *
 * `?redirect=` — le middleware `auth` y range la page demandée avant la
 * redirection ; on y revient une fois connecté. Le paramètre n'est jamais
 * affiché et n'est pas traduit : il est composé par le code.
 */

definePageMeta({ layout: 'auth', middleware: 'guest' })

/**
 * URL en toutes lettres, dans chaque langue. Le fichier reste `auth/login.vue` —
 * les traductions de l'écran suivent le CHEMIN DU FICHIER
 * (`pages/auth.login.json`), l'adresse publique suit la LANGUE. Confondre les
 * deux obligerait à choisir entre des fichiers de traduction français et des URL
 * anglaises.
 */
defineI18nRoute({ paths: { fr: '/connexion', en: '/login' } })

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()
const auth = useAuthStore()

useHead(() => ({ title: t('auth.login.title') }))

const email = ref('')
const password = ref('')
const rememberMe = ref(false)

const isSubmitting = ref(false)
/** Issue de la dernière tentative ; `null` tant qu'aucune n'a eu lieu. */
const outcome = ref<LoginResult | null>(null)
/** Erreur de transport — pas une réponse de l'API, une panne. */
const submitError = ref<Error | null>(null)
/** Validation locale, avant tout appel : ne pas déranger le serveur pour rien. */
const fieldErrors = ref<{ email?: string; password?: string }>({})

const isMfaStep = computed(() => outcome.value?.status === 'mfa_required')

/**
 * Où revenir après la connexion. La valeur vient de l'URL, donc de
 * l'extérieur : seuls les chemins internes sont acceptés. Une valeur commençant
 * par `//` ou portant un schéma ouvrirait une redirection vers un autre site
 * depuis un lien qui a l'air d'être le nôtre.
 */
const redirectTo = computed(() => {
  const raw = route.query.redirect
  const value = Array.isArray(raw) ? raw[0] : raw
  if (typeof value !== 'string' || !value.startsWith('/') || value.startsWith('//')) {
    return localePath('/')
  }
  return value
})

function validate(): boolean {
  const errors: { email?: string; password?: string } = {}
  if (email.value.trim().length === 0) errors.email = t('validation.required')
  else if (!email.value.includes('@')) errors.email = t('validation.email')
  if (password.value.length === 0) errors.password = t('validation.required')
  fieldErrors.value = errors
  return Object.keys(errors).length === 0
}

async function submit(): Promise<void> {
  outcome.value = null
  submitError.value = null
  if (!validate()) return

  isSubmitting.value = true
  try {
    const result = await auth.signIn({
      email: email.value.trim(),
      password: password.value,
      remember_me: rememberMe.value,
    })
    outcome.value = result

    if (result.status === 'authenticated') {
      await router.push(redirectTo.value)
      return
    }
    if (result.status === 'email_unverified') {
      // L'adresse est retenue pour l'écran suivant, qui proposera le renvoi.
      auth.rememberVerificationTarget(result.email)
    }
    // Le mot de passe ne reste jamais à l'écran après un échec : le champ se
    // vide, celui de l'adresse garde sa valeur.
    password.value = ''
  } catch (error) {
    submitError.value = error instanceof Error ? error : new Error(String(error))
  } finally {
    isSubmitting.value = false
  }
}

/** Message d'échec, choisi par l'issue. `null` quand il n'y a rien à dire. */
const failure = computed<{ intent: 'danger' | 'warning'; title: string; message: string } | null>(
  () => {
    const result = outcome.value
    if (result === null) return null

    switch (result.status) {
      case 'invalid_credentials':
        return {
          intent: 'danger',
          title: t('auth.login.errors.invalid.title'),
          message: t('auth.login.errors.invalid.message'),
        }
      case 'locked':
        return {
          intent: 'warning',
          title: t('auth.login.errors.locked.title'),
          message: t('auth.login.errors.locked.message'),
        }
      case 'suspended':
        return {
          intent: 'warning',
          title: t('auth.login.errors.suspended.title'),
          message: t('auth.login.errors.suspended.message'),
        }
      case 'email_unverified':
        return {
          intent: 'warning',
          title: t('auth.login.errors.unverified.title'),
          message: t('auth.login.errors.unverified.message'),
        }
      default:
        return null
    }
  },
)
</script>

<template>
  <AuthScreen :continue-to="'/'">
    <AuthCard
      :title="t('auth.login.title')"
      :description="isMfaStep ? undefined : t('auth.login.description')"
    >
      <!-- Second facteur : la connexion s'est arrêtée après le mot de passe.
           L'emplacement est prévu, l'étape n'est pas implémentée. -->
      <AuthSecondFactor v-if="isMfaStep" @cancel="outcome = null" />

      <template v-else>
        <UiAlert
          v-if="submitError"
          class="mb-5"
          intent="danger"
          :title="t('validation.server.generic')"
          :message="submitError.message"
          live
        />

        <!-- Une seule issue appelle une suite immédiate : l'adresse non
             vérifiée, dont le lien se redemande depuis l'écran voisin. Le
             bouton vit sous l'alerte et non dans son créneau d'actions, pour
             que celui-ci reste absent quand il n'y a rien à proposer. -->
        <div v-else-if="failure" class="mb-5">
          <UiAlert
            :intent="failure.intent"
            :title="failure.title"
            :message="failure.message"
            live
          />
          <UiButton
            v-if="outcome?.status === 'email_unverified'"
            class="mt-3"
            variant="secondary"
            size="sm"
            icon="mail"
            :to="localePath('auth-verify-email')"
            :label="t('auth.login.errors.unverified.action')"
          />
        </div>

        <form class="grid gap-4" novalidate @submit.prevent="submit">
          <UiInput
            v-model="email"
            type="email"
            autocomplete="username"
            inputmode="email"
            icon="mail"
            :label="t('auth.login.fields.email')"
            :placeholder="t('auth.login.fields.emailPlaceholder')"
            :error="fieldErrors.email"
            :disabled="isSubmitting"
            required
          />

          <div>
            <AuthPasswordField
              v-model="password"
              autocomplete="current-password"
              :label="t('auth.login.fields.password')"
              :error="fieldErrors.password"
              :disabled="isSubmitting"
              required
            />
            <p class="mt-2 text-sm">
              <NuxtLink :to="localePath('auth-forgot-password')" class="text-text-link">
                {{ t('auth.login.forgotPassword') }}
              </NuxtLink>
            </p>
          </div>

          <UiCheckbox
            v-model="rememberMe"
            :label="t('auth.login.rememberMe')"
            :hint="t('auth.login.rememberMeHint')"
            :disabled="isSubmitting"
          />

          <UiButton
            type="submit"
            variant="primary"
            block
            size="lg"
            :loading="isSubmitting"
            :label="t('auth.login.submit')"
          />
        </form>
      </template>

      <template #footer>
        <p>
          {{ t('auth.login.noAccount') }}
          <NuxtLink :to="localePath('auth-register')" class="ml-1 font-bold text-text-link">
            {{ t('auth.login.createAccount') }}
          </NuxtLink>
        </p>
      </template>
    </AuthCard>
  </AuthScreen>
</template>
