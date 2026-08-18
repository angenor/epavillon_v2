<script setup lang="ts">
import type { EditionListRow, EditionSortKey } from '~/types/admin-events'
import type { Intent, SortDirection, TableColumn } from '~/types/ui'
import type { EventStatus } from '~/types/event/edition'

/**
 * LA LISTE DES ÉDITIONS.
 *
 * HUIT COLONNES, ET ELLES NE DISPARAISSENT PAS DANS LE MÊME ORDRE. Ce qui sert à
 * IDENTIFIER une édition — son titre, sa série, son année — reste jusqu'en 375 px ;
 * ce qui sert à SITUER — lieu, décompte de propositions, état de programmation —
 * se replie. La règle du projet interdit le défilement horizontal du corps de
 * page : on retire des colonnes, on ne laisse pas filer le tableau.
 *
 * LES DATES SONT DANS LE FUSEAU DE L'ÉDITION, chacune dans le sien. C'est la
 * particularité de cet écran : la liste des propositions affiche tout dans le
 * fuseau d'UNE édition, ici chaque ligne porte le sien. Afficher la COP31 en heure
 * de Belém et le cycle PACO en heure de Paris n'est pas une incohérence, c'est la
 * seule lecture juste — et chaque cellule le mentionne.
 */

interface Props {
  rows: EditionListRow[]
  caption: string
  sortKey: EditionSortKey
  sortDirection: SortDirection
  loading?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
  sort: [key: string, direction: Exclude<SortDirection, null>]
  open: [row: EditionListRow]
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()

const columns = computed<TableColumn[]>(() => [
  { key: 'title', label: t('admin.event.list.columns.title'), sortable: true },
  { key: 'series', label: t('admin.event.list.columns.series'), sortable: true, hideBelow: 'lg' },
  {
    key: 'edition_year',
    label: t('admin.event.list.columns.edition_year'),
    sortable: true,
    numeric: true,
    align: 'end',
    width: '5rem',
  },
  { key: 'starts_at', label: t('admin.event.list.columns.starts_at'), sortable: true },
  { key: 'location', label: t('admin.event.list.columns.location'), sortable: true, hideBelow: 'xl' },
  { key: 'status', label: t('admin.event.list.columns.status'), sortable: true },
  {
    key: 'proposal_count',
    label: t('admin.event.list.columns.proposal_count'),
    sortable: true,
    numeric: true,
    align: 'end',
    hideBelow: 'lg',
  },
  {
    key: 'programme',
    label: t('admin.event.list.columns.programme'),
    sortable: true,
    hideBelow: 'xl',
  },
])

/**
 * La couleur d'un état n'est pas celle qu'on croit — règle du guide de style.
 * `ongoing` est JAUNE et non vert : « en cours » demande de l'attention, ce n'est
 * pas une réussite. `completed` est gris, ce qui est clos ; `suspended` jaune, ce
 * qui attend un arbitrage ; `cancelled` rouge.
 */
const STATUS_INTENT: Record<EventStatus, Intent> = {
  draft: 'neutral',
  announced: 'info',
  ongoing: 'warning',
  completed: 'neutral',
  cancelled: 'danger',
  suspended: 'warning',
}

/** « du 9 au 20 novembre 2027 », dans le fuseau de l'édition. */
function periodOf(row: EditionListRow): string {
  return t('common.datetime.dateRange', {
    start: date(row.starts_at, row.timezone),
    end: date(row.ends_at, row.timezone),
  })
}
</script>

<template>
  <UiTable
    :columns="columns"
    :rows="props.rows"
    row-key="id"
    row-label-key="slug"
    :caption="props.caption"
    :sort-key="props.sortKey"
    :sort-direction="props.sortDirection"
    :loading="props.loading"
    sticky-header
    @sort="(key, direction) => emit('sort', key, direction)"
    @row-click="(row) => emit('open', row)"
  >
    <template #toolbar>
      <slot name="toolbar" />
    </template>

    <template #cell-title="{ row }">
      <div class="min-w-0">
        <p class="truncate font-semibold text-text">{{ tr(row.title) }}</p>
        <p class="mt-0.5 flex flex-wrap items-center gap-x-2 text-xs text-text-muted">
          <span v-if="row.edition_label" class="font-mono">{{ row.edition_label }}</span>
          <span v-if="row.acronym" class="font-mono">{{ row.acronym }}</span>
          <span>{{ t('admin.event.list.cell.days', row.day_count) }}</span>
        </p>
      </div>
    </template>

    <template #cell-series="{ row }">
      <template v-if="row.series_name">
        <p class="truncate text-sm text-text">{{ tr(row.series_name) }}</p>
        <p v-if="row.series_kind" class="text-xs text-text-subtle">
          {{ t('admin.event.list.seriesKind.' + row.series_kind) }}
        </p>
      </template>
      <span v-else class="text-sm text-text-subtle">{{ t('admin.event.list.cell.noSeries') }}</span>
    </template>

    <template #cell-edition_year="{ row }">{{ row.edition_year }}</template>

    <template #cell-starts_at="{ row }">
      <p class="text-sm text-text">{{ periodOf(row) }}</p>
      <p class="text-xs text-text-subtle">
        {{ t('common.datetime.zoneOf', { zone: row.city ?? row.timezone }) }}
      </p>
    </template>

    <template #cell-location="{ row }">
      <template v-if="row.city">
        <p class="truncate text-sm text-text">{{ row.city }}</p>
        <p v-if="row.country_name" class="text-xs text-text-subtle">{{ tr(row.country_name) }}</p>
      </template>
      <span v-else class="text-sm text-text-subtle">{{ t('admin.event.list.cell.online') }}</span>
    </template>

    <template #cell-status="{ row }">
      <div class="flex flex-col items-start gap-1">
        <UiBadge
          :intent="STATUS_INTENT[row.status]"
          :label="t('admin.event.list.status.' + row.status)"
          size="sm"
        />
        <!-- « Sans pavillon » n'est pas un manque à combler : c'est une COP où
             l'IFDD n'envoie qu'un représentant, et donc sans appel. -->
        <span class="text-xs text-text-subtle">
          {{ t(row.has_pavilion ? 'admin.event.list.cell.pavilion' : 'admin.event.list.cell.noPavilion') }}
        </span>
      </div>
    </template>

    <template #cell-proposal_count="{ row }">
      <template v-if="row.call_status">
        <p class="font-mono text-sm tabular-nums text-text">{{ row.proposal_count }}</p>
        <p class="text-xs text-text-subtle">
          {{ t('admin.event.list.callStatus.' + row.call_status) }}
        </p>
      </template>
      <span v-else class="text-xs text-text-subtle">{{ t('admin.event.list.cell.noCall') }}</span>
    </template>

    <template #cell-programme="{ row }">
      <template v-if="row.programme_published_at">
        <UiBadge
          intent="success"
          size="sm"
          :label="t('admin.event.list.programme.published', {
            date: date(row.programme_published_at, row.timezone),
          })"
        />
      </template>
      <UiBadge
        v-else
        intent="neutral"
        size="sm"
        :label="t('admin.event.list.programme.unpublished')"
      />
      <p class="mt-0.5 text-xs text-text-subtle">
        {{ t('admin.event.list.cell.sessions', {
          scheduled: row.scheduled_session_count,
          total: row.session_count,
        }) }}
      </p>
    </template>

    <template #empty>
      <slot name="empty" />
    </template>
  </UiTable>
</template>
