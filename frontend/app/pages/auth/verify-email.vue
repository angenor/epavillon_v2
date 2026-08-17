<script setup lang="ts">
import type { TokenRejection, VerifyEmailResult } from '~/types/auth'

/**
 * Vérification d'adresse — `/verification-adresse`, `/en/verify-email`.
 *
 * DEUX ÉCRANS EN UN, et c'est la même page parce que c'est la même adresse qui
 * les relie :
 *
 *  1. SANS JETON — « Un lien de vérification a été envoyé à … ». C'est l'écran
 *     qui suit l'inscription. Le renvoi devient possible au bout de 60 secondes.
 *  2. AVEC `?token=` — le lien du courriel a été suivi : on vérifie, et l'on
 *     annonce le résultat. Trois refus possibles, trois suites différentes.
 *
 * L'ADRESSE N'EST PAS DANS L'URL. Elle vient du store, posé par l'inscription.
 * `?email=…` l'aurait mise dans l'historique du navigateur, dans les journaux du
 * serveur et dans tout lien recopié — pour un affichage qui dure trente
 * secondes. Conséquence assumée : un rechargement de la page perd l'adresse, et
 * l'écran bascule sur son ÉTAT VIDE, qui explique quoi faire.
 *
 * `allow-authenticated` — suivre ce lien en étant connecté est ordinaire : on
 * s'est connecté sur un autre onglet, ou l'on vérifie une adresse secondaire.
 * Le refuser obligerait à se déconnecter pour valider sa propre adresse.
 *
 * TROIS REFUS, TROIS MESSAGES. « Lien expiré » invite à en redemander un ;
 * « déjà utilisé » annonce que le travail est fait et renvoie à la connexion ;
 * « lien invalide » ne suppose rien — l'adresse a pu être tronquée par un
 * client de messagerie, ce qui arrive plus souvent qu'une attaque.
 */

definePageMeta({ layout: 'auth', middleware: 'guest' })
defineI18nRoute({ paths: { fr: '/verification-adresse', en: '/verify-email' } })

const { t } = useI18n()
const route = useRoute()
const localePath = useLocalePath()
const api = useApi()
const auth = useAuthStore()

useHead(() => ({ title: t('auth.verify-email.title') }))

/** Le jeton du lien reçu par courriel. Non traduit : composé par l'API. */
const token = computed(() => {
  const raw = route.query.token
  const value = Array.isArray(raw) ? raw[0] : raw
  return typeof value === 'string' && value.length > 0 ? value : null
})

const pendingEmail = computed(() => auth.pendingVerificationEmail)

/**
 * Où l'on va une fois l'adresse vérifiée : se connecter, PUIS le rattachement à
 * une organisation.
 *
 * C'EST LA SUITE DU PARCOURS D'INSCRIPTION, PAS UN RACCOURCI. Le formulaire de
 * création de compte ne demande volontairement rien sur l'organisation — c'est
 * l'écran A2 qui s'en charge, et lui seul sait chercher avant de créer. Encore
 * faut-il y ARRIVER : sans ce paramètre, la personne atterrissait sur l'accueil,
 * sans organisation, sans rien qui le lui dise, et le premier dépôt de dossier
 * butait dessus des semaines plus tard.
 *
 * Le mécanisme est celui du middleware `auth` — `?redirect=` —, déjà écrit et
 * déjà validé côté page de connexion, qui refuse tout ce qui n'est pas un chemin
 * interne. On n'en invente pas un second.
 */
const signInThenJoin = computed(
  () => `${localePath('auth-login')}?redirect=${encodeURIComponent(localePath('organization-join'))}`,
)

// --- Mode 1 : le lien a été suivi -------------------------------------------

const isVerifying = ref(false)
const verification = ref<VerifyEmailResult | null>(null)
const verifyError = ref<Error | null>(null)

async function verify(): Promise<void> {
  if (token.value === null) return
  isVerifying.value = true
  verifyError.value = null
  try {
    verification.value = await api.auth.verifyEmail(token.value)
  } catch (error) {
    verifyError.value = error instanceof Error ? error : new Error(String(error))
  } finally {
    isVerifying.value = false
  }
}

// La vérification n'a lieu QUE dans le navigateur : elle consomme le jeton, et
// une action irréversible déclenchée par le rendu serveur se rejouerait à chaque
// prélecture de lien par un antivirus ou un client de messagerie.
onMounted(() => {
  if (token.value !== null) void verify()
})

const rejection = computed<TokenRejection | null>(() =>
  verification.value?.status === 'rejected' ? verification.value.reason : null,
)

// --- Mode 2 : le lien vient d'être envoyé -----------------------------------

const countdown = useResendCountdown(60)
const isResending = ref(false)
const resendError = ref<Error | null>(null)
/** Le lien vient d'être renvoyé — confirmation affichée sous le bouton. */
const hasResent = ref(false)

async function resend(): Promise<void> {
  if (!countdown.canResend.value || pendingEmail.value === null) return
  isResending.value = true
  resendError.value = null
  try {
    await api.auth.resendVerification(pendingEmail.value)
    hasResent.value = true
    countdown.start()
  } catch (error) {
    // Rebours remis à zéro : faire patienter une minute après un échec serait
    // punir quelqu'un pour une panne qui n'est pas la sienne.
    countdown.reset()
    resendError.value = error instanceof Error ? error : new Error(String(error))
  } finally {
    isResending.value = false
  }
}

// Le premier envoi vient d'avoir lieu — c'est l'inscription qui l'a déclenché :
// le rebours part avec l'écran, sans attendre un premier clic.
onMounted(() => {
  if (token.value === null && pendingEmail.value !== null) countdown.start()
})

const resendLabel = computed(() =>
  countdown.canResend.value
    ? t('auth.verify-email.sent.resend')
    : t('auth.verify-email.sent.resendIn', { seconds: countdown.remaining.value }),
)
</script>

<template>
  <AuthScreen allow-authenticated>
    <!-- ─── Le lien a été suivi ─────────────────────────────────────────── -->
    <template v-if="token !== null">
      <UiLoadingState
        v-if="isVerifying || verification === null"
        variant="text"
        :lines="3"
        :label="t('auth.verify-email.checking')"
      />

      <UiErrorState
        v-else-if="verifyError"
        :title="t('common.states.error.title')"
        :description="t('common.states.error.description')"
        :detail="verifyError.message"
        :retry-label="t('common.actions.retry')"
        @retry="verify()"
      />

      <AuthCard
        v-else-if="verification.status === 'verified'"
        icon="check-circle"
        icon-intent="success"
        :title="t('auth.verify-email.verified.title')"
        :description="t('auth.verify-email.verified.description', { email: verification.email })"
      >
        <UiButton
          :to="signInThenJoin"
          variant="primary"
          block
          size="lg"
          icon-trailing="arrow-right"
          :label="t('auth.verify-email.verified.signIn')"
        />
        <p class="mt-3 text-center text-xs text-text-muted">
          {{ t('auth.verify-email.verified.nextStep') }}
        </p>
      </AuthCard>

      <AuthCard
        v-else
        icon="error"
        icon-intent="danger"
        :title="t('auth.verify-email.title')"
        :description="t(`auth.verify-email.rejected.${rejection}`)"
      >
        <!-- Un jeton déjà consommé ne se redemande pas : l'adresse est vérifiée,
             il ne reste qu'à se connecter. -->
        <UiButton
          v-if="rejection === 'already_used'"
          :to="signInThenJoin"
          variant="primary"
          block
          size="lg"
          :label="t('auth.verify-email.verified.signIn')"
        />
        <UiButton
          v-else
          :to="localePath('auth-login')"
          variant="secondary"
          block
          :label="t('auth.verify-email.rejected.action')"
        />
      </AuthCard>
    </template>

    <!-- ─── Le lien vient d'être envoyé ─────────────────────────────────── -->
    <AuthCard
      v-else-if="pendingEmail !== null"
      icon="mail"
      :title="t('auth.verify-email.sent.title')"
    >
      <!-- L'adresse est mise en évidence : c'est elle qu'on relit pour repérer
           la faute de frappe qui explique le courriel jamais reçu. -->
      <p class="text-sm text-text-secondary">
        <i18n-t keypath="auth.verify-email.sent.description" tag="span" scope="global">
          <template #email>
            <strong class="break-words text-text">{{ pendingEmail }}</strong>
          </template>
        </i18n-t>
      </p>

      <p class="mt-3 text-sm text-text-muted">{{ t('auth.verify-email.sent.checkSpam') }}</p>

      <UiAlert
        v-if="resendError"
        class="mt-5"
        intent="danger"
        :title="t('validation.server.generic')"
        :message="resendError.message"
        live
      />
      <UiAlert
        v-else-if="hasResent"
        class="mt-5"
        intent="success"
        :message="t('auth.verify-email.sent.resent')"
        live
        compact
      />

      <UiButton
        class="mt-5"
        variant="secondary"
        block
        icon="refresh"
        :disabled="!countdown.canResend.value"
        :loading="isResending"
        :label="resendLabel"
        @click="resend()"
      />

      <p class="mt-2 text-xs text-text-subtle">{{ t('auth.verify-email.sent.resendHint') }}</p>

      <template #footer>
        <p>
          {{ t('auth.verify-email.sent.wrongAddress') }}
          <NuxtLink :to="localePath('auth-register')" class="ml-1 font-bold text-text-link">
            {{ t('auth.verify-email.sent.startOver') }}
          </NuxtLink>
        </p>
      </template>
    </AuthCard>

    <!-- ─── Ni jeton, ni adresse : l'état vide ──────────────────────────── -->
    <UiEmptyState
      v-else
      icon="mail"
      :title="t('auth.verify-email.empty.title')"
      :description="t('auth.verify-email.empty.description')"
      :action-label="t('auth.verify-email.empty.action')"
      :action-to="localePath('auth-login')"
    />
  </AuthScreen>
</template>
