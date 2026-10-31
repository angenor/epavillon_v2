<script setup lang="ts">
import type { PostCommentPayload, ReviewDeskScreen, SaveReviewPayload } from '~/types/admin-review'
import type { EffectivePermission } from '~/types/identity'
import type {
  CommentPerson,
  ProposalCommentView,
  ProposalStatus,
  ProposalTransitionRule,
} from '~/types/programme/proposal'
import type { PersonId, Uuid } from '~/types/shared'
import type { TabItem } from '~/types/ui'
import type { DecisionOption } from '~/utils/review-scoring'

/**
 * LE DOSSIER, EN TABLEAU DE BORD — `/admin/propositions/:id` (01/11, maquette
 * `docs/maquettes/dossier-admin/` validée par le commanditaire).
 *
 * TOUT S'Y FAIT SANS QUITTER LA PAGE : lire, joindre le déposant, placer
 * l'activité, rattacher le dossier à la bonne organisation, noter, trancher.
 *
 * COMPOSITION :
 *  · à gauche, LES AUTRES DOSSIERS (`AdminReviewNavigator`), repliable ; en
 *    tiroir sous 1024 px ;
 *  · en-tête : titre, état, une seule action principale (« Écrire au
 *    déposant ») ; « Changer le statut » déplie la décision ;
 *  · la bande de programmation : souhaité, écart, retenu. Le créneau retenu
 *    vit sur la séance, jamais sur le dossier ;
 *  · le corps : présentation, personnes à joindre, intervenants, autres
 *    dossiers de l'organisation, pièces, échanges ; en marge, l'organisation
 *    porteuse, les chiffres, la fiche du dossier ;
 *  · en bas à droite, les deux fenêtres flottantes (échanges, évaluation),
 *    inchangées : noter est facultatif (arbitré le 16/09).
 *
 * L'ÉVALUATION EN AVEUGLE est appliquée À LA SOURCE (`blind_review`) : tant que
 * ma revue n'est pas déposée, les revues de mes pairs ne sont pas envoyées.
 *
 * Chaque geste se teste par PERMISSION sur l'édition regardée :
 * `programme.review.write` noter, `programme.proposal.decide` trancher,
 * `programme.proposal.edit` corriger et rattacher, `programme.session.schedule`
 * placer.
 *
 * RÈGLE N° 8 : un dossier hors périmètre se refuse comme un inexistant ; ses
 * autres dossiers d'une édition hors périmètre ne sont pas envoyés.
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

/**
 * LE FIL, LU PAR LA PERSONNE CONNECTÉE : à part de la fiche, parce qu'il change
 * seul (lu, non lu, destinataires) et que recharger la fiche reposerait un
 * accusé d'ouverture du dossier.
 */
const {
  data: comments,
  status: commentsStatus,
  error: commentsError,
  refresh: refreshComments,
} = await useAsyncData<ProposalCommentView[]>(
  () => `review-comments-${proposalId.value}`,
  () => api.proposals.comments(proposalId.value, auth.person?.id ?? null),
  { watch: [proposalId], default: () => [], lazy: true },
)

const { data: commentAudience } = await useAsyncData<CommentPerson[]>(
  () => `review-comment-audience-${proposalId.value}`,
  () => api.review.commentAudience(proposalId.value),
  { watch: [proposalId], default: () => [], lazy: true },
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

// ---------------------------------------------------------------------------
// Les autres dossiers, les fenêtres flottantes
// ---------------------------------------------------------------------------

const nav = useProposalNavigator()
const neighbours = computed(() => nav.neighbours(proposalId.value))

const currentUnread = computed(() =>
  commentsStatus.value === 'success' ? comments.value.filter((comment) => !comment.is_read).length : null,
)

watch(currentUnread, (count) => {
  if (count !== null) nav.setUnreadCount(proposalId.value, count)
})

const navOpen = useState('review-navigator-open', () => true)
const drawerOpen = ref(false)
const desk = useState<'comments' | 'score' | null>('review-desk-open', () => null)

function setDesk(name: 'comments' | 'score', open: boolean): void {
  if (open) desk.value = name
  else if (desk.value === name) desk.value = null
}

/**
 * Le panneau des dossiers prend la place de la barre du back-office : elle se
 * replie en rail tant qu'il est ouvert, et reprend son état en quittant les fiches.
 */
const sidebar = useAdminSidebar()
const sidebarBefore = useState<boolean | null>('review-sidebar-before', () => null)

function restoreSidebar(): void {
  if (sidebarBefore.value === null) return
  sidebar.setCollapsed(sidebarBefore.value, { persist: false })
  sidebarBefore.value = null
}

function syncSidebar(open: boolean): void {
  if (!window.matchMedia('(min-width: 64rem)').matches) return
  if (!open) return restoreSidebar()
  if (sidebarBefore.value === null) sidebarBefore.value = sidebar.isCollapsed.value
  sidebar.setCollapsed(true, { persist: false })
}

onMounted(() => syncSidebar(navOpen.value))
watch(navOpen, (open) => syncSidebar(open))
onBeforeRouteLeave((to) => {
  if (!/\/admin\/propositions\/[^/]+$/.test(to.path)) restoreSidebar()
})

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

const figures = computed(() => {
  const counts = screen.value?.dashboard.counts
  return counts
    ? [
        { key: 'registrations', value: counts.registrations },
        { key: 'speakers', value: counts.speakers },
        { key: 'documents', value: counts.documents },
      ]
    : []
})

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

/** Retour de la page de modification : l'avis s'affiche une fois, puis le paramètre s'efface. */
onMounted(() => {
  if (route.query.modifie === undefined) return
  notice.value = t('admin.proposal.edit.saved')
  const { modifie: _, ...rest } = route.query
  router.replace({ query: rest })
})

/** Un brouillon reste à son organisation ; un dossier clos ou une édition passée ne bougent plus. */
const canEditContent = computed(() => {
  const current = screen.value
  if (!current?.permissions.can_edit) return false
  return current.proposal.status !== 'draft' && canEditProposal(current.proposal, current.edition)
})

const editTo = computed(() => localePath(`/admin/propositions/${proposalId.value}/modifier`))

/** La rubrique Courriels, sur un brouillon adressé au déposant. */
const writeTo = computed(() => localePath({ path: '/admin/courriels/nouveau', query: { dossier: proposalId.value } }))

const decisionShown = ref(false)
watch(proposalId, () => (decisionShown.value = false))

const plannerTo = computed(() => localePath('/admin/programmation'))

const canSchedule = computed(() => Boolean(screen.value?.permissions.can_schedule))
const canReassign = computed(() => {
  const current = screen.value
  return Boolean(current?.permissions.can_edit) && current?.proposal.status !== 'draft'
})

// ---------------------------------------------------------------------------
// Le créneau, l'organisation porteuse, le déposant
// ---------------------------------------------------------------------------

const slotPanel = ref<{ sessionId: string; fromPreferred: boolean } | null>(null)
const slotPanelOpen = computed({
  get: () => slotPanel.value !== null,
  set: (open: boolean) => {
    if (!open) slotPanel.value = null
  },
})
const slotHasMeeting = computed(() =>
  Boolean(screen.value?.dashboard.sessions.find((slot) => slot.id === slotPanel.value?.sessionId)?.meeting),
)

async function slotSaved(summary: { message: string; conflicts: number }): Promise<void> {
  notice.value = summary.conflicts
    ? `${summary.message} ${t('admin.proposal.dashboard.panel.conflicts', summary.conflicts)}`
    : summary.message
  await refresh()
}

const leadPanelOpen = ref(false)
const submitterPanelOpen = ref(false)
const reassignBusy = ref(false)
const reassignError = ref<string | null>(null)

watch([leadPanelOpen, submitterPanelOpen], () => (reassignError.value = null))

async function changeLead(organizationId: string): Promise<void> {
  reassignBusy.value = true
  reassignError.value = null
  try {
    await api.review.changeLeadOrganization(proposalId.value, { organization_id: organizationId })
    leadPanelOpen.value = false
    notice.value = t('admin.proposal.dashboard.orgPanel.done')
    await Promise.all([refresh(), nav.refresh()])
  } catch (thrown) {
    reassignError.value = writeError(thrown)
  } finally {
    reassignBusy.value = false
  }
}

async function changeSubmitter(personId: string): Promise<void> {
  reassignBusy.value = true
  reassignError.value = null
  try {
    await api.review.changeSubmitter(proposalId.value, { person_id: personId })
    submitterPanelOpen.value = false
    notice.value = t('admin.proposal.dashboard.submitterPanel.done')
    await refresh()
  } catch (thrown) {
    reassignError.value = writeError(thrown)
  } finally {
    reassignBusy.value = false
  }
}

/** Super-administrateur seulement : agir dans l'espace de l'organisation, à sa demande (30/10). */
const canActForOrganization = computed(() => hasPermission(granted.value, 'programme.proposal.act_for_organization'))
const organizationSpaceTo = computed(() => localePath(`/mon-organisation/dossiers/${proposalId.value}`))

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
    await Promise.all([refresh(), nav.refresh()])
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

async function postComment(payload: Omit<PostCommentPayload, 'proposal_id'>): Promise<boolean> {
  const person = auth.person
  if (!person || !screen.value) return false
  busy.value = true
  commentError.value = null
  try {
    await api.review.comment(person.id, { ...payload, proposal_id: screen.value.proposal.id })
    notice.value = t('admin.proposal.review.comments.sent')
    // Une demande de correction change l'état du dossier : la fiche suit.
    await Promise.all([refreshComments(), payload.is_change_request ? refresh() : Promise.resolve()])
    return true
  } catch (thrown) {
    commentError.value = writeError(thrown)
    return false
  } finally {
    busy.value = false
  }
}

function patchRead(ids: Set<Uuid>, isRead: boolean): void {
  comments.value = comments.value.map((comment) =>
    ids.has(comment.id) ? { ...comment, is_read: isRead } : comment,
  )
}

/** Lecture silencieuse : un échec laisse le point, et le message sera repris à sa prochaine vue. */
async function markCommentsRead(ids: Uuid[]): Promise<void> {
  const person = auth.person
  if (!person || ids.length === 0) return
  await api.proposals.markCommentsRead(person.id, proposalId.value, ids)
  patchRead(new Set(ids), true)
}

async function markCommentUnread(commentId: Uuid): Promise<void> {
  const person = auth.person
  if (!person) return
  commentError.value = null
  try {
    await api.proposals.markCommentUnread(person.id, commentId)
    patchRead(new Set([commentId]), false)
  } catch (thrown) {
    commentError.value = writeError(thrown)
  }
}

async function addCommentRecipients(rootId: Uuid, personIds: PersonId[]): Promise<boolean> {
  const person = auth.person
  if (!person) return false
  busy.value = true
  commentError.value = null
  try {
    await api.review.addCommentRecipients(person.id, rootId, personIds)
    notice.value = t('admin.proposal.review.comments.recipients.addedNotice', personIds.length)
    await refreshComments()
    return true
  } catch (thrown) {
    commentError.value = writeError(thrown)
    return false
  } finally {
    busy.value = false
  }
}

// ---------------------------------------------------------------------------
// Temps réel : un message, un changement d'état ou la liste bougent sans recharger
// ---------------------------------------------------------------------------

const toast = useToast()

/** Relit le fil ; fenêtre fermée, un message d'autrui se signale en bref. */
async function rereadComments(): Promise<void> {
  const loaded = commentsStatus.value === 'success'
  const known = new Set(comments.value.map((comment) => comment.id))
  await refreshComments()
  if (!loaded || desk.value === 'comments' || document.visibilityState !== 'visible') return
  const last = commentArrivals(known, comments.value, auth.person?.id).at(-1)
  if (!last || !screen.value) return
  toast.info(t('admin.proposal.review.comments.arrived.title'), {
    message: t('admin.proposal.review.comments.arrived.message', {
      author: personName(last.author_id) ?? leadOrganizationName.value,
      reference: screen.value.proposal.reference_code,
    }),
  })
}

useRealtime(
  () => {
    const id = proposalId.value
    const eventId = adminScope.currentEventId
    return [`proposal:${id}`, `proposal:${id}:status`, ...(eventId ? [`event:${eventId}:proposals`] : [])]
  },
  async (topics) => {
    const id = proposalId.value
    await Promise.all([
      topics.includes(`proposal:${id}`) ? rereadComments() : null,
      // La fiche entière : une note en cours de saisie reste (`AdminReviewScorePanel`).
      topics.includes(`proposal:${id}:status`) ? refresh() : null,
      topics.some((topic) => topic.endsWith(':proposals')) ? nav.refresh() : null,
    ])
  },
)

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
      await Promise.all([refresh(), nav.refresh()])
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
  <!-- pb-36 : les deux boutons flottants ne couvrent jamais la fin du dossier. -->
  <div class="mx-auto w-full max-w-[110rem] pb-36">
    <!-- ACCÈS REFUSÉ : aucun droit d'administration, ou édition hors périmètre.
         Distinct d'un dossier introuvable : l'un dit « vous n'avez pas ce
         droit », l'autre « ce dossier n'existe pas ». -->
    <UiForbiddenState
      v-if="(!adminScope.isLoading && !adminScope.canAdminister) || isForbiddenError(error)"
      :required-scope="t('admin.proposal.review.forbidden.scope')"
      action-to="/admin/propositions"
      :action-label="t('admin.proposal.review.backToList')"
    />

    <div v-else class="lg:flex lg:items-start lg:gap-8">
      <!-- LES AUTRES DOSSIERS : colonne collante sur grand écran, rail une fois
           replié. Sous 1024 px, ils passent dans un tiroir. -->
      <aside
        v-if="navOpen"
        class="sticky top-[calc(var(--admin-header-height)+1rem)] hidden h-[calc(100dvh-var(--admin-header-height)-2rem)] w-76 shrink-0 overflow-hidden rounded-lg border border-border bg-surface-raised lg:block"
      >
        <AdminReviewNavigator
          :current-id="proposalId"
          collapsible
          @collapse="navOpen = false"
        />
      </aside>
      <div
        v-else
        class="sticky top-[calc(var(--admin-header-height)+1rem)] hidden shrink-0 lg:block"
      >
        <UiButton
          variant="secondary"
          icon="sidebar"
          icon-only
          :label="t('admin.proposal.review.navigator.expand')"
          @click="navOpen = true"
        />
      </div>

      <div class="@container min-w-0 flex-1">
        <div class="flex flex-wrap items-center gap-x-4 gap-y-2">
          <NuxtLink :to="nav.listLink.value" class="inline-flex min-h-(--target-min) items-center gap-1.5 text-sm no-underline">
            <UiIcon name="arrow-left" size="1rem" :stroke-width="1.8" />
            {{ t('admin.proposal.review.backToList') }}
          </NuxtLink>
          <UiButton
            class="lg:hidden"
            variant="secondary"
            icon="sidebar"
            @click="drawerOpen = true"
          >
            {{ t('admin.proposal.review.navigator.open') }}
          </UiButton>
        </div>

        <UiErrorState
          v-if="error"
          class="mt-6"
          :retry-label="t('common.actions.retry')"
          @retry="refresh()"
        />

        <div v-else-if="status === 'pending' && !screen" class="mt-6 flex flex-col gap-4">
          <UiSkeletonLoader height="12rem" />
          <UiSkeletonLoader height="28rem" />
        </div>

        <UiEmptyState
          v-else-if="!screen"
          class="mt-6"
          icon="inbox"
          :title="t('admin.proposal.review.notFound.title')"
          :description="t('admin.proposal.review.notFound.description')"
          :action-label="t('admin.proposal.review.notFound.action')"
          :action-to="localePath('/admin/propositions')"
        />

        <template v-else>
          <AdminReviewHeader
            class="mt-4"
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
            :show-decision="decisionShown"
            @decide="openDecision"
          >
            <template #actions>
              <UiButton icon="mail" :to="writeTo" :title="t('admin.proposal.dashboard.writeToSubmitterHint')">
                {{ t('admin.proposal.dashboard.writeToSubmitter') }}
              </UiButton>
              <UiButton
                v-if="screen.permissions.can_decide"
                variant="secondary"
                :aria-expanded="decisionShown"
                aria-controls="review-decision"
                @click="decisionShown = !decisionShown"
              >
                {{ t('admin.proposal.dashboard.changeStatus') }}
              </UiButton>
              <UiButton v-if="canEditContent" variant="secondary" icon="edit" :to="editTo">
                {{ t('admin.proposal.edit.action') }}
              </UiButton>
              <UiButton
                v-if="canActForOrganization"
                variant="secondary"
                icon="building"
                :to="organizationSpaceTo"
                :label="t('admin.proposal.review.actForOrganization')"
              />
              <UiButton
                variant="ghost"
                icon="chevron-left"
                icon-only
                :disabled="!neighbours.previous"
                :to="neighbours.previous ? nav.linkTo(neighbours.previous.id) : undefined"
                :label="t('admin.proposal.review.navigator.previous', { reference: neighbours.previous?.reference_code ?? '' })"
              />
              <UiButton
                variant="ghost"
                icon="chevron-right"
                icon-only
                :disabled="!neighbours.next"
                :to="neighbours.next ? nav.linkTo(neighbours.next.id) : undefined"
                :label="t('admin.proposal.review.navigator.next', { reference: neighbours.next?.reference_code ?? '' })"
              />
            </template>
          </AdminReviewHeader>

          <UiAlert
            v-if="notice"
            class="mt-4"
            intent="success"
            live
            dismissible
            :message="notice"
            @dismiss="notice = null"
          />

          <AdminReviewSlotBand
            class="mt-8"
            :proposal="screen.proposal"
            :sessions="screen.dashboard.sessions"
            :timezone="timezone"
            :zone-label="zoneLabel"
            :can-schedule="canSchedule"
            :planner-to="plannerTo"
            @edit="(sessionId: string) => (slotPanel = { sessionId, fromPreferred: false })"
            @place="(sessionId: string, fromPreferred: boolean) => (slotPanel = { sessionId, fromPreferred })"
          />

          <UiTabs
            class="mt-10"
            :items="tabs"
            :model-value="activeTab"
            :label="t('admin.proposal.review.tabs.dossier')"
            @update:model-value="selectTab"
          />

          <!-- LE CORPS : colonne principale, et la marge du dossier. Sous 48 rem
               de large, la marge passe sous le texte. -->
          <div
            v-if="activeTab === 'dossier'"
            class="mt-8 grid grid-cols-1 gap-x-8 gap-y-10 @3xl:grid-cols-[minmax(0,1fr)_19rem]"
          >
            <div class="flex min-w-0 flex-col gap-10">
              <section aria-labelledby="review-project-title">
                <h2 id="review-project-title" class="border-b border-border pb-2 text-lg font-semibold">
                  {{ t('admin.proposal.dashboard.sections.presentation') }}
                </h2>
                <AdminReviewDossier class="mt-4" :proposal="screen.proposal" />
              </section>

              <AdminReviewContacts
                :submitter="screen.dashboard.submitter"
                :contact="screen.dashboard.contact"
                :can-change-submitter="canReassign"
                @change-submitter="submitterPanelOpen = true"
              />

              <AdminReviewSpeakers :entries="screen.speakers" :timezone="timezone" />

              <AdminReviewOtherProposals
                :entries="screen.dashboard.other_proposals"
                :organization-id="screen.dashboard.lead_organization_id"
              />

              <section aria-labelledby="dashboard-documents-title">
                <h2
                  id="dashboard-documents-title"
                  class="flex items-baseline gap-x-3 border-b border-border pb-2 text-lg font-semibold"
                >
                  {{ t('admin.proposal.dashboard.sections.documents') }}
                  <small class="text-sm font-normal text-text-muted">{{ screen.documents.length }}</small>
                </h2>
                <AdminReviewDocuments class="mt-3" :entries="screen.documents" :timezone="timezone" />
              </section>

              <!-- ÉCHANGES ET ÉVALUATION : les fenêtres flottantes d'aujourd'hui,
                   inchangées ; ce bloc les rend trouvables depuis le corps. -->
              <section aria-labelledby="dashboard-exchanges-title">
                <h2 id="dashboard-exchanges-title" class="border-b border-border pb-2 text-lg font-semibold">
                  {{ t('admin.proposal.dashboard.sections.exchanges') }}
                </h2>
                <p class="mt-3 max-w-(--measure) text-sm text-text-secondary">
                  {{ t('admin.proposal.dashboard.sections.exchangesHint') }}
                </p>
                <div class="mt-3 flex flex-wrap gap-2">
                  <UiButton variant="secondary" icon="chat" @click="setDesk('comments', true)">
                    {{ t('admin.proposal.dashboard.sections.openComments') }}
                  </UiButton>
                  <UiButton variant="secondary" icon="star" @click="setDesk('score', true)">
                    {{ t('admin.proposal.dashboard.sections.openScore') }}
                  </UiButton>
                  <UiButton variant="ghost" icon="clock" @click="selectTab('history')">
                    {{ t('admin.proposal.dashboard.sections.openHistory') }}
                  </UiButton>
                </div>
              </section>
            </div>

            <aside class="flex flex-col gap-4" :aria-label="t('admin.proposal.dashboard.asideLabel')">
              <AdminReviewLeadOrganizationCard
                :entries="screen.organizations"
                :dashboard="screen.dashboard"
                :can-change="canReassign"
                @change="leadPanelOpen = true"
              />

              <section class="rounded-lg border border-border bg-surface-raised p-5">
                <h2 class="text-base font-semibold">{{ t('admin.proposal.dashboard.figures.title') }}</h2>
                <dl class="mt-3 grid grid-cols-2 gap-4">
                  <div v-for="figure in figures" :key="figure.key" class="flex flex-col-reverse">
                    <dt class="mt-1 text-sm text-text-muted">
                      {{ t(`admin.proposal.dashboard.figures.${figure.key}`, figure.value) }}
                    </dt>
                    <dd class="text-2xl leading-none font-light tabular-nums">{{ figure.value }}</dd>
                  </div>
                </dl>
              </section>

              <!-- EMPLACEMENT DE LA CARTE « COURRIELS ENVOYÉS » : le composant
                   AdminMailingsProposalMailings (rubrique Courriels), à brancher
                   ici avec :proposal-id="screen.proposal.id". -->

              <section class="rounded-lg border border-border bg-surface-raised p-5">
                <h2 class="text-base font-semibold">{{ t('admin.proposal.dashboard.sections.facts') }}</h2>
                <AdminReviewDossierFacts
                  class="mt-3"
                  :proposal="screen.proposal"
                  :themes="screen.themes"
                  :timezone="timezone"
                  :zone-label="zoneLabel"
                  :country-name="countryName"
                  :submitter-name="personName(screen.proposal.submitted_by)"
                  :contact-name="personName(screen.proposal.contact_person_id)"
                />
              </section>
            </aside>
          </div>

          <!-- L'HISTORIQUE CHAMP PAR CHAMP est le même composant que l'espace
               organisation : `programme.proposal_history()` rend la même chose
               aux deux écrans. -->
          <WorkspaceHistoryList v-else class="mt-8" :entries="screen.history" :timezone="timezone" />

          <AdminReviewSlotPanel
            v-model:open="slotPanelOpen"
            :event-id="screen.proposal.event_id"
            :session-id="slotPanel?.sessionId ?? null"
            :apply-preferred="slotPanel?.fromPreferred ?? false"
            :has-meeting="slotHasMeeting"
            :write-to="writeTo"
            @saved="slotSaved"
          />

          <AdminReviewLeadOrganizationPanel
            v-model:open="leadPanelOpen"
            :current-id="screen.dashboard.lead_organization_id"
            :current-name="leadOrganizationName"
            :busy="reassignBusy"
            :error="reassignError"
            @submit="changeLead"
          />

          <AdminReviewSubmitterPanel
            v-model:open="submitterPanelOpen"
            :current-id="screen.proposal.submitted_by"
            :current-name="screen.dashboard.submitter?.display_name ?? ''"
            :busy="reassignBusy"
            :error="reassignError"
            @submit="changeSubmitter"
          />

          <AdminReviewCommentsDesk
            :open="desk === 'comments'"
            :stack="0"
            :reference-code="screen.proposal.reference_code"
            :comments="comments"
            :loading="commentsStatus === 'pending' && !busy"
            :failed="Boolean(commentsError)"
            :participants="screen.participants"
            :audience="commentAudience"
            :viewer-id="auth.person?.id ?? null"
            :lead-organization-name="leadOrganizationName"
            :timezone="timezone"
            :zone-label="zoneLabel"
            :can-write="screen.permissions.can_review || screen.permissions.can_decide"
            :busy="busy"
            :error="commentError"
            :submit="postComment"
            :mark-read="markCommentsRead"
            :add-recipients="addCommentRecipients"
            @update:open="(value: boolean) => setDesk('comments', value)"
            @unread="markCommentUnread"
            @retry="refreshComments()"
          />

          <AdminReviewFloatingDesk
            :open="desk === 'score'"
            :stack="1"
            :reference-code="screen.proposal.reference_code"
            :criteria="screen.criteria"
            :uses-scoring-grid="screen.uses_scoring_grid"
            :max-weighted-score="screen.max_weighted_score"
            :my-review="screen.my_review"
            :permissions="screen.permissions"
            :peer-reviews="screen.peer_reviews"
            :committee="screen.committee"
            :blind-veiled="screen.blind_veiled"
            :veiled-count="screen.veiled_count"
            :required-reviews="screen.required_reviews"
            :timezone="timezone"
            :busy="busy"
            :error="reviewError"
            :saved-at="savedAt"
            @update:open="(value: boolean) => setDesk('score', value)"
            @save="saveReview"
            @recuse="recusalOpen = true"
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
            :proposal-id="screen.proposal.id"
            :reference-code="screen.proposal.reference_code"
            :current-status="screen.proposal.status"
            :busy="busy"
            :error="decisionError"
            @submit="decide"
          />
        </template>
      </div>
    </div>

    <UiDrawer
      v-model:open="drawerOpen"
      side="left"
      width="22rem"
      :title="t('admin.proposal.review.navigator.open')"
    >
      <div v-if="drawerOpen" class="-mx-5 -my-4 h-[calc(100%+2rem)]">
        <AdminReviewNavigator
          :current-id="proposalId"
          @navigate="drawerOpen = false"
        />
      </div>
    </UiDrawer>
  </div>
</template>
