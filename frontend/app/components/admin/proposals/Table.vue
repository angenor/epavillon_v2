<script setup lang="ts">
import type { ProposalDashboardRow } from '~/types/views'
import type { ProposalSortKey } from '~/types/admin-proposals'
import type { SortDirection, TableColumn } from '~/types/ui'
import type { TimeZoneName, Uuid } from '~/types/shared'

/**
 * LE TABLEAU DES PROPOSITIONS — onze colonnes, quarante lignes, et la densité
 * est le sujet.
 *
 * CE QUE CHAQUE CELLULE DOIT DIRE SANS SURVOL :
 *
 *  · DOSSIER — le numéro, en chasse fixe : c'est la clé qu'on colle depuis un
 *    courriel, elle doit s'aligner d'une ligne à l'autre. Le point d'appel des
 *    dossiers NON CONSULTÉS est ici, discret : un liseré et une puce, pas une
 *    pastille de couleur. « Je ne l'ai pas encore ouvert » n'est pas une alerte.
 *  · ORGANISATION — le porteur, et « +2 » quand le dossier est co-organisé. La
 *    co-organisation n'est pas un détail : elle change qui répond du dossier.
 *  · REVUES — l'avancement (« 2/3 ») et, sous lui, ce qui manque : de qui on
 *    attend la revue, ou combien sont en retard. Un « 2/3 » seul ne dit pas s'il
 *    faut relancer quelqu'un.
 *  · NOTE et RANG — vides tant que rien n'est noté, et non « 0 » ni « — » muet :
 *    un dossier non noté n'a pas la note zéro.
 *
 * LE TRI EST CONTRÔLÉ PAR LA PAGE, pas par ce composant : il émet `sort` et
 * reçoit l'état. C'est ce que fait déjà `UiTable`, et c'est ce qui permettra au
 * tri de partir au serveur (B7) sans toucher à ce fichier.
 *
 * PAS DE DÉFILEMENT HORIZONTAL DE LA PAGE, ET TROIS PALIERS. Les onze colonnes
 * demandées ne tiennent pas ensemble dans les 1 130 px que laisse la navigation
 * latérale sur un portable : plutôt que de les faire défiler, on retire celles
 * qui SITUENT un dossier avant celles qui servent à DÉCIDER. Pays et format ne
 * reviennent qu'à partir de 1 536 px, les thématiques à 1 280 px ; sous 640 px il
 * ne reste que le numéro, le titre et le statut — le minimum pour reconnaître un
 * dossier et savoir où il en est. Ce qui dépasse malgré tout défile DANS le
 * cadre du tableau, jamais dans la page.
 */

interface Props {
  rows: ProposalDashboardRow[]
  /** Dossiers jamais ouverts par la personne connectée. */
  unreadIds: Set<Uuid>
  /** Fuseau de l'édition : toute date affichée le porte. La mention du fuseau
   *  est portée par la légende du tableau, que la page compose. */
  timezone: TimeZoneName
  /** `calls_for_proposals.required_reviews` — le dénominateur du « 2/3 ». */
  requiredReviews: number | null
  caption: string
  sortKey: ProposalSortKey
  sortDirection: Exclude<SortDirection, null>
  selected: string[]
  loading?: boolean
  /** La sélection multiple est-elle offerte ? Non si aucune action n'est permise. */
  selectable?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
  sort: [key: ProposalSortKey, direction: Exclude<SortDirection, null>]
  'update:selected': [keys: string[]]
  open: [row: ProposalDashboardRow]
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()

/**
 * LES LARGEURS SONT DÉCLARÉES, pas laissées au contenu. Sans elles, la colonne
 * des thématiques — dont les libellés viennent de la base et vont jusqu'à
 * « Justice climatique et peuples autochtones » — prend le quart du tableau et
 * écrase le titre sur six lignes. Ce qui compte pour décider (titre, note, rang)
 * doit tenir sans défiler ; le reste s'accommode.
 */
const columns = computed<TableColumn[]>(() => [
  { key: 'reference_code', label: t('admin.proposals.columns.reference'), sortable: true, width: '7.5rem' },
  { key: 'title', label: t('admin.proposals.columns.title'), sortable: true, width: '24rem' },
  { key: 'organization', label: t('admin.proposals.columns.organization'), sortable: true, width: '9rem', hideOnMobile: true },
  { key: 'country', label: t('admin.proposals.columns.country'), sortable: true, width: '7rem', hideBelow: '2xl' },
  { key: 'themes', label: t('admin.proposals.columns.themes'), width: '10rem', hideBelow: 'xl' },
  { key: 'format', label: t('admin.proposals.columns.format'), sortable: true, width: '5.5rem', hideBelow: '2xl' },
  { key: 'status', label: t('admin.proposals.columns.status'), sortable: true, width: '7.5rem' },
  { key: 'reviews', label: t('admin.proposals.columns.reviews'), sortable: true, width: '7rem', hideOnMobile: true },
  { key: 'average_score', label: t('admin.proposals.columns.score'), sortable: true, numeric: true, width: '4.5rem', hideOnMobile: true },
  { key: 'event_rank', label: t('admin.proposals.columns.rank'), sortable: true, numeric: true, width: '4rem', hideOnMobile: true },
  { key: 'submitted_at', label: t('admin.proposals.columns.submitted'), sortable: true, width: '7.5rem', hideOnMobile: true },
])

/**
 * LA COULEUR D'UN STATUT SUIT LA RÈGLE D'USAGE, et deux choix ne vont pas de
 * soi : « en évaluation » est JAUNE — c'est en cours, cela demande de
 * l'attention, ce n'est pas une réussite —, et « annulé » est VIOLET, comme le
 * report : la décision est prise, elle n'attend plus rien. Le gris est réservé à
 * ce qui est clos sans décision du comité (brouillon, retiré).
 */
const STATUS_TONE: Record<ProposalDashboardRow['status'], string> = {
  draft: 'text-neutral bg-neutral-surface',
  submitted: 'text-info bg-info-surface',
  under_review: 'text-warning bg-warning-surface',
  changes_requested: 'text-warning bg-warning-surface',
  accepted: 'text-success bg-success-surface',
  rejected: 'text-danger bg-danger-surface',
  withdrawn: 'text-neutral bg-neutral-surface',
  cancelled: 'text-postponed bg-postponed-surface',
}

/**
 * Le dénominateur du « 2/3 » : ce que l'APPEL exige (`required_reviews`), à
 * défaut ce qui a été confié. Un dossier hors appel — programmation directe de
 * l'IFDD — n'a pas d'exigence : son avancement se lit alors sur ses
 * affectations, et sur rien si personne ne lui en a donné.
 */
function expectedReviews(row: ProposalDashboardRow): number {
  return row.required_reviews ?? props.requiredReviews ?? row.assigned_reviewers
}

/** Ceux dont on attend encore la revue — c'est eux qu'on relance. */
function pendingReviewers(row: ProposalDashboardRow): string {
  return row.reviewers
    .filter((reviewer) => reviewer.submitted_at === null)
    .map((reviewer) => reviewer.name)
    .join(', ')
}

function onSort(key: string, direction: Exclude<SortDirection, null>): void {
  emit('sort', key as ProposalSortKey, direction)
}
</script>

<template>
  <UiTable
    :columns="columns"
    :rows="props.rows"
    row-key="id"
    row-label-key="reference_code"
    :caption="props.caption"
    visually-hidden-caption
    :sort-key="props.sortKey"
    :sort-direction="props.sortDirection"
    :loading="props.loading"
    :loading-rows="10"
    :selectable="props.selectable"
    :selected="props.selected"
    dense
    sticky-header
    @sort="onSort"
    @row-click="(row: ProposalDashboardRow) => emit('open', row)"
    @update:selected="(keys: string[]) => emit('update:selected', keys)"
  >
    <template #toolbar>
      <slot name="toolbar" />
    </template>

    <template #empty>
      <slot name="empty" />
    </template>

    <!-- NUMÉRO DE DOSSIER — chasse fixe pour que les numéros s'alignent, et le
         repère « non consulté » posé ici : un liseré à gauche de la cellule et
         une puce. Discret par construction, comme le demande le prompt. -->
    <template #cell-reference_code="{ row }">
      <div class="flex items-center gap-2">
        <span
          v-if="props.unreadIds.has(row.id)"
          class="size-1.5 shrink-0 rounded-full bg-accent-solid"
          :title="t('admin.proposals.row.unread')"
        >
          <span class="sr-only">{{ t('admin.proposals.row.unread') }}</span>
        </span>
        <span v-else class="size-1.5 shrink-0" aria-hidden="true" />
        <span
          class="font-mono text-xs whitespace-nowrap"
          :class="props.unreadIds.has(row.id) ? 'font-semibold text-text' : 'text-text-secondary'"
        >
          {{ row.reference_code }}
        </span>
      </div>
    </template>

    <!-- TITRE — la donnée multilingue de la base, résolue à l'affichage. Sur
         deux lignes au plus : un titre de proposition fait couramment cent
         vingt caractères, et le laisser filer casse la densité de la ligne. -->
    <template #cell-title="{ row }">
      <span class="clamp-2 font-medium text-text" :title="tr(row.title)">{{ tr(row.title) }}</span>
      <span
        v-if="row.open_change_requests > 0"
        class="mt-1 block text-xs text-warning"
      >
        {{ t('admin.proposals.row.changeRequests', row.open_change_requests) }}
      </span>
    </template>

    <template #cell-organization="{ row }">
      <div class="flex items-center gap-1.5">
        <span class="clamp-2 text-text-secondary">
          {{ row.organization_acronym || row.organization_name }}
        </span>
        <!-- « +2 » : la co-organisation, qui n'existait nulle part en v1 et
             finissait dans le texte de présentation. -->
        <span
          v-if="row.co_organizer_count > 0"
          class="inline-flex shrink-0 items-center rounded-full border border-border bg-surface-sunken px-1.5 py-0.5 text-[0.6875rem] font-bold whitespace-nowrap text-text-secondary"
          :title="t('admin.proposals.row.coOrganizers', row.co_organizer_count)"
        >
          +{{ row.co_organizer_count }}
          <span class="sr-only"> {{ t('admin.proposals.row.coOrganizers', row.co_organizer_count) }}</span>
        </span>
      </div>
    </template>

    <template #cell-country="{ row }">
      <span class="text-text-secondary">
        {{ row.organization_country ? tr(row.organization_country) : t('common.labels.unknown') }}
      </span>
    </template>

    <!-- TROIS PASTILLES AU PLUS, les suivantes en « +N » — règle du guide de
         style, portée par le composant : au-delà de trois, elles cessent
         d'informer. -->
    <template #cell-themes="{ row }">
      <!-- `whitespace-normal` forcé sur les pastilles : leurs libellés viennent
           de la base et vont jusqu'à « Justice climatique et peuples autochtones ».
           Insécables, ils imposaient à eux seuls le quart de la largeur du
           tableau et écrasaient le titre. Le guide de style exige le libellé
           COMPLET, jamais une abréviation : on le replie donc plutôt que de le
           couper. -->
      <div class="theme-cell">
        <UiThemeTagList :themes="row.themes" :max="2" size="sm" />
      </div>
    </template>

    <template #cell-format="{ row }">
      <span class="text-text-secondary whitespace-nowrap">
        {{ t(`admin.proposals.format.${row.format}`) }}
      </span>
    </template>

    <template #cell-status="{ row }">
      <span
        class="inline-flex items-center rounded-sm px-2 py-1 text-[0.6875rem] font-semibold tracking-caps text-balance uppercase"
        :class="STATUS_TONE[row.status]"
      >
        {{ t(`admin.proposals.status.${row.status}`) }}
      </span>
      <span v-if="row.is_knocked_out" class="mt-1 block text-xs text-danger">
        {{ t('admin.proposals.row.knockedOut') }}
      </span>
    </template>

    <!-- AVANCEMENT DES REVUES : le rapport, puis ce qui manque. Un « 2/3 » seul
         ne dit pas s'il faut relancer, ni qui. -->
    <template #cell-reviews="{ row }">
      <span class="font-mono text-sm tabular-nums">
        {{ row.review_count }}/{{ expectedReviews(row) }}
        <span class="sr-only">
          {{ t('admin.proposals.row.reviewProgress', { done: row.review_count, expected: expectedReviews(row) }) }}
        </span>
      </span>
      <span v-if="row.overdue_reviews > 0" class="mt-0.5 block text-xs text-warning">
        {{ t('admin.proposals.row.late', row.overdue_reviews) }}
      </span>
      <!-- « Aucun révisionniste affecté » ne se dit QUE si rien n'est rendu non
           plus : un dossier peut porter trois revues sans qu'aucune affectation
           n'ait été enregistrée — l'affectation organise le travail, elle ne le
           conditionne pas. L'annoncer sur un dossier complet ferait courir après
           un problème qui n'existe pas. -->
      <span
        v-else-if="row.assigned_reviewers === 0 && row.review_count === 0 && row.status !== 'draft'"
        class="mt-0.5 block text-xs text-text-subtle"
      >
        {{ t('admin.proposals.row.reviewersNone') }}
      </span>
      <span
        v-else-if="pendingReviewers(row)"
        class="clamp-1 mt-0.5 block text-xs text-text-subtle"
        :title="pendingReviewers(row)"
      >
        {{ t('admin.proposals.row.reviewersPending', { names: pendingReviewers(row) }) }}
      </span>
    </template>

    <!-- NOTE — sur 20, l'échelle familière des équipes depuis la v1. Vide tant
         qu'aucune revue n'est rendue : un dossier non noté n'a pas zéro. -->
    <template #cell-average_score="{ row }">
      <span v-if="row.average_score !== null" class="font-semibold">
        {{ row.average_score.toFixed(1) }}
      </span>
      <span v-else class="text-text-subtle" :title="t('admin.proposals.row.noScore')">—</span>
    </template>

    <template #cell-event_rank="{ row }">
      <span v-if="row.average_score !== null" class="text-text-secondary">{{ row.event_rank }}</span>
      <span v-else class="text-text-subtle">—</span>
    </template>

    <!-- TOUTE DATE PORTE SON FUSEAU : celui de l'édition, jamais celui du
         navigateur. La colonne est étroite, la mention de fuseau est donc dans
         l'infobulle et dans la légende du tableau. -->
    <template #cell-submitted_at="{ row }">
      <span v-if="row.submitted_at" class="whitespace-nowrap text-text-secondary">
        {{ date(row.submitted_at, props.timezone) }}
      </span>
      <span v-else class="text-text-subtle">{{ t('admin.proposals.row.notSubmitted') }}</span>
    </template>
  </UiTable>
</template>

<style scoped>
/* TRONCATURE À DEUX LIGNES, écrite à la main : les utilitaires `line-clamp-*`
   de Tailwind ne sont pas générés dans ce projet — la classe existait dans le
   balisage sans aucune règle derrière, et les titres s'étalaient sur six lignes.
   Même approche que `ProgrammeCalendar`, qui portait déjà son clamp en CSS. */
.clamp-1,
.clamp-2 {
  display: -webkit-box;
  overflow: hidden;
  -webkit-box-orient: vertical;
}

.clamp-1 {
  -webkit-line-clamp: 1;
}

.clamp-2 {
  -webkit-line-clamp: 2;
}

/* Les pastilles thématiques sont insécables par dessin — c'est juste sur une
   carte de programmation, pas dans une colonne de tableau : un seul libellé
   long y imposait le quart de la largeur totale. On les replie ICI, sans
   toucher au composant partagé, et sans jamais abréger le libellé. */
.theme-cell :deep(span) {
  white-space: normal;
}
</style>
