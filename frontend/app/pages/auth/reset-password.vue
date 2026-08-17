<script setup lang="ts">
import type { PasswordResetResult, TokenCheckResult, TokenRejection } from '~/types/auth'
import { MIN_PASSWORD_LENGTH, evaluatePassword } from '~/utils/password-strength'

/**
 * Nouveau mot de passe — `/nouveau-mot-de-passe`, `/en/reset-password`.
 *
 * LE JETON EST CONTRÔLÉ AVANT D'AFFICHER LE FORMULAIRE. Laisser quelqu'un
 * composer un mot de passe, le taper, puis lui apprendre que le lien avait
 * expiré la veille est le genre de détail qui fait renoncer. Le contrôle est une
 * LECTURE — il ne consomme pas le jeton, seul l'envoi le fait.
 *
 * ET IL EST REFAIT À L'ENVOI. Entre l'affichage du formulaire et sa validation,
 * le jeton a pu périmer : un onglet ouvert la veille au soir et rempli le
 * lendemain matin est un cas ordinaire, pas une bizarrerie.
 *
 * MÊME RÈGLE DE MOT DE PASSE QU'À L'INSCRIPTION — longueur seule, indicateur
 * informatif. Deux écrans qui exigeraient deux choses différentes du même mot de
 * passe seraient impossibles à défendre.
 *
 * AUCUNE CONNEXION AUTOMATIQUE APRÈS COUP. Réinitialiser depuis un lien reçu par
 * courriel ne prouve que l'accès à la boîte aux lettres ; l'écran renvoie donc
 * vers la connexion, où le nouveau mot de passe sert immédiatement.
 */

definePageMeta({ layout: 'auth', middleware: 'guest' })
defineI18nRoute({ paths: { fr: '/nouveau-mot-de-passe', en: '/reset-password' } })

const { t } = useI18n()
const route = useRoute()
const localePath = useLocalePath()
const api = useApi()

useHead(() => ({ title: t('auth.reset-password.title') }))

const token = computed(() => {
  const raw = route.query.token
  const value = Array.isArray(raw) ? raw[0] : raw
  return typeof value === 'string' && value.length > 0 ? value : null
})

// --- Contrôle du jeton -------------------------------------------------------

const check = ref<TokenCheckResult | null>(null)
const checkError = ref<Error | null>(null)
const isChecking = ref(false)

async function checkToken(): Promise<void> {
  if (token.value === null) return
  isChecking.value = true
  checkError.value = null
  try {
    check.value = await api.auth.checkPasswordResetToken(token.value)
  } catch (error) {
    checkError.value = error instanceof Error ? error : new Error(String(error))
  } finally {
    isChecking.value = false
  }
}

onMounted(() => {
  if (token.value !== null) void checkToken()
})

// --- Nouveau mot de passe ----------------------------------------------------

const password = ref('')
const fieldError = ref<string | undefined>(undefined)
const isSubmitting = ref(false)
const submitError = ref<Error | null>(null)
const result = ref<PasswordResetResult | null>(null)

const strength = computed(() => evaluatePassword(password.value))

/**
 * Le refus affiché : celui du contrôle initial, ou celui de l'envoi quand le
 * jeton a péri entre les deux. Un seul endroit décide du message, sans quoi les
 * deux chemins finiraient par ne plus dire la même chose.
 */
const rejection = computed<TokenRejection | null>(() => {
  if (result.value?.status === 'rejected') return result.value.reason
  if (check.value?.status === 'rejected') return check.value.reason
  return null
})

async function submit(): Promise<void> {
  submitError.value = null
  if (token.value === null) return

  if (!strength.value.meetsRequirements) {
    fieldError.value = t('validation.passwordTooWeak', { min: MIN_PASSWORD_LENGTH })
    return
  }
  fieldError.value = undefined

  isSubmitting.value = true
  try {
    result.value = await api.auth.resetPassword(token.value, password.value)
    // Le mot de passe ne reste pas en mémoire une fois envoyé.
    password.value = ''
  } catch (error) {
    submitError.value = error instanceof Error ? error : new Error(String(error))
  } finally {
    isSubmitting.value = false
  }
}
</script>

<template>
  <AuthScreen>
    <!-- ─── Aucun jeton : l'état vide ───────────────────────────────────── -->
    <UiEmptyState
      v-if="token === null"
      icon="lock"
      :title="t('auth.reset-password.empty.title')"
      :description="t('auth.reset-password.empty.description')"
      :action-label="t('auth.reset-password.empty.action')"
      :action-to="localePath('auth-forgot-password')"
    />

    <UiLoadingState
      v-else-if="isChecking || (check === null && checkError === null)"
      variant="form"
      :lines="2"
      :label="t('auth.reset-password.checking')"
    />

    <UiErrorState
      v-else-if="checkError"
      :title="t('common.states.error.title')"
      :description="t('common.states.error.description')"
      :detail="checkError.message"
      :retry-label="t('common.actions.retry')"
      @retry="checkToken()"
    />

    <!-- ─── Mot de passe changé ─────────────────────────────────────────── -->
    <AuthCard
      v-else-if="result?.status === 'reset'"
      icon="check-circle"
      icon-intent="success"
      :title="t('auth.reset-password.done.title')"
      :description="t('auth.reset-password.done.description')"
    >
      <UiButton
        :to="localePath('auth-login')"
        variant="primary"
        block
        size="lg"
        icon-trailing="arrow-right"
        :label="t('auth.reset-password.done.signIn')"
      />
    </AuthCard>

    <!-- ─── Jeton refusé, au contrôle ou à l'envoi ──────────────────────── -->
    <AuthCard
      v-else-if="rejection !== null"
      icon="error"
      icon-intent="danger"
      :title="t('auth.reset-password.title')"
      :description="t(`auth.reset-password.rejected.${rejection}`)"
    >
      <UiButton
        v-if="rejection === 'already_used'"
        :to="localePath('auth-login')"
        variant="primary"
        block
        size="lg"
        :label="t('auth.reset-password.done.signIn')"
      />
      <UiButton
        v-else
        :to="localePath('auth-forgot-password')"
        variant="primary"
        block
        size="lg"
        :label="t('auth.reset-password.rejected.action')"
      />
    </AuthCard>

    <!-- ─── Le formulaire ───────────────────────────────────────────────── -->
    <AuthCard
      v-else-if="check?.status === 'valid'"
      :title="t('auth.reset-password.title')"
      :description="t('auth.reset-password.description')"
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
        <!-- L'adresse concernée est rappelée, en lecture seule : on sait pour
             quel compte on change le mot de passe. Le champ reste focalisable
             et copiable — c'est ce qui distingue `readonly` de `disabled`. Il
             porte aussi le `username` que les gestionnaires de mots de passe
             attendent pour enregistrer la bonne entrée. -->
        <UiInput
          :model-value="check.email"
          type="email"
          autocomplete="username"
          icon="mail"
          readonly
          :label="t('auth.reset-password.fields.account')"
        />

        <div>
          <AuthPasswordField
            v-model="password"
            autocomplete="new-password"
            :label="t('auth.reset-password.fields.password')"
            :hint="t('auth.reset-password.fields.passwordHint')"
            :error="fieldError"
            :disabled="isSubmitting"
            required
          />
          <AuthPasswordStrength :password="password" />
        </div>

        <UiButton
          type="submit"
          variant="primary"
          block
          size="lg"
          :loading="isSubmitting"
          :label="t('auth.reset-password.submit')"
        />
      </form>

      <template #footer>
        <NuxtLink :to="localePath('auth-login')" class="font-bold text-text-link">
          {{ t('auth.reset-password.backToLogin') }}
        </NuxtLink>
      </template>
    </AuthCard>
  </AuthScreen>
</template>
