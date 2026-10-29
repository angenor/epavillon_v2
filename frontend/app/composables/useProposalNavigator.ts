import type { ProposalFilterText, ProposalListScreen } from '~/types/admin-proposals'
import type { ProposalDashboardRow } from '~/types/views'

/**
 * LES AUTRES DOSSIERS, vus depuis une fiche : le panneau de gauche et les
 * boutons « précédent », « suivant ».
 *
 * Même appel que la liste (`api.proposals.list`), mêmes filtres portés par
 * l'URL (`useProposalListQuery`), même tri : la fiche montre la sélection que
 * la liste montrait. Rien de nouveau côté serveur.
 *
 * Passer d'un dossier à l'autre ne recharge pas la liste : elle vient du cache
 * tant qu'on ne la rafraîchit pas (une décision, une note déposée).
 */
export function useProposalNavigator() {
  const { t } = useI18n()
  const { tr } = useI18nText()
  const api = useApi()
  const auth = useAuthStore()
  const adminScope = useAdminScopeStore()
  const route = useRoute()
  const router = useRouter()
  const localePath = useLocalePath()
  const query = useProposalListQuery()

  const { data: screen, status, error, refresh } = useAsyncData<ProposalListScreen | null>(
    'review-navigator-list',
    async () => {
      const eventId = adminScope.currentEventId
      if (!eventId) return null
      return api.proposals.list(eventId, adminScope.scope, auth.person?.id ?? null)
    },
    {
      watch: [() => adminScope.currentEventId],
      lazy: true,
      getCachedData: (key, nuxtApp, context) =>
        context.cause === 'refresh:manual' || context.cause === 'watch'
          ? undefined
          : (nuxtApp.payload.data[key] as ProposalListScreen | null | undefined) ?? undefined,
    },
  )

  const allRows = computed(() => screen.value?.rows ?? [])
  const unreadIds = computed(() => new Set(screen.value?.unread_ids ?? []))
  const unreadMessages = computed(() => unreadCommentMap(screen.value?.unread_comments))
  const viewerId = computed(() => auth.person?.id ?? null)

  const rowText = computed<ProposalFilterText>(() => ({
    status: (row) => t(`admin.proposals.status.${row.status}`),
    format: (row) => t(`admin.proposals.format.${row.format}`),
    country: (row) => (row.organization_country ? tr(row.organization_country) : ''),
  }))

  const rows = computed<ProposalDashboardRow[]>(() => {
    const filtered = filterProposals(allRows.value, query.filters.value, unreadIds.value, unreadMessages.value)
      .filter((row) => !query.minePending.value || isAwaitingReviewBy(row, viewerId.value))
    return sortProposals(filtered, query.sortKey.value, query.sortDirection.value, rowText.value)
  })

  const awaitingMine = computed(
    () => allRows.value.filter((row) => isAwaitingReviewBy(row, viewerId.value)).length,
  )

  /** Lien vers un dossier, filtres compris : on passe de l'un à l'autre sans perdre la sélection. */
  function linkTo(id: string): string {
    return router.resolve({
      path: localePath(`/admin/propositions/${id}`),
      query: proposalSelectionQuery(route.query),
    }).fullPath
  }

  const listLink = computed(() =>
    router.resolve({ path: localePath('/admin/propositions'), query: proposalSelectionQuery(route.query) }).fullPath,
  )

  /** Les messages lus dans la fiche : la liste en cache date d'avant la lecture. */
  function setUnreadCount(proposalId: string, count: number): void {
    const current = screen.value
    if (!current) return
    const others = current.unread_comments.filter((entry) => entry.proposal_id !== proposalId)
    screen.value = {
      ...current,
      unread_comments: count > 0 ? [...others, { proposal_id: proposalId, count }] : others,
    }
  }

  function neighbours(currentId: string): { previous: ProposalDashboardRow | null; next: ProposalDashboardRow | null } {
    const index = rows.value.findIndex((row) => row.id === currentId)
    if (index === -1) return { previous: null, next: null }
    return { previous: rows.value[index - 1] ?? null, next: rows.value[index + 1] ?? null }
  }

  return {
    ...query,
    screen,
    status,
    error,
    refresh,
    allRows,
    rows,
    unreadIds,
    unreadMessages,
    awaitingMine,
    linkTo,
    listLink,
    neighbours,
    setUnreadCount,
  }
}
