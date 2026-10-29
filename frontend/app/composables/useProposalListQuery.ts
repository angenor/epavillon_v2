import type { LocationQuery } from 'vue-router'
import type {
  ProposalFlag,
  ProposalListFilters,
  ProposalSortKey,
} from '~/types/admin-proposals'
import type { ParticipationMode } from '~/types/event/edition'
import type { ProposalStatus } from '~/types/programme/proposal'
import type { SortDirection } from '~/types/ui'

/**
 * LES FILTRES ET LE TRI DES PROPOSITIONS, PORTÉS PAR L'URL.
 *
 * Partagés par la liste (`/admin/propositions`) et le panneau des dossiers de la
 * fiche (`/admin/propositions/:id`) : un même lien filtré ouvre la même sélection
 * aux deux endroits, et passer d'un dossier à l'autre garde les filtres.
 *
 * Les paramètres sont en FRANÇAIS et minuscules : ils apparaissent dans une URL
 * qu'on se transmet par courriel, et le tableau de bord en pose déjà un
 * (`?filtre=non-evaluees`). Ce contrat-là est public, il ne suit pas les noms de
 * colonnes de la base.
 */

const FLAG_PARAM: Record<string, ProposalFlag> = {
  'non-evaluees': 'unreviewed',
  'en-retard': 'late',
  'non-consultees': 'unread',
  'messages-non-lus': 'unread_messages',
}
const FLAG_TO_PARAM: Record<ProposalFlag, string> = {
  unreviewed: 'non-evaluees',
  late: 'en-retard',
  unread: 'non-consultees',
  unread_messages: 'messages-non-lus',
}

const SORT_PARAM: Record<string, ProposalSortKey> = {
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

/** « À noter par moi » : propre au panneau de la fiche, il ne passe pas par les facettes de l'API. */
const MINE_PARAM = 'moi'
const MINE_VALUE = 'a-noter'

/** Les paramètres qui décrivent une sélection, à reporter d'un dossier à l'autre et vers la liste. */
const SELECTION_KEYS = [
  'q', 'statut', 'thematique', 'format', 'pays', 'organisation', 'revisionniste', 'filtre', 'tri', 'sens', MINE_PARAM,
]

export const EMPTY_PROPOSAL_FILTERS: ProposalListFilters = {
  search: '',
  statuses: [],
  themes: [],
  formats: [],
  countries: [],
  organizations: [],
  reviewer: null,
  flags: [],
}

/** Une valeur de requête peut arriver seule ou répétée : on rend toujours une liste. */
function queryList(value: unknown): string[] {
  if (Array.isArray(value)) return value.flatMap((entry) => String(entry).split(',')).filter(Boolean)
  if (typeof value === 'string') return value.split(',').filter(Boolean)
  return []
}

function queryText(value: unknown): string {
  return typeof value === 'string' ? value : ''
}

/** La sélection courante, sans la page ni l'onglet : ce qu'un lien vers un autre dossier emporte. */
export function proposalSelectionQuery(query: LocationQuery): LocationQuery {
  return Object.fromEntries(Object.entries(query).filter(([key]) => SELECTION_KEYS.includes(key)))
}

export function useProposalListQuery() {
  const route = useRoute()
  const router = useRouter()

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

  /** Le tri par défaut : la note, décroissante. C'est la question du comité. */
  const sortKey = computed<ProposalSortKey>(
    () => SORT_PARAM[queryText(route.query.tri)] ?? 'average_score',
  )
  const sortDirection = computed<Exclude<SortDirection, null>>(() =>
    queryText(route.query.sens) === 'asc' ? 'asc' : 'desc',
  )
  const page = computed(() => Math.max(1, Number.parseInt(queryText(route.query.page) || '1', 10) || 1))
  const minePending = computed(() => queryText(route.query[MINE_PARAM]) === MINE_VALUE)

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
      // résultat qui n'en compte plus qu'une donne un tableau vide.
      page: null,
    })
  }

  function setSort(key: ProposalSortKey, direction: Exclude<SortDirection, null>): void {
    updateQuery({ tri: SORT_TO_PARAM[key], sens: direction, page: null })
  }

  function setMinePending(value: boolean): void {
    updateQuery({ [MINE_PARAM]: value ? MINE_VALUE : null })
  }

  return { filters, sortKey, sortDirection, page, minePending, updateQuery, setFilters, setSort, setMinePending }
}
