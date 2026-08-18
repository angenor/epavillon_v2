<script setup lang="ts">
import type {
  ProposalFacet,
  ProposalFilterText,
  ProposalFlag,
  ProposalListFilters,
  ProposalListScreen,
  ProposalSortKey,
} from '~/types/admin-proposals'
import type { EffectivePermission } from '~/types/identity'
import type { ProposalStatus, ProposalTransitionRule } from '~/types/programme/proposal'
import type { ParticipationMode } from '~/types/event/edition'
import type { ProposalDashboardRow } from '~/types/views'
import type { SortDirection } from '~/types/ui'
import type { CsvColumn } from '~/utils/proposal-list'

/**
 * LISTE DES PROPOSITIONS REÇUES — `/admin/propositions`.
 *
 * L'ÉCRAN OÙ LE COMITÉ TRAVAILLE. Quarante dossiers, onze colonnes, et une seule
 * question à l'ouverture : lesquels tiennent le haut du classement, et lesquels
 * n'ont encore été lus par personne. D'où le tri par défaut sur la NOTE
 * DÉCROISSANTE — c'est le prompt qui le demande, et c'est ce que fait déjà
 * l'index `ix_proposals_ranking` de la base.
 *
 * LES BROUILLONS SONT LÀ, MARQUÉS COMME TELS. « Propositions reçues » inviterait
 * à les masquer ; l'équipe de l'IFDD a besoin de voir, à trois jours de
 * l'échéance, qu'il reste cinq dossiers commencés et jamais déposés. La vue ne
 * filtre que les dossiers supprimés, l'entonnoir du tableau de bord les compte
 * déjà, et le filtre de statut permet de les écarter en un clic.
 *
 * TOUT L'ÉTAT VIT DANS L'URL : filtres, tri, page. Trois raisons, et aucune
 * n'est cosmétique — le tableau de bord renvoie ici avec un filtre posé
 * (`?filtre=non-evaluees`), une liste filtrée se transmet par courriel entre
 * deux membres du comité, et le jour où le filtrage partira au serveur (B7),
 * ces paramètres deviendront ceux de la requête sans qu'un composant change.
 *
 * RÈGLE MÉTIER N° 8 — LE PÉRIMÈTRE D'ADMINISTRATION. Un administrateur peut
 * n'avoir accès qu'à une seule édition ; la liste est filtrée par ce périmètre,
 * y compris quand l'URL est forgée. `useApi()` REFUSE une édition hors périmètre
 * plutôt que de rendre une liste vide — les deux ne se lisent pas pareil.
 *
 * LES ACTIONS SE TESTENT PAR PERMISSION, jamais par nom de rôle : affecter
 * demande `event.call.manage`, changer un statut `programme.proposal.decide`,
 * chacune sur la portée de l'édition regardée. Ce que fait l'écran est de
 * l'affichage ; le refus appartient à l'API.
 *
 * QUATRE ÉTATS : chargement (lignes squelettes à la forme du tableau), erreur
 * avec reprise, vide (aucune édition, ou aucun résultat après filtrage — deux
 * messages différents), accès refusé.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.proposals' }],
})

const { t } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()

useHead(() => ({ title: t('admin.proposals.title') }))

await adminScope.ensureLoaded()

// ---------------------------------------------------------------------------
// Données
// ---------------------------------------------------------------------------

const {
  data: screen,
  status,
  error,
  refresh,
} = await useAsyncData<ProposalListScreen | null>(
  'admin-proposal-list',
  async () => {
    const eventId = adminScope.currentEventId
    if (!eventId) return null
    return api.proposals.list(eventId, adminScope.scope, auth.person?.id ?? null)
  },
  { watch: [() => adminScope.currentEventId], lazy: true },
)

/** Permissions effectives : ce que cette personne peut faire, et sur quoi. */
const { data: granted } = await useAsyncData<EffectivePermission[]>(
  'admin-proposal-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

/** La machine à états, lue une fois : elle ne dépend d'aucune édition. */
const { data: transitionRules } = await useAsyncData<ProposalTransitionRule[]>(
  'proposal-transition-rules',
  () => api.proposals.transitionRules(),
  { default: () => [], lazy: true },
)

const { data: committee } = await useAsyncData<ProposalFacet[]>(
  'admin-proposal-committee',
  async () => {
    const eventId = adminScope.currentEventId
    if (!eventId) return []
    return api.proposals.committee(eventId, adminScope.scope)
  },
  { default: () => [], watch: [() => adminScope.currentEventId], lazy: true },
)

const canAssign = computed(() =>
  hasPermission(granted.value, 'event.call.manage', adminScope.currentEventId),
)
const canDecide = computed(() =>
  hasPermission(granted.value, 'programme.proposal.decide', adminScope.currentEventId),
)

// ---------------------------------------------------------------------------
// Filtres, tri et pagination — portés par l'URL
// ---------------------------------------------------------------------------

const PER_PAGE = 20

/** Une valeur de requête peut arriver seule ou répétée : on rend toujours une liste. */
function queryList(value: unknown): string[] {
  if (Array.isArray(value)) return value.flatMap((entry) => String(entry).split(',')).filter(Boolean)
  if (typeof value === 'string') return value.split(',').filter(Boolean)
  return []
}

function queryText(value: unknown): string {
  return typeof value === 'string' ? value : ''
}

/**
 * Les paramètres sont en FRANÇAIS et minuscules : ils apparaissent dans une URL
 * qu'on se transmet par courriel, et le tableau de bord en pose déjà un
 * (`?filtre=non-evaluees`). Ce contrat-là est public, il ne suit pas les noms de
 * colonnes de la base.
 */
const FLAG_PARAM: Record<string, ProposalFlag> = {
  'non-evaluees': 'unreviewed',
  'en-retard': 'late',
  'non-consultees': 'unread',
}
const FLAG_TO_PARAM: Record<ProposalFlag, string> = {
  unreviewed: 'non-evaluees',
  late: 'en-retard',
  unread: 'non-consultees',
}

const filters = computed<ProposalListFilters>(() => ({
  search: queryText(route.query.q),
  statuses: queryList(route.query.statut) as ProposalStatus[],
  themes: queryList(route.query.thematique),
  formats: queryList(route.query.format) as ParticipationMode[],
  countries: queryList(route.query.pays),
  organizations: queryList(route.query.organisation),
  reviewer: queryText(route.query.revisionniste) || null,
  flags: queryList(route.query.filtre)
    .map((value) => FLAG_PARAM[value])
    .filter((flag): flag is ProposalFlag => Boolean(flag)),
}))

const SORT_PARAM: Record<string, ProposalSortKey> = {
  dossier: 'reference_code',
  titre: 'title',
  organisation: 'organization',
  pays: 'country',
  format: 'format',
  statut: 'status',
  revues: 'reviews',
  note: 'average_score',
  rang: 'event_rank',
  depot: 'submitted_at',
}
const SORT_TO_PARAM = Object.fromEntries(
  Object.entries(SORT_PARAM).map(([param, key]) => [key, param]),
) as Record<ProposalSortKey, string>

/** Le tri par défaut : la note, décroissante. C'est la question du comité. */
const sortKey = computed<ProposalSortKey>(
  () => SORT_PARAM[queryText(route.query.tri)] ?? 'average_score',
)
const sortDirection = computed<Exclude<SortDirection, null>>(() =>
  queryText(route.query.sens) === 'asc' ? 'asc' : 'desc',
)
const page = computed(() => Math.max(1, Number.parseInt(queryText(route.query.page) || '1', 10) || 1))

/** Écrit dans l'URL ; une valeur vide retire son paramètre plutôt que de l'y laisser. */
function updateQuery(patch: Record<string, string | string[] | null>): void {
  const query: Record<string, string | string[]> = { ...route.query } as Record<string, string | string[]>
  for (const [key, value] of Object.entries(patch)) {
    if (value === null || value === '' || (Array.isArray(value) && value.length === 0)) delete query[key]
    else query[key] = value
  }
  router.replace({ query })
}

function setFilters(next: ProposalListFilters): void {
  updateQuery({
    q: next.search.trim() || null,
    statut: next.statuses.join(','),
    thematique: next.themes.join(','),
    format: next.formats.join(','),
    pays: next.countries.join(','),
    organisation: next.organizations.join(','),
    revisionniste: next.reviewer,
    filtre: next.flags.map((flag) => FLAG_TO_PARAM[flag]).join(','),
    // Tout changement de filtre ramène en page 1 : rester en page 3 d'un
    // résultat qui n'en compte plus qu'une donne un tableau vide et une
    // pagination qui affirme le contraire.
    page: null,
  })
}

function setSort(key: ProposalSortKey, direction: Exclude<SortDirection, null>): void {
  updateQuery({ tri: SORT_TO_PARAM[key], sens: direction, page: null })
}

// ---------------------------------------------------------------------------
// Ce que l'écran affiche
// ---------------------------------------------------------------------------

const allRows = computed(() => screen.value?.rows ?? [])
const unreadIds = computed(() => new Set(screen.value?.unread_ids ?? []))
const timezone = computed(() => screen.value?.timezone ?? 'UTC')
const zoneLabel = computed(() => screen.value?.city?.trim() || timezone.value)

/**
 * Les textes que les fonctions pures de tri et d'export ne peuvent pas produire :
 * un statut et un format sont des libellés d'interface, un pays est une donnée
 * multilingue de la base. Trier sur le code d'ENUM donnerait un ordre qui n'a de
 * sens dans aucune des deux langues.
 */
const rowText = computed<ProposalFilterText>(() => ({
  status: (row) => t(`admin.proposals.status.${row.status}`),
  format: (row) => t(`admin.proposals.format.${row.format}`),
  country: (row) => (row.organization_country ? tr(row.organization_country) : ''),
}))

const filteredRows = computed(() =>
  sortProposals(
    filterProposals(allRows.value, filters.value, unreadIds.value),
    sortKey.value,
    sortDirection.value,
    rowText.value,
  ),
)

const pageRows = computed(() =>
  filteredRows.value.slice((page.value - 1) * PER_PAGE, page.value * PER_PAGE),
)

/** Légende du tableau, lue par les lecteurs d'écran : périmètre et tri. */
const caption = computed(() =>
  t('admin.proposals.caption', {
    event: adminScope.currentEvent ? tr(adminScope.currentEvent.title) : '',
    count: filteredRows.value.length,
    sort: t(`admin.proposals.columns.${captionSortKey.value}`),
  }),
)

const captionSortKey = computed(() => {
  const map: Record<ProposalSortKey, string> = {
    reference_code: 'reference',
    title: 'title',
    organization: 'organization',
    country: 'country',
    format: 'format',
    status: 'status',
    reviews: 'reviews',
    average_score: 'score',
    event_rank: 'rank',
    submitted_at: 'submitted',
  }
  return map[sortKey.value]
})

// ---------------------------------------------------------------------------
// Sélection et actions groupées
// ---------------------------------------------------------------------------

/**
 * LA SÉLECTION EST UNE LISTE DE CLÉS, portée par la page. Elle survit au
 * changement de page ; « tout sélectionner » ne touche que les lignes affichées
 * — c'est `UiTable` qui le garantit, et la barre d'actions le rappelle.
 */
const selected = ref<string[]>([])

// Changer d'édition vide la sélection : agir en masse sur des dossiers d'une
// autre COP est exactement ce que la règle n° 8 interdit.
watch(() => adminScope.currentEventId, () => (selected.value = []))

const selectedRows = computed(() => allRows.value.filter((row) => selected.value.includes(row.id)))

const statusOptions = computed(() =>
  bulkStatusOptions(selectedRows.value, transitionRules.value, granted.value, adminScope.currentEventId),
)

const assignOpen = ref(false)
const statusOpen = ref(false)
const busy = ref(false)
const actionError = ref<string | null>(null)
const actionNotice = ref<string | null>(null)
const actionSkipped = ref<string[]>([])

/**
 * CE QUI N'A PAS SUIVI, dossier par dossier — mais pas seize lignes d'affilée.
 * Au-delà de cinq, l'énumération noie l'information au lieu de la donner : on en
 * nomme cinq, et le reste est compté. Rien n'est tu, rien n'est illisible.
 */
const MAX_SKIPS_SHOWN = 5

function describeSkips(skips: { reference_code: string; reason: string }[]): string[] {
  const lines = skips
    .slice(0, MAX_SKIPS_SHOWN)
    .map((skip) => `${skip.reference_code} — ${t(`admin.proposals.bulk.reason.${skip.reason}`)}`)

  if (skips.length > MAX_SKIPS_SHOWN) {
    lines.push(t('admin.proposals.bulk.andMore', skips.length - MAX_SKIPS_SHOWN))
  }
  return lines
}

async function submitAssign(payload: { reviewerId: string; dueAt: string | null }): Promise<void> {
  busy.value = true
  actionError.value = null
  try {
    const result = await api.proposals.assignReviewer(auth.person?.id ?? null, {
      proposal_ids: selected.value,
      reviewer_id: payload.reviewerId,
      due_at: payload.dueAt,
    })
    actionNotice.value = t('admin.proposals.assign.success', result.applied.length)
    actionSkipped.value = describeSkips(result.skipped)
    assignOpen.value = false
    selected.value = []
    await refresh()
  } catch {
    actionError.value = t('admin.proposals.bulk.error')
  } finally {
    busy.value = false
  }
}

async function submitStatus(payload: { toStatus: ProposalStatus; reason: string | null }): Promise<void> {
  busy.value = true
  actionError.value = null
  try {
    const result = await api.proposals.changeStatus(auth.person?.id ?? null, {
      proposal_ids: selected.value,
      to_status: payload.toStatus,
      reason: payload.reason,
    })
    actionNotice.value = t('admin.proposals.changeStatus.success', result.applied.length)
    actionSkipped.value = describeSkips(result.skipped)
    statusOpen.value = false
    selected.value = []
    await refresh()
  } catch {
    actionError.value = t('admin.proposals.bulk.error')
  } finally {
    busy.value = false
  }
}

// ---------------------------------------------------------------------------
// Export CSV
// ---------------------------------------------------------------------------

/**
 * L'EXPORT PORTE CE QUE L'ÉCRAN MONTRE — les lignes FILTRÉES, dans l'ordre du
 * tri, ou la seule sélection quand il est lancé depuis la barre d'actions. Un
 * export qui rendrait les quarante dossiers alors que l'écran en affiche six
 * ferait douter du filtre, et c'est un fichier qu'on envoie ensuite par
 * courriel : il doit correspondre à ce qu'on croit avoir extrait.
 *
 * Le fichier est construit et téléchargé DANS LE NAVIGATEUR : quarante lignes ne
 * valent pas un aller-retour. À la bascule (B7), un export de plusieurs milliers
 * de lignes deviendra un travail différé — c'est déjà ce que `platform.jobs`
 * prévoit.
 */
function exportCsv(rows: ProposalDashboardRow[]): void {
  const columns: CsvColumn[] = [
    { header: t('admin.proposals.export.columns.reference'), value: (row) => row.reference_code },
    { header: t('admin.proposals.export.columns.title'), value: (row) => tr(row.title) },
    { header: t('admin.proposals.export.columns.organization'), value: (row) => row.organization_name },
    { header: t('admin.proposals.export.columns.coOrganizers'), value: (row) => String(row.co_organizer_count) },
    {
      header: t('admin.proposals.export.columns.country'),
      value: (row) => (row.organization_country ? tr(row.organization_country) : ''),
    },
    {
      header: t('admin.proposals.export.columns.themes'),
      value: (row) => row.themes.map((theme) => tr(theme.label)).join(' · '),
    },
    { header: t('admin.proposals.export.columns.format'), value: (row) => t(`admin.proposals.format.${row.format}`) },
    { header: t('admin.proposals.export.columns.status'), value: (row) => t(`admin.proposals.status.${row.status}`) },
    { header: t('admin.proposals.export.columns.reviewsDone'), value: (row) => String(row.review_count) },
    {
      header: t('admin.proposals.export.columns.reviewsExpected'),
      value: (row) => String(row.required_reviews ?? row.assigned_reviewers),
    },
    {
      header: t('admin.proposals.export.columns.score'),
      value: (row) => (row.average_score === null ? '' : row.average_score.toFixed(2)),
    },
    {
      header: t('admin.proposals.export.columns.rank'),
      value: (row) => (row.average_score === null ? '' : String(row.event_rank)),
    },
    {
      // La date de dépôt dans le fuseau de l'ÉDITION, comme partout ailleurs
      // dans cet écran : un tableur ne porte pas de fuseau, la valeur doit donc
      // être celle qu'on lit à l'écran.
      header: t('admin.proposals.export.columns.submitted'),
      value: (row) => (row.submitted_at ? date(row.submitted_at, timezone.value) : ''),
    },
    {
      header: t('admin.proposals.export.columns.reviewers'),
      value: (row) => row.reviewers.map((reviewer) => reviewer.name).join(' · '),
    },
  ]

  const csv = toCsv(rows, columns)
  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = csvFileName('propositions', adminScope.currentEvent?.acronym ?? null, new Date())
  link.click()
  URL.revokeObjectURL(url)

  actionNotice.value = t('admin.proposals.export.done', rows.length)
  actionSkipped.value = []
}

function openProposal(row: ProposalDashboardRow): void {
  // La fiche d'évaluation est l'écran A8 : la route est déjà celle vers laquelle
  // pointe le tableau de bord.
  navigateTo(localePath(`/admin/propositions/${row.id}`))
}

const deadlineNotice = computed(() => {
  const deadline = screen.value?.deadline
  if (!deadline) return null
  const key = Date.parse(deadline) < Date.now() ? 'passed' : 'label'
  return t(`admin.proposals.deadline.${key}`, { date: date(deadline, timezone.value) })
})
</script>

<template>
  <div class="mx-auto w-full max-w-[100rem]">
    <!-- ACCÈS REFUSÉ — aucun droit d'administration. Distinct d'un écran vide :
         l'un dit « vous n'avez pas ce droit », l'autre « il n'y a rien à voir ». -->
    <UiForbiddenState
      v-if="!adminScope.isLoading && !adminScope.canAdminister"
      :required-scope="t('admin.proposals.forbidden.scope')"
      action-to="/"
      :action-label="t('nav.admin.backToSite')"
    />

    <template v-else>
      <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-2">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">
            {{ t('admin.proposals.title') }}
          </h1>
          <p class="mt-1 text-text-muted">{{ t('admin.proposals.subtitle') }}</p>
        </div>
        <p v-if="deadlineNotice" class="text-sm text-text-subtle">{{ deadlineNotice }}</p>
      </header>

      <UiErrorState
        v-if="error"
        class="mt-8"
        :retry-label="t('common.actions.retry')"
        @retry="refresh()"
      />

      <!-- Aucune édition retenue : l'écran le dit, il n'affiche pas un tableau
           vide qui se lirait « aucun dossier ». -->
      <UiEmptyState
        v-else-if="!screen && status !== 'pending'"
        class="mt-8"
        icon="inbox"
        :title="t('admin.proposals.empty.title')"
        :description="t('admin.proposals.empty.description')"
      />

      <template v-else>
        <!-- RÉSULTAT DE LA DERNIÈRE ACTION GROUPÉE : ce qui a été fait, et ce qui
             a été écarté, dossier par dossier. Un décompte seul laisserait croire
             à un succès complet. -->
        <UiAlert
          v-if="actionNotice"
          class="mt-6"
          :intent="actionSkipped.length > 0 ? 'warning' : 'success'"
          live
          dismissible
          :title="actionNotice"
          @dismiss="actionNotice = null"
        >
          <ul v-if="actionSkipped.length > 0" class="mt-1 space-y-0.5 text-sm">
            <li class="font-medium">{{ t('admin.proposals.bulk.partial', actionSkipped.length) }}</li>
            <li v-for="line in actionSkipped" :key="line" class="text-text-secondary">{{ line }}</li>
          </ul>
        </UiAlert>

        <AdminProposalsFilters
          class="mt-6"
          :filters="filters"
          :facets="screen?.facets ?? {
            statuses: [], themes: [], formats: [], countries: [],
            organizations: [], reviewers: [], flags: [],
          }"
          :total="allRows.length"
          :shown="filteredRows.length"
          :disabled="status === 'pending'"
          @update:filters="setFilters"
        />

        <AdminProposalsBulkBar
          v-if="canAssign || canDecide"
          class="mt-4"
          :count="selected.length"
          :partial="filteredRows.length > pageRows.length || filteredRows.length < allRows.length"
          :can-assign="canAssign"
          :can-decide="canDecide"
          :busy="busy"
          @assign="assignOpen = true"
          @change-status="statusOpen = true"
          @export="exportCsv(selectedRows)"
          @clear="selected = []"
        />

        <AdminProposalsTable
          class="mt-4"
          :rows="pageRows"
          :unread-ids="unreadIds"
          :timezone="timezone"
          :required-reviews="screen?.required_reviews ?? null"
          :caption="caption"
          :sort-key="sortKey"
          :sort-direction="sortDirection"
          :selected="selected"
          :selectable="canAssign || canDecide"
          :loading="status === 'pending'"
          @sort="setSort"
          @update:selected="(keys: string[]) => (selected = keys)"
          @open="openProposal"
        >
          <template #toolbar>
            <p class="text-sm text-text-muted">
              {{ t('admin.proposals.results.sortedBy', { column: t(`admin.proposals.columns.${captionSortKey}`) }) }}
            </p>
            <UiButton
              class="ml-auto"
              variant="secondary"
              size="sm"
              icon="download"
              :disabled="filteredRows.length === 0"
              @click="exportCsv(filteredRows)"
            >
              {{ t('admin.proposals.export.action') }}
            </UiButton>
          </template>

          <template #empty>
            <!-- DEUX VIDES DIFFÉRENTS : aucun dossier du tout, ou aucun qui
                 corresponde aux filtres. Le second offre de les retirer ; les
                 confondre envoie chercher des dossiers qui sont bien là. -->
            <UiEmptyState
              v-if="allRows.length === 0"
              icon="inbox"
              :title="t('admin.proposals.noProposals.title')"
              :description="t('admin.proposals.noProposals.description')"
            />
            <UiEmptyState
              v-else
              icon="search"
              :title="t('admin.proposals.noResults.title')"
              :description="t('admin.proposals.noResults.description', { total: allRows.length })"
              :action-label="t('admin.proposals.noResults.action')"
              @action="setFilters({
                search: '', statuses: [], themes: [], formats: [],
                countries: [], organizations: [], reviewer: null, flags: [],
              })"
            />
          </template>
        </AdminProposalsTable>

        <UiPagination
          v-if="filteredRows.length > PER_PAGE"
          class="mt-4"
          :page="page"
          :per-page="PER_PAGE"
          :total="filteredRows.length"
          @update:page="(next: number) => updateQuery({ page: next > 1 ? String(next) : null })"
        />
      </template>
    </template>

    <AdminProposalsAssignDialog
      v-model:open="assignOpen"
      :count="selected.length"
      :committee="committee"
      :timezone="timezone"
      :zone-label="zoneLabel"
      :busy="busy"
      :error="actionError"
      @submit="submitAssign"
    />

    <AdminProposalsStatusDialog
      v-model:open="statusOpen"
      :count="selected.length"
      :options="statusOptions"
      :busy="busy"
      :error="actionError"
      @submit="submitStatus"
    />
  </div>
</template>
