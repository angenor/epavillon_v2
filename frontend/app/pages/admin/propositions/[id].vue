<script setup lang="ts">
import type { PostCommentPayload, ReviewDeskScreen, SaveReviewPayload } from '~/types/admin-review'
import type { EffectivePermission } from '~/types/identity'
import type { ProposalStatus, ProposalTransitionRule } from '~/types/programme/proposal'
import type { TabItem } from '~/types/ui'
import type { DecisionOption } from '~/utils/review-scoring'

/**
 * FICHE D'ÉVALUATION D'UNE PROPOSITION — `/admin/propositions/:id`.
 *
 * C'EST ICI QU'UN MEMBRE DU COMITÉ DÉCIDE, ET TOUT S'Y FAIT SANS QUITTER LA
 * PAGE : lire le dossier, noter, écrire au comité ou au déposant, se déporter,
 * trancher. Chaque aller-retour vers un autre écran est une note perdue ou une
 * lecture recommencée — l'écran le plus dense de la plateforme est aussi celui
 * qu'on ne doit jamais avoir à quitter.
 *
 * DEUX COLONNES, ET LEUR RAISON D'ÊTRE. À GAUCHE le dossier, en largeur
 * dominante : c'est un texte qu'on lit, avec ses organisations, ses
 * intervenants, ses pièces et son historique. À DROITE le panneau d'évaluation,
 * COLLANT AU DÉFILEMENT — on note EN LISANT, et un panneau qui s'échappe vers le
 * haut oblige à noter de mémoire. Sous 1024 px, la colonne de droite passe
 * dessous et la colle est retirée : coller un panneau de 600 px sur un téléphone
 * masque le texte qu'il sert à juger.
 *
 * L'ÉVALUATION EN AVEUGLE EST LA RÈGLE DE L'APPEL, pas un réglage d'écran.
 * `calls_for_proposals.blind_review` commande, et le voile est appliqué À LA
 * SOURCE : tant que ma revue n'est pas déposée, les revues de mes pairs ne sont
 * pas dans la réponse. L'écran en dit le nombre et la raison — un panneau vide
 * sans explication passe pour une panne.
 *
 * LA NOTATION ET LA DÉCISION SONT DEUX GESTES DIFFÉRENTS, faits par deux
 * personnes différentes, et l'écran les sépare : la grille à droite, les actions
 * de décision dans un bloc distinct de l'en-tête. Chacune se teste par
 * PERMISSION — `programme.review.write` pour noter, `programme.proposal.decide`
 * pour trancher —, toujours sur la portée de l'édition regardée.
 *
 * RÈGLE MÉTIER N° 8 — LE PÉRIMÈTRE D'ADMINISTRATION. Un dossier d'une édition
 * hors périmètre n'est pas affiché, même par URL forgée : `useApi()` refuse
 * l'édition, et la page rend « accès refusé » plutôt qu'une fiche vide.
 *
 * QUATRE ÉTATS : chargement, erreur avec reprise, dossier introuvable, accès
 * refusé.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [
    { labelKey: 'nav.admin.proposals', to: '/admin/propositions' },
    { labelKey: 'admin.proposal.review.title' },
  ],
})

const { t } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()
const route = useRoute()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const localePath = useLocalePath()

const proposalId = computed(() => String(route.params.id ?? ''))

await adminScope.ensureLoaded()

// ---------------------------------------------------------------------------
// Données
// ---------------------------------------------------------------------------

const {
  data: screen,
  status,
  error,
  refresh,
} = await useAsyncData<ReviewDeskScreen | null>(
  () => `review-desk-${proposalId.value}`,
  () => api.review.desk(proposalId.value, auth.person?.id ?? null),
  { watch: [proposalId], lazy: true },
)

/** La machine à états, lue et non réécrite : elle ne dépend d'aucune édition. */
const { data: transitionRules } = await useAsyncData<ProposalTransitionRule[]>(
  'proposal-transition-rules',
  () => api.proposals.transitionRules(),
  { default: () => [], lazy: true },
)

const { data: granted } = await useAsyncData<EffectivePermission[]>(
  'admin-review-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

useHead(() => ({
  title: screen.value
    ? `${screen.value.proposal.reference_code} — ${tr(screen.value.proposal.title)}`
    : t('admin.proposal.review.title'),
}))

const timezone = computed(() => screen.value?.edition.timezone ?? 'UTC')
const zoneLabel = computed(() => screen.value?.edition.city?.trim() || timezone.value)

/**
 * Le pays du dossier est une donnée multilingue de la base ; il est résolu ici
 * plutôt que dans le composant, qui ne doit connaître qu'un texte prêt.
 */
const { data: countries } = await useAsyncData('reference-countries', () => api.reference.countries(), {
  default: () => [],
  lazy: true,
})

const countryName = computed(() => {
  const id = screen.value?.proposal.country_id
  if (!id) return null
  const country = countries.value.find((entry) => entry.id === id)
  return country ? tr(country.name) : null
})

function personName(id: string | null | undefined): string | null {
  if (!id) return null
  return screen.value?.participants.find((person) => person.id === id)?.display_name ?? null
}

const leadOrganizationName = computed(
  () =>
    screen.value?.organizations.find((entry) => entry.link.role === 'lead')?.organization?.legal_name ??
    '',
)

/**
 * LES DÉCISIONS OFFERTES viennent de `proposal_transitions_allowed`, filtrées
 * par la permission de la personne SUR CETTE ÉDITION. La liste est vide quand la
 * base n'ouvre aucun chemin depuis l'état courant — et l'en-tête le dit.
 */
const decisions = computed<DecisionOption[]>(() =>
  screen.value
    ? decisionOptions(
        screen.value.proposal.status,
        transitionRules.value,
        granted.value,
        screen.value.proposal.event_id,
      )
    : [],
)

// ---------------------------------------------------------------------------
// Onglets du dossier
//
// L'onglet actif vit dans l'URL : un lien envoyé vers l'historique d'un dossier
// doit ouvrir l'historique, pas la présentation.
// ---------------------------------------------------------------------------

const router = useRouter()

const activeTab = computed<string>(() => {
  const requested = route.query.vue
  const asked = Array.isArray(requested) ? requested[0] : requested
  return String(asked) === 'history' ? 'history' : 'dossier'
})

function selectTab(value: string): void {
  router.replace({ query: { ...route.query, vue: value === 'dossier' ? undefined : value } })
}

const tabs = computed<TabItem[]>(() => [
  { value: 'dossier', label: t('admin.proposal.review.tabs.dossier') },
  {
    value: 'history',
    label: t('admin.proposal.review.tabs.history'),
    count: screen.value?.history.length,
  },
])

// ---------------------------------------------------------------------------
// Écritures
// ---------------------------------------------------------------------------

const busy = ref(false)
const reviewError = ref<string | null>(null)
const commentError = ref<string | null>(null)
const decisionError = ref<string | null>(null)
const notice = ref<string | null>(null)
const savedAt = ref<string | null>(null)

/**
 * Un refus de l'API s'affiche TEL QUEL : elle seule sait pourquoi elle refuse, et
 * son catalogue est déjà français. Le site ne reprend la parole que lorsqu'elle
 * s'est tue — c'est ce que fait `apiErrorMessage`.
 */
function writeError(thrown: unknown): string {
  return thrown instanceof ForbiddenError ? thrown.message : apiErrorMessage(thrown, (key) => t(key))
}

const recusalOpen = ref(false)
const decisionOpen = ref(false)
const pendingDecision = ref<DecisionOption | null>(null)

async function saveReview(payload: Omit<SaveReviewPayload, 'proposal_id'>): Promise<void> {
  const person = auth.person
  if (!person || !screen.value) return
  busy.value = true
  reviewError.value = null
  try {
    await api.review.save(person.id, { ...payload, proposal_id: screen.value.proposal.id })
    // Le dépôt d'une revue change les agrégats du dossier ET peut lever le voile
    // de l'évaluation en aveugle : l'écran entier se recharge, il ne se rafistole
    // pas champ par champ.
    savedAt.value = new Date().toLocaleTimeString()
    await refresh()
  } catch (thrown) {
    reviewError.value = writeError(thrown)
  } finally {
    busy.value = false
  }
}

async function recuse(reason: string): Promise<void> {
  const person = auth.person
  if (!person || !screen.value) return
  busy.value = true
  reviewError.value = null
  try {
    await api.review.recuse(person.id, { proposal_id: screen.value.proposal.id, reason })
    recusalOpen.value = false
    notice.value = t('admin.proposal.review.recusal.success')
    await refresh()
  } catch (thrown) {
    reviewError.value = writeError(thrown)
  } finally {
    busy.value = false
  }
}

async function postComment(payload: Omit<PostCommentPayload, 'proposal_id'>): Promise<void> {
  const person = auth.person
  if (!person || !screen.value) return
  busy.value = true
  commentError.value = null
  try {
    await api.review.comment(person.id, { ...payload, proposal_id: screen.value.proposal.id })
    notice.value = t('admin.proposal.review.comments.sent')
    await refresh()
  } catch (thrown) {
    commentError.value = writeError(thrown)
  } finally {
    busy.value = false
  }
}

function openDecision(option: DecisionOption): void {
  pendingDecision.value = option
  decisionError.value = null
  decisionOpen.value = true
}

async function decide(payload: { toStatus: ProposalStatus; reason: string | null }): Promise<void> {
  if (!screen.value) return
  busy.value = true
  decisionError.value = null
  try {
    const result = await api.review.decide(auth.person?.id ?? null, {
      proposal_id: screen.value.proposal.id,
      to_status: payload.toStatus,
      reason: payload.reason,
    })

    // Les refus de la machine à états sont des RÉPONSES, pas des erreurs de
    // réseau : ils se disent dans la boîte de dialogue, qui reste ouverte.
    if (result.status === 'applied') {
      decisionOpen.value = false
      notice.value = t('admin.proposal.review.decision.success', {
        status: t(`admin.proposal.review.status.${payload.toStatus}`),
      })
      await refresh()
    } else {
      decisionError.value = t(`admin.proposal.review.decision.${
        result.status === 'reason_required' ? 'reasonRequired' : 'notAllowed'
      }`)
    }
  } catch (thrown) {
    decisionError.value = writeError(thrown)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="mx-auto w-full max-w-[100rem]">
    <!-- ACCÈS REFUSÉ — aucun droit d'administration, ou édition hors périmètre.
         Distinct d'un dossier introuvable : l'un dit « vous n'avez pas ce
         droit », l'autre « ce dossier n'existe pas ». -->
    <UiForbiddenState
      v-if="(!adminScope.isLoading && !adminScope.canAdminister) || isForbiddenError(error)"
      :required-scope="t('admin.proposal.review.forbidden.scope')"
      action-to="/admin/propositions"
      :action-label="t('admin.proposal.review.backToList')"
    />

    <template v-else>
      <UiErrorState
        v-if="error"
        :retry-label="t('common.actions.retry')"
        @retry="refresh()"
      />

      <div v-else-if="status === 'pending' && !screen" class="flex flex-col gap-4">
        <UiSkeletonLoader height="9rem" />
        <div class="grid gap-6 lg:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
          <UiSkeletonLoader height="28rem" />
          <UiSkeletonLoader height="28rem" />
        </div>
      </div>

      <UiEmptyState
        v-else-if="!screen"
        icon="inbox"
        :title="t('admin.proposal.review.notFound.title')"
        :description="t('admin.proposal.review.notFound.description')"
        :action-label="t('admin.proposal.review.notFound.action')"
        :action-to="localePath('/admin/propositions')"
      />

      <template v-else>
        <NuxtLink
          :to="localePath('/admin/propositions')"
          class="inline-flex items-center gap-1.5 text-sm no-underline"
        >
          <UiIcon name="arrow-left" size="1rem" :stroke-width="1.8" />
          {{ t('admin.proposal.review.backToList') }}
        </NuxtLink>

        <AdminReviewHeader
          class="mt-3"
          :proposal="screen.proposal"
          :rank="screen.rank"
          :required-reviews="screen.required_reviews"
          :committee="screen.committee"
          :read-count="screen.read_count"
          :first-visit="screen.first_visit"
          :timezone="timezone"
          :decisions="decisions"
          :can-decide="screen.permissions.can_decide"
          :busy="busy"
          @decide="openDecision"
        />

        <UiAlert
          v-if="notice"
          class="mt-4"
          intent="success"
          live
          dismissible
          :message="notice"
          @dismiss="notice = null"
        />

        <!-- DEUX COLONNES. La lecture domine (3/2) ; le panneau colle au
             défilement à partir de 1024 px, et repasse dessous en dessous. -->
        <div class="mt-6 grid items-start gap-6 lg:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
          <div class="flex min-w-0 flex-col gap-8">
            <UiTabs
              :items="tabs"
              :model-value="activeTab"
              :label="t('admin.proposal.review.tabs.dossier')"
              @update:model-value="selectTab"
            />

            <template v-if="activeTab === 'dossier'">
              <AdminReviewDossier
                :proposal="screen.proposal"
                :themes="screen.themes"
                :timezone="timezone"
                :zone-label="zoneLabel"
                :country-name="countryName"
                :submitter-name="personName(screen.proposal.submitted_by)"
                :contact-name="personName(screen.proposal.contact_person_id)"
              />

              <AdminReviewOrganizations :entries="screen.organizations" :timezone="timezone" />
              <AdminReviewSpeakers :entries="screen.speakers" :timezone="timezone" />
              <AdminReviewDocuments :entries="screen.documents" :timezone="timezone" />
            </template>

            <!-- L'HISTORIQUE CHAMP PAR CHAMP est le même composant que l'espace
                 organisation : `programme.proposal_history()` rend la même chose
                 aux deux écrans, et deux rendus divergeraient sur les cas qui
                 comptent — un titre multilingue, une date, un statut. -->
            <WorkspaceHistoryList v-else :entries="screen.history" :timezone="timezone" />
          </div>

          <!-- LA COLONNE D'ÉVALUATION. `lg:sticky` la garde sous les yeux
               pendant la lecture ; `top-20` la pose sous l'en-tête collant du
               back-office. -->
          <div class="flex flex-col gap-6 lg:sticky lg:top-20">
            <AdminReviewScorePanel
              :criteria="screen.criteria"
              :max-weighted-score="screen.max_weighted_score"
              :my-review="screen.my_review"
              :permissions="screen.permissions"
              :timezone="timezone"
              :busy="busy"
              :error="reviewError"
              :saved-at="savedAt"
              @save="saveReview"
              @recuse="recusalOpen = true"
            />

            <AdminReviewPeerReviews
              :peer-reviews="screen.peer_reviews"
              :committee="screen.committee"
              :criteria="screen.criteria"
              :max-weighted-score="screen.max_weighted_score"
              :blind-veiled="screen.blind_veiled"
              :veiled-count="screen.veiled_count"
              :required-reviews="screen.required_reviews"
              :timezone="timezone"
            />
          </div>
        </div>

        <!-- LES ÉCHANGES, SOUS LE PANNEAU et sur toute la largeur : un fil de
             discussion dans une colonne de 400 px se lit trois mots par ligne. -->
        <AdminReviewComments
          class="mt-8"
          :comments="screen.comments"
          :participants="screen.participants"
          :viewer-id="auth.person?.id ?? null"
          :lead-organization-name="leadOrganizationName"
          :timezone="timezone"
          :can-write="screen.permissions.can_review || screen.permissions.can_decide"
          :busy="busy"
          :error="commentError"
          @post="postComment"
        />

        <AdminReviewRecusalDialog
          v-model:open="recusalOpen"
          :busy="busy"
          :error="reviewError"
          @submit="recuse"
        />

        <AdminReviewDecisionDialog
          v-model:open="decisionOpen"
          :option="pendingDecision"
          :reference-code="screen.proposal.reference_code"
          :current-status="screen.proposal.status"
          :busy="busy"
          :error="decisionError"
          @submit="decide"
        />
      </template>
    </template>
  </div>
</template>
