<script setup lang="ts">
import type { TokenRejection } from '~/types/auth'
import type { AcceptInvitationResult } from '~/types/organization-workspace'

/**
 * ACCEPTATION D'UNE INVITATION — `/invitation?token=…`, `/en/invitation?token=…`.
 *
 * PAS DE `defineI18nRoute` ICI, ET C'EST VOLONTAIRE. Les deux chemins sont
 * composés par l'API — `mail.rs` écrit `/invitation` en français et
 * `/en/invitation` en anglais —, et c'est exactement ce que produit la stratégie
 * `prefix_except_default` sur un chemin non traduit. Traduire ce chemin comme le
 * font les écrans d'authentification casserait les liens déjà partis par
 * courriel : ceux-là ne se redéploient pas.
 *
 * AUCUNE SESSION N'EST EXIGÉE, et c'est la règle qui gouverne tout l'écran. Le
 * jeton EST la preuve d'adresse ; la personne qu'une invitation vise n'a le plus
 * souvent pas encore de compte, et lui demander de se connecter d'abord
 * reviendrait à exiger d'elle ce que l'invitation est censée déclencher. Le
 * middleware `guest` n'est donc pas là pour refuser quoi que ce soit — il ne
 * redirige personne, il garantit seulement que la session soit RÉSOLUE avant le
 * rendu, sans quoi l'écran choisirait sa suite avant de savoir qui regarde.
 *
 * TROIS REFUS, TROIS SUITES DIFFÉRENTES — c'est la leçon des écrans à jeton de
 * B1, et elle vaut ici aussi : un lien périmé se redemande à l'organisation qui
 * l'a émis, un lien déjà utilisé annonce que le travail est fait et propose la
 * connexion, un lien invalide ne suppose rien et ne propose que l'accueil.
 *
 * ET UN QUATRIÈME CAS, QUI N'EST PAS UN REFUS DE JETON : quelqu'un de connecté
 * suit le lien reçu par un collègue. L'API répond `ORG_INVITATION_NOT_YOURS`, et
 * l'écran le rend comme un accès refusé nommé — se déconnecter est la seule
 * suite utile, et l'écran l'offre plutôt que de la faire deviner.
 *
 * L'ACCEPTATION N'A LIEU QUE DANS LE NAVIGATEUR. Elle consomme le jeton : une
 * action irréversible déclenchée au rendu serveur se rejouerait à chaque
 * prélecture de lien par un antivirus ou un client de messagerie, et l'invitée
 * trouverait son lien déjà utilisé sans y avoir touché.
 */

definePageMeta({ layout: 'auth', middleware: 'guest' })

const { t } = useI18n()
const route = useRoute()
const localePath = useLocalePath()
const api = useApi()
const auth = useAuthStore()

useHead(() => ({ title: t('invitation.title') }))

/** Le jeton du lien reçu par courriel. Non traduit : composé par l'API. */
const token = computed(() => {
  const raw = route.query.token
  const value = Array.isArray(raw) ? raw[0] : raw
  return typeof value === 'string' && value.length > 0 ? value : null
})

const isAccepting = ref(false)
const outcome = ref<AcceptInvitationResult | null>(null)
const acceptError = ref<Error | null>(null)

/**
 * Le refus d'autorisation se distingue de la panne : il ne se réessaie pas, et
 * sa suite n'est pas la même. Le code de l'API fait foi ; le statut sert de
 * repli tant que le corps d'erreur n'est pas normalisé côté transport.
 */
const isNotYours = computed(() => {
  const error = acceptError.value as { statusCode?: number; data?: { code?: string } } | null
  return error?.data?.code === 'ORG_INVITATION_NOT_YOURS' || error?.statusCode === 403
})

/**
 * Se déconnecter, puis reprendre l'acceptation.
 *
 * L'API consomme le jeton AVANT de comparer la session à la personne invitée,
 * mais son refus annule la transaction : le lien reste donc utilisable, et la
 * suite naturelle est ici même — inutile de renvoyer quelqu'un chercher son
 * courriel pour recliquer sur le lien qu'il vient d'ouvrir.
 */
async function signOutThenAccept(): Promise<void> {
  await auth.signOut()
  await accept()
}

async function accept(): Promise<void> {
  if (token.value === null) return
  isAccepting.value = true
  acceptError.value = null
  try {
    outcome.value = await api.invitation.accept(token.value)
  } catch (error) {
    acceptError.value = error instanceof Error ? error : new Error(String(error))
  } finally {
    isAccepting.value = false
  }
}

onMounted(() => {
  if (token.value !== null) void accept()
})

const rejection = computed<TokenRejection | null>(() =>
  outcome.value?.status === 'rejected' ? outcome.value.reason : null,
)

const organizationName = computed(() =>
  outcome.value?.status === 'accepted' ? outcome.value.organization.legal_name : '',
)

/**
 * OÙ L'ON VA UNE FOIS L'ADHÉSION ACTIVE, et la réponse dépend de qui regarde.
 *
 * Connectée, la personne va droit à l'espace de son organisation. Sinon, l'écran
 * propose les DEUX suites plutôt que d'en deviner une : l'API ne dit pas si un
 * compte existe derrière l'adresse, et se tromper coûte cher dans les deux sens
 * — envoyer créer un compte à qui en a déjà un, ou envoyer se connecter
 * quelqu'un qui n'a pas de mot de passe. La création passe en premier : c'est le
 * cas de la personne qu'une invitation vise le plus souvent.
 *
 * Le retour après connexion emprunte le `?redirect=` du middleware `auth`, déjà
 * écrit et déjà éprouvé, qui refuse tout ce qui n'est pas un chemin interne.
 */
const workspacePath = computed(() => localePath('organization-workspace'))
const signInThenWorkspace = computed(
  () => `${localePath('auth-login')}?redirect=${encodeURIComponent(workspacePath.value)}`,
)
</script>

<template>
  <AuthScreen allow-authenticated>
    <!-- ─── Le lien a été suivi ─────────────────────────────────────────── -->
    <template v-if="token !== null">
      <UiLoadingState
        v-if="isAccepting || (outcome === null && acceptError === null)"
        variant="text"
        :lines="3"
        :label="t('invitation.checking')"
      />

      <!-- ACCÈS REFUSÉ — connecté sous un autre compte que l'invitée. Ce n'est
           pas une panne : aucune reprise n'est proposée, seulement la sortie. -->
      <AuthCard
        v-else-if="acceptError && isNotYours"
        icon="error"
        icon-intent="danger"
        :title="t('invitation.notYours.title')"
        :description="t('invitation.notYours.description')"
      >
        <UiButton
          variant="secondary"
          block
          size="lg"
          :label="t('invitation.notYours.action')"
          :loading="isAccepting"
          @click="signOutThenAccept()"
        />
      </AuthCard>

      <UiErrorState
        v-else-if="acceptError"
        :title="t('common.states.error.title')"
        :description="t('common.states.error.description')"
        :detail="acceptError.message"
        :retry-label="t('common.actions.retry')"
        @retry="accept()"
      />

      <!-- ─── L'adhésion est active ─────────────────────────────────────── -->
      <AuthCard
        v-else-if="outcome && outcome.status === 'accepted'"
        icon="check-circle"
        icon-intent="success"
        :title="t('invitation.accepted.title', { organization: organizationName })"
        :description="t('invitation.accepted.description')"
      >
        <UiButton
          v-if="auth.isAuthenticated"
          :to="workspacePath"
          variant="primary"
          block
          size="lg"
          icon-trailing="arrow-right"
          :label="t('invitation.accepted.workspace')"
        />

        <template v-else>
          <UiButton
            :to="localePath('auth-register')"
            variant="primary"
            block
            size="lg"
            icon-trailing="arrow-right"
            :label="t('invitation.accepted.createAccount')"
          />
          <p class="mt-3 text-center text-xs text-text-muted">
            {{ t('invitation.accepted.createAccountHint') }}
          </p>
        </template>

        <!-- Le pied n'est offert qu'à qui n'a pas de session : proposer « se
             connecter » à quelqu'un de déjà connecté n'aurait aucun sens. -->
        <template v-if="!auth.isAuthenticated" #footer>
          <p>
            {{ t('invitation.accepted.haveAccount') }}
            <NuxtLink :to="signInThenWorkspace" class="ml-1 font-bold text-text-link">
              {{ t('invitation.accepted.signIn') }}
            </NuxtLink>
          </p>
        </template>
      </AuthCard>

      <!-- ─── Les trois refus ───────────────────────────────────────────── -->
      <AuthCard
        v-else-if="rejection"
        :icon="rejection === 'already_used' ? 'check-circle' : 'error'"
        :icon-intent="rejection === 'already_used' ? 'success' : 'danger'"
        :title="t(`invitation.rejected.${rejection}.title`)"
        :description="t(`invitation.rejected.${rejection}.description`)"
      >
        <!-- Une invitation déjà acceptée ne se redemande pas : l'adhésion
             existe, il ne reste qu'à se connecter pour la retrouver. -->
        <UiButton
          v-if="rejection === 'already_used'"
          :to="signInThenWorkspace"
          variant="primary"
          block
          size="lg"
          :label="t('invitation.rejected.already_used.action')"
        />
        <UiButton
          v-else
          :to="localePath('/')"
          variant="secondary"
          block
          :label="t(`invitation.rejected.${rejection}.action`)"
        />
      </AuthCard>
    </template>

    <!-- ─── Ni jeton, ni rien à confirmer : l'état vide ──────────────────── -->
    <UiEmptyState
      v-else
      icon="mail"
      :title="t('invitation.empty.title')"
      :description="t('invitation.empty.description')"
      :action-label="t('invitation.empty.action')"
      :action-to="localePath('/')"
    />
  </AuthScreen>
</template>
