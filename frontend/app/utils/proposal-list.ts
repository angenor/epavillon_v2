/**
 * FILTRER, TRIER, EXPORTER la liste des propositions du back-office (A7).
 *
 * Fonctions PURES : aucun accès au réseau, aucune traduction, aucun fuseau. Ce
 * qui demande une locale — le libellé d'un statut, le nom d'un pays — est fourni
 * par l'appelant sous forme de texte déjà résolu. C'est ce qui permet de trier
 * sur ce que l'utilisateur LIT, et non sur un code d'ENUM : trier la colonne
 * « Format » sur `in_person` / `online` / `hybrid` donnerait un ordre qui n'a de
 * sens dans aucune des deux langues.
 *
 * CE TRAVAIL SERA CELUI DU SERVEUR (prompt B7). Quarante lignes tiennent en
 * mémoire et un aller-retour pour trier quarante lignes serait du gaspillage ;
 * quatre cents, non. Les filtres vivent donc dans l'URL et non dans un état de
 * composant : le jour où ils partiront dans la requête, seule la source des
 * lignes changera.
 */

import type {
  BulkStatusOption,
  ProposalFilterText,
  ProposalListFilters,
  ProposalSortKey,
} from '~/types/admin-proposals'
import type { ProposalDashboardRow } from '~/types/views'
import type { ProposalStatus, ProposalTransitionRule } from '~/types/programme/proposal'
import type { EffectivePermission } from '~/types/identity'
import type { SortDirection } from '~/types/ui'
import type { Uuid } from '~/types/shared'

// ---------------------------------------------------------------------------
// Recherche
// ---------------------------------------------------------------------------

/**
 * Comparaison INSENSIBLE AUX ACCENTS ET À LA CASSE. La base s'en charge par la
 * configuration `french` de son `search_vector` ; ici, `NFD` sépare la lettre de
 * son signe diacritique, que l'on retire. Sans cela, « cotiere » ne trouverait
 * pas « côtière » — et c'est ainsi qu'on tape un titre dont on se souvient de
 * loin, sur un clavier qui n'est pas le sien.
 */
export function normalizeSearch(value: string): string {
  return value
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLowerCase()
    .trim()
}

/**
 * Les champs qu'une recherche plein texte parcourt : le numéro de dossier
 * d'abord — c'est lui qu'on colle depuis un courriel —, puis le titre français
 * porté par la vue, le nom de l'organisation et son sigle.
 */
function searchableText(row: ProposalDashboardRow): string {
  return normalizeSearch(
    [row.reference_code, row.title_text ?? '', row.organization_name, row.organization_acronym ?? ''].join(' '),
  )
}

// ---------------------------------------------------------------------------
// Filtres
// ---------------------------------------------------------------------------

/**
 * TOUS LES FILTRES SE CUMULENT PAR ET ; les valeurs d'un même filtre par OU.
 * « Statut : retenu ou en évaluation » et « Thématique : adaptation » se lit
 * naturellement ; l'inverse — deux statuts exigés à la fois — ne désigne aucun
 * dossier et ferait une liste vide sans que personne comprenne pourquoi.
 */
export function filterProposals(
  rows: ProposalDashboardRow[],
  filters: ProposalListFilters,
  unreadIds: Set<Uuid>,
): ProposalDashboardRow[] {
  const needle = normalizeSearch(filters.search)

  return rows.filter((row) => {
    if (needle && !searchableText(row).includes(needle)) return false
    if (filters.statuses.length && !filters.statuses.includes(row.status)) return false
    if (filters.formats.length && !filters.formats.includes(row.format)) return false
    if (filters.themes.length && !filters.themes.some((code) => row.theme_codes.includes(code))) return false
    if (
      filters.countries.length &&
      (row.organization_country_code === null ||
        !filters.countries.includes(row.organization_country_code))
    ) {
      return false
    }
    if (filters.organizations.length && !filters.organizations.includes(row.organization_id)) return false
    if (filters.reviewer && !row.reviewer_ids.includes(filters.reviewer)) return false

    // Les trois signaux transverses. Cumulés entre eux comme les autres filtres :
    // « non évaluées ET en retard » désigne les dossiers qu'on cherche vraiment
    // quand l'échéance approche.
    if (filters.flags.includes('unreviewed') && (row.review_count > 0 || row.status === 'draft')) return false
    if (filters.flags.includes('late') && row.overdue_reviews === 0) return false
    if (filters.flags.includes('unread') && !unreadIds.has(row.id)) return false

    return true
  })
}

// ---------------------------------------------------------------------------
// Tri
// ---------------------------------------------------------------------------

/** Comparateur de texte, respectueux des accents et de la casse françaises. */
const collator = new Intl.Collator('fr', { sensitivity: 'base', numeric: true })

/**
 * Un nombre absent se range TOUJOURS EN DERNIER, quel que soit le sens du tri.
 * Un dossier sans note n'est ni le meilleur ni le pire : il n'est pas comparable,
 * et le remonter en tête d'un tri décroissant ferait passer pour premiers ceux
 * que personne n'a encore lus. C'est le `NULLS LAST` de la vue.
 */
function compareNullableNumbers(a: number | null, b: number | null, direction: 1 | -1): number {
  if (a === null && b === null) return 0
  if (a === null) return 1
  if (b === null) return -1
  return (a - b) * direction
}

function compareNullableText(a: string | null, b: string | null, direction: 1 | -1): number {
  if (!a && !b) return 0
  if (!a) return 1
  if (!b) return -1
  return collator.compare(a, b) * direction
}

export function sortProposals(
  rows: ProposalDashboardRow[],
  key: ProposalSortKey,
  direction: Exclude<SortDirection, null>,
  text: ProposalFilterText,
): ProposalDashboardRow[] {
  const sense: 1 | -1 = direction === 'asc' ? 1 : -1

  return [...rows].sort((a, b) => {
    switch (key) {
      case 'reference_code':
        return collator.compare(a.reference_code, b.reference_code) * sense
      case 'title':
        return compareNullableText(a.title_text, b.title_text, sense)
      case 'organization':
        return collator.compare(a.organization_name, b.organization_name) * sense
      case 'country':
        return compareNullableText(text.country(a), text.country(b), sense)
      case 'format':
        return collator.compare(text.format(a), text.format(b)) * sense
      case 'status':
        return collator.compare(text.status(a), text.status(b)) * sense
      case 'reviews':
        // Sur l'AVANCEMENT, pas sur le nombre brut : deux revues sur deux est
        // terminé, deux sur cinq ne l'est pas.
        return (
          (a.review_count / Math.max(1, a.required_reviews ?? a.assigned_reviewers ?? 1) -
            b.review_count / Math.max(1, b.required_reviews ?? b.assigned_reviewers ?? 1)) *
          sense
        )
      case 'average_score':
        return compareNullableNumbers(a.average_score, b.average_score, sense)
      case 'event_rank':
        return (a.event_rank - b.event_rank) * sense
      case 'submitted_at':
        return compareNullableText(a.submitted_at, b.submitted_at, sense)
      default:
        return 0
    }
  })
}

// ---------------------------------------------------------------------------
// Actions groupées
// ---------------------------------------------------------------------------

/**
 * LES TRANSITIONS PROPOSABLES POUR UNE SÉLECTION, dérivées de
 * `programme.proposal_transitions_allowed` — la machine à états est une donnée,
 * l'écran ne la réimplémente pas.
 *
 * Une sélection est hétérogène : quatre dossiers déposés, six en évaluation.
 * Chaque transition porte donc le nombre de dossiers auxquels elle s'applique
 * VRAIMENT, et l'écran l'affiche — « Passer en évaluation (4 sur 10) ». Proposer
 * une action sans dire qu'elle n'en touchera que quatre, c'est laisser croire
 * que six dossiers ont été oubliés par erreur.
 *
 * Les transitions réservées au soumissionnaire (`allowed_for_owner` sans
 * permission requise, comme le retrait) ne sont pas offertes ici : le
 * back-office ne retire pas un dossier au nom de l'organisation.
 */
export function bulkStatusOptions(
  selection: ProposalDashboardRow[],
  rules: ProposalTransitionRule[],
  granted: EffectivePermission[] | null,
  eventId: Uuid | null,
): BulkStatusOption[] {
  const options = new Map<ProposalStatus, BulkStatusOption>()

  for (const rule of rules) {
    if (rule.required_permission === null) continue
    if (!hasPermission(granted, rule.required_permission, eventId)) continue

    const eligible = selection.filter((row) => row.status === rule.from_status).length
    const existing = options.get(rule.to_status)

    options.set(rule.to_status, {
      to_status: rule.to_status,
      // Une transition exigée avec motif l'emporte : si l'un des chemins menant
      // à cet état réclame un motif, le dialogue doit le demander.
      requires_reason: rule.requires_reason || (existing?.requires_reason ?? false),
      eligible: (existing?.eligible ?? 0) + eligible,
    })
  }

  return [...options.values()].filter((option) => option.eligible > 0)
}

// ---------------------------------------------------------------------------
// Export CSV
// ---------------------------------------------------------------------------

/** Une colonne d'export : son en-tête traduit et la valeur qu'elle tire d'une ligne. */
export interface CsvColumn {
  header: string
  value: (row: ProposalDashboardRow) => string
}

/**
 * PROTECTION CONTRE L'INJECTION DE FORMULE. Une cellule commençant par `=`, `+`,
 * `-`, `@`, une tabulation ou un retour chariot est interprétée comme une
 * formule par Excel et LibreOffice. Les titres de ce fichier sont rédigés par des
 * tiers : un dossier intitulé « =HYPERLINK(...) » deviendrait un lien actif dans
 * le tableur d'un membre du comité. On préfixe donc d'une apostrophe, qui neutralise
 * l'interprétation sans changer ce qui s'affiche.
 */
function neutralize(value: string): string {
  return /^[=+\-@\t\r]/.test(value) ? `'${value}` : value
}

function escapeCell(value: string): string {
  const safe = neutralize(value)
  return /[";\n\r]/.test(safe) ? `"${safe.replace(/"/g, '""')}"` : safe
}

/**
 * EXPORT CSV, POINT-VIRGULE ET BOM UTF-8.
 *
 * Ce fichier s'ouvre dans l'Excel francophone des équipes de l'IFDD : sans le
 * BOM, les accents y arrivent en caractères de remplacement, et avec la virgule
 * pour séparateur, tout atterrit dans une seule colonne. Ce n'est pas une
 * préférence, c'est la condition pour que le fichier serve à quelque chose.
 */
export function toCsv(rows: ProposalDashboardRow[], columns: CsvColumn[]): string {
  const lines = [
    columns.map((column) => escapeCell(column.header)).join(';'),
    ...rows.map((row) => columns.map((column) => escapeCell(column.value(row))).join(';')),
  ]
  return `\uFEFF${lines.join('\r\n')}\r\n`
}

/**
 * Nom de fichier daté, stable et triable : `propositions-COP31-2026-08-18.csv`.
 * L'instant est passé en argument — le rendu serveur et le rendu client ne
 * peuvent alors pas produire deux noms différents.
 */
export function csvFileName(prefix: string, eventLabel: string | null, at: Date): string {
  const day = at.toISOString().slice(0, 10)
  const parts = [prefix, eventLabel, day].filter((part): part is string => Boolean(part))
  return `${parts.join('-').replace(/\s+/g, '-')}.csv`
}
