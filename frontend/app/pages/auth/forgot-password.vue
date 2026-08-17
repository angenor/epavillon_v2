<script setup lang="ts">
/**
 * Mot de passe oublié — `/mot-de-passe-oublie`, `/en/forgot-password`.
 *
 * LA MÊME RÉPONSE DANS TOUS LES CAS. Adresse connue ou non, l'écran affiche
 * « Si un compte existe pour cette adresse, un lien vient d'être envoyé ». C'est
 * la formulation qui coûte le moins à la personne de bonne foi — elle apprend
 * quoi faire ensuite — et qui n'apprend rien à qui essaie des adresses au hasard.
 *
 * LA TOURNURE CONDITIONNELLE EST LE MESSAGE, PAS UN ORNEMENT. Écrire « un lien
 * a été envoyé » sans condition serait mentir la moitié du temps ; écrire
 * « cette adresse est inconnue » serait répondre à la question qu'un attaquant
 * pose. Le « si » dit exactement ce qui s'est passé.
 *
 * LE RENVOI OBÉIT AU MÊME DÉLAI QUE LA VÉRIFICATION D'ADRESSE — soixante
 * secondes. Deux écrans, une seule règle : c'est ce qui rend le comportement
 * prévisible, et c'est le même composable qui la tient.
 */

definePageMeta({ layout: 'auth', middleware: 'guest' })
defineI18nRoute({ paths: { fr: '/mot-de-passe-oublie', en: '/forgot-password' } })

const { t } = useI18n()
const localePath = useLocalePath()
const api = useApi()

useHead(() => ({ title: t('auth.forgot-password.title') }))

const email = ref('')
const fieldError = ref<string | undefined>(undefined)
const isSubmitting = ref(false)
const submitError = ref<Error | null>(null)
/** La demande a été prise en compte — l'écran passe en mode confirmation. */
const isSent = ref(false)

const countdown = useResendCountdown(60)

function validate(): boolean {
  if (email.value.trim().length === 0) {
    fieldError.value = t('validation.required')
    return false
  }
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email.value.trim())) {
    fieldError.value = t('validation.email')
    return false
  }
  fieldError.value = undefined
  return true
}

async function submit(): Promise<void> {
  submitError.value = null
  if (!validate()) return

  isSubmitting.value = true
  try {
    await api.auth.requestPasswordReset(email.value.trim())
    isSent.value = true
    countdown.start()
  } catch (error) {
    submitError.value = error instanceof Error ? error : new Error(String(error))
  } finally {
    isSubmitting.value = false
  }
}

const resendLabel = computed(() =>
  countdown.canResend.value
    ? t('auth.forgot-password.sent.resend')
    : t('auth.forgot-password.sent.resendIn', { seconds: countdown.remaining.value }),
)
</script>

<template>
  <AuthScreen>
    <AuthCard
      v-if="isSent"
      icon="mail"
      :title="t('auth.forgot-password.sent.title')"
    >
      <p class="text-sm text-text-secondary">
        <i18n-t keypath="auth.forgot-password.sent.description" tag="span" scope="global">
          <template #email>
            <strong class="break-words text-text">{{ email.trim() }}</strong>
          </template>
        </i18n-t>
      </p>

      <p class="mt-3 text-sm text-text-muted">{{ t('auth.forgot-password.sent.checkSpam') }}</p>

      <UiAlert
        v-if="submitError"
        class="mt-5"
        intent="danger"
        :title="t('validation.server.generic')"
        :message="submitError.message"
        live
      />

      <UiButton
        class="mt-5"
        variant="secondary"
        block
        icon="refresh"
        :disabled="!countdown.canResend.value"
        :loading="isSubmitting"
        :label="resendLabel"
        @click="submit()"
      />

      <template #footer>
        <NuxtLink :to="localePath('auth-login')" class="font-bold text-text-link">
          {{ t('auth.forgot-password.backToLogin') }}
        </NuxtLink>
      </template>
    </AuthCard>

    <AuthCard
      v-else
      :title="t('auth.forgot-password.title')"
      :description="t('auth.forgot-password.description')"
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
        <UiInput
          v-model="email"
          type="email"
          autocomplete="username"
          inputmode="email"
          icon="mail"
          :label="t('auth.forgot-password.fields.email')"
          :hint="t('auth.forgot-password.fields.emailHint')"
          :error="fieldError"
          :disabled="isSubmitting"
          required
        />

        <UiButton
          type="submit"
          variant="primary"
          block
          size="lg"
          :loading="isSubmitting"
          :label="t('auth.forgot-password.submit')"
        />
      </form>

      <template #footer>
        <NuxtLink :to="localePath('auth-login')" class="font-bold text-text-link">
          {{ t('auth.forgot-password.backToLogin') }}
        </NuxtLink>
      </template>
    </AuthCard>
  </AuthScreen>
</template>
