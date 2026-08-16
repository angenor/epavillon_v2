<script setup lang="ts">
import type { ProposalDashboardRow } from '~/types/views'
import type { ProposalStatus } from '~/types/programme/proposal'
import type { Intent, SortDirection, TableColumn } from '~/types/ui'

/**
 * Section « Données » — le tableau du back-office, sur douze vraies lignes de
 * `programme.v_proposal_dashboard`, avec sa pagination.
 *
 * DOUZE LIGNES, PAS UNE. Un tableau de démonstration à trois lignes ne prouve
 * rien : ni la densité, ni l'alignement des nombres, ni le comportement de
 * l'en-tête collant, ni ce que devient une colonne de titres longs. Les données
 * viennent de `useApi()`, comme dans un écran réel.
 *
 * LE TRI EST CONTRÔLÉ ICI, dans la page, et pas dans le composant : c'est le
 * comportement attendu d'un écran réel, où le classement viendra du serveur.
 */

interface Props {
  /** Lignes de `v_proposal_dashboard`, déjà filtrées par périmètre. */
  rows: ProposalDashboardRow[]
  loading?: boolean
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()

/** `programme.proposal_status` → intention. Huit statuts, cinq intentions. */
const STATUS_INTENTS: Record<ProposalStatus, Intent> = {
  draft: 'neutral',
  submitted: 'info',
  under_review: 'info',
  changes_requested: 'warning',
  accepted: 'success',
  rejected: 'danger',
  withdrawn: 'neutral',
  cancelled: 'neutral',
}

const columns = computed<TableColumn[]>(() => [
  { key: 'reference_code', label: t('style-guide.data.columns.reference'), width: '9rem', sortable: true },
  { key: 'title', label: t('style-guide.data.columns.title'), sortable: true },
  { key: 'organization_name', label: t('style-guide.data.columns.organization'), hideOnMobile: true },
  { key: 'status', label: t('style-guide.data.columns.status'), width: '10rem' },
  { key: 'weighted_score', label: t('style-guide.data.columns.score'), numeric: true, sortable: true, width: '6rem', hideOnMobile: true },
  { key: 'reviews_missing', label: t('style-guide.data.columns.reviews'), numeric: true, width: '7rem', hideOnMobile: true },
  { key: 'event_rank', label: t('style-guide.data.columns.rank'), numeric: true, sortable: true, width: '5rem', hideOnMobile: true },
  { key: 'actions', label: t('style-guide.data.columns.actions'), align: 'end', width: '4rem' },
])

const sortKey = ref<string | null>('event_rank')
const sortDirection = ref<SortDirection>('asc')
const page = ref(1)
const perPage = ref(12)

function onSort(key: string, direction: Exclude<SortDirection, null>): void {
  sortKey.value = key
  sortDirection.value = direction
  page.value = 1
}

/** Tri local — dans un écran réel, il viendrait du serveur. */
const sorted = computed(() => {
  const rows = [...props.rows]
  const key = sortKey.value
  if (!key) return rows
  const factor = sortDirection.value === 'desc' ? -1 : 1
  return rows.sort((a, b) => {
    const left = key === 'title' ? (a.title_text ?? '') : a[key as keyof ProposalDashboardRow]
    const right = key === 'title' ? (b.title_text ?? '') : b[key as keyof ProposalDashboardRow]
    if (typeof left === 'number' && typeof right === 'number') return (left - right) * factor
    return String(left ?? '').localeCompare(String(right ?? ''), undefined, { numeric: true }) * factor
  })
})

const paginated = computed(() =>
  sorted.value.slice((page.value - 1) * perPage.value, page.value * perPage.value),
)

const rowMenu = computed(() => [
  { value: 'open', label: t('style-guide.data.menu.open'), icon: 'eye' },
  { value: 'review', label: t('style-guide.data.menu.review'), icon: 'edit' },
  { value: 'export', label: t('style-guide.data.menu.export'), icon: 'download', separatorBefore: true },
  { value: 'reject', label: t('style-guide.data.menu.reject'), icon: 'ban', destructive: true, separatorBefore: true },
])
</script>

<template>
  <StyleGuideSection
    id="donnees"
    :title="t('style-guide.data.title')"
    :description="t('style-guide.data.description')"
  >
    <StyleGuideDemo
      :title="t('style-guide.data.table.title')"
      :note="t('style-guide.data.table.note')"
      surface
      flush
    >
      <UiTable
        :columns="columns"
        :rows="paginated"
        row-key="id"
        :caption="t('style-guide.data.table.caption')"
        visually-hidden-caption
        :sort-key="sortKey"
        :sort-direction="sortDirection"
        :loading="props.loading"
        :loading-rows="12"
        @sort="onSort"
      >
        <template #cell-reference_code="{ row }">
          <span class="font-mono text-xs text-text-muted">{{ row.reference_code }}</span>
        </template>

        <template #cell-title="{ row }">
          <!-- `title` est le JSON brut, résolu à l'affichage. `title_text` existe
               aussi dans la vue, mais il est réservé au tri et à l'export : s'en
               servir ici interdirait à la liste de changer de langue. -->
          <span class="font-medium text-text">{{ tr(row.title) }}</span>
          <span v-if="row.is_knocked_out" class="mt-1 block">
            <UiBadge intent="danger" size="sm" icon="ban">
              {{ t('style-guide.data.knockedOut') }}
            </UiBadge>
          </span>
        </template>

        <template #cell-organization_name="{ row }">
          <span class="text-text-muted">{{ row.organization_name }}</span>
        </template>

        <template #cell-status="{ row }">
          <UiBadge :intent="STATUS_INTENTS[row.status as ProposalStatus]" size="sm">
            {{ t(`style-guide.business.status.${row.status}`) }}
          </UiBadge>
        </template>

        <template #cell-weighted_score="{ row }">
          <span v-if="row.weighted_score !== null">{{ row.weighted_score }}</span>
          <span v-else class="text-text-subtle">—</span>
        </template>

        <template #cell-reviews_missing="{ row }">
          <span v-if="row.reviews_missing === null" class="text-text-subtle">—</span>
          <span v-else-if="row.reviews_missing > 0" class="font-semibold text-warning">
            {{ row.reviews_missing }}
          </span>
          <span v-else class="text-success">{{ row.review_count }}</span>
        </template>

        <template #cell-event_rank="{ row }">{{ row.event_rank }}</template>

        <template #cell-actions="{ row }">
          <UiContextMenu
            :items="rowMenu"
            :label="t('style-guide.data.menu.label', { reference: row.reference_code })"
          />
        </template>
      </UiTable>
    </StyleGuideDemo>

    <StyleGuideDemo
      :title="t('style-guide.data.pagination.title')"
      :note="t('style-guide.data.pagination.note')"
    >
      <UiPagination
        v-model:page="page"
        v-model:per-page="perPage"
        :total="props.rows.length"
        :per-page-options="[12, 24, 48]"
      />
    </StyleGuideDemo>

    <StyleGuideDemo
      :title="t('style-guide.data.empty.title')"
      :note="t('style-guide.data.empty.note')"
      surface
      flush
    >
      <UiTable
        :columns="columns.slice(0, 4)"
        :rows="[]"
        row-key="id"
        :caption="t('style-guide.data.empty.caption')"
        visually-hidden-caption
      >
        <template #empty>
          <UiEmptyState
            filtered
            compact
            :title="t('style-guide.data.empty.stateTitle')"
            :description="t('style-guide.data.empty.stateDescription')"
            :action-label="t('style-guide.data.empty.stateAction')"
          />
        </template>
      </UiTable>
    </StyleGuideDemo>
  </StyleGuideSection>
</template>
