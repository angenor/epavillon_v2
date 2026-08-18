<script setup lang="ts">
import type { OrganizationListRow, OrganizationSortKey } from '~/types/admin-organizations'
import type { OrganizationStatus } from '~/types/org'
import type { Intent, SortDirection, TableColumn } from '~/types/ui'
import type { TimeZoneName } from '~/types/shared'

/**
 * LA LISTE DES ORGANISATIONS.
 *
 * NEUF COLONNES, ET ELLES NE DISPARAISSENT PAS DANS LE MÊME ORDRE. Ce qui
 * IDENTIFIE une fiche — nom, sigle, sceau — reste jusqu'en 375 px ; ce qui la
 * SITUE — pays, type, dernière activité — se replie. La règle du projet interdit
 * le défilement horizontal du corps de page.
 *
 * LE RATIO N'EST PAS UN POURCENTAGE COMME UN AUTRE. `ratio_acceptation` vaut
 * `null` pour une organisation qui n'a jamais rien déposé, et la cellule affiche
 * alors un tiret avec son explication — « 0 % » ferait passer une organisation
 * qui n'a jamais essayé pour une organisation qui échoue à chaque fois.
 *
 * LE SCORE DE CONFIANCE EST UNE MESURE, PAS UN JUGEMENT. Il se lit sur 100, avec
 * le détail de ce qui le compose en infobulle : sceau, domaine vérifié,
 * complétude, membres actifs. Sous le seuil, la pastille passe en jaune —
 * attention, pas échec : une fiche récente n'a rien fait de mal.
 *
 * LES DATES SONT DANS LE FUSEAU DE LA PERSONNE CONNECTÉE. Une organisation n'a
 * pas de fuseau, contrairement à une édition : « créée le 11 juin » se lit donc
 * là où se trouve celui qui regarde, et la colonne le mentionne.
 */

interface Props {
  rows: OrganizationListRow[]
  caption: string
  sortKey: OrganizationSortKey
  sortDirection: SortDirection
  /** Fuseau de lecture — celui de la personne connectée. */
  timezone: TimeZoneName
  loading?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
  sort: [key: string, direction: Exclude<SortDirection, null>]
  open: [row: OrganizationListRow]
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()

const columns = computed<TableColumn[]>(() => [
  { key: 'legal_name', label: t('admin.organization.list.columns.legal_name'), sortable: true },
  { key: 'country', label: t('admin.organization.list.columns.country'), sortable: true, hideBelow: 'lg' },
  { key: 'type', label: t('admin.organization.list.columns.type'), sortable: true, hideBelow: 'xl' },
  {
    key: 'membres_actifs',
    label: t('admin.organization.list.columns.membres_actifs'),
    sortable: true,
    numeric: true,
    align: 'end',
    hideBelow: 'lg',
  },
  {
    key: 'propositions_deposees',
    label: t('admin.organization.list.columns.propositions_deposees'),
    sortable: true,
    numeric: true,
    align: 'end',
    hideBelow: 'xl',
  },
  {
    key: 'propositions_acceptees',
    label: t('admin.organization.list.columns.propositions_acceptees'),
    sortable: true,
    numeric: true,
    align: 'end',
    hideBelow: '2xl',
  },
  {
    key: 'ratio_acceptation',
    label: t('admin.organization.list.columns.ratio_acceptation'),
    sortable: true,
    numeric: true,
    align: 'end',
  },
  {
    key: 'score_confiance',
    label: t('admin.organization.list.columns.score_confiance'),
    sortable: true,
    numeric: true,
    align: 'end',
  },
  {
    key: 'derniere_activite',
    label: t('admin.organization.list.columns.derniere_activite'),
    sortable: true,
    hideBelow: '2xl',
  },
])

/**
 * La couleur d'un état n'est pas celle qu'on croit — règle du guide de style.
 * `candidate` est JAUNE : une fiche à valider demande de l'attention, ce n'est ni
 * une réussite ni un échec. `merged` et `archived` sont GRIS, la couleur de ce
 * qui est clos — une fiche absorbée n'attend plus rien et ne demande aucune
 * action. `rejected` est rouge, seul état qui soit un refus.
 */
const STATUS_INTENT: Record<OrganizationStatus, Intent> = {
  candidate: 'warning',
  active: 'success',
  merged: 'neutral',
  archived: 'neutral',
  rejected: 'danger',
}

/** Le ratio du cadrage, en pourcentage entier. Nul quand rien n'a été déposé. */
function ratioLabel(row: OrganizationListRow): string | null {
  if (row.ratio_acceptation === null) return null
  return t('common.formats.percent', { value: Math.round(row.ratio_acceptation * 100) })
}

function trustIntent(score: number): Intent {
  return score < LOW_TRUST_SCORE ? 'warning' : 'neutral'
}
</script>

<template>
  <UiTable
    :columns="columns"
    :rows="props.rows"
    row-key="organization_id"
    row-label-key="legal_name"
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

    <template #cell-legal_name="{ row }">
      <div class="min-w-0">
        <p class="flex min-w-0 items-center gap-2">
          <span class="truncate font-semibold text-text">{{ row.legal_name }}</span>
          <UiIcon
            v-if="row.est_verifiee"
            name="check-circle"
            size="1rem"
            class="shrink-0 text-success"
            :aria-label="t('admin.organization.list.cell.verified')"
          />
        </p>
        <p class="mt-0.5 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-text-muted">
          <span v-if="row.acronym" class="font-mono">{{ row.acronym }}</span>
          <UiBadge
            :intent="STATUS_INTENT[row.statut]"
            size="sm"
            :label="t('admin.organization.list.status.' + row.statut)"
          />
          <!-- Le pont entre la liste et la file : une fiche douteuse se repère
               ici, et la file dit avec qui elle se confond. -->
          <UiBadge
            v-if="row.pending_duplicate_count > 0"
            intent="warning"
            size="sm"
            icon="copy"
            :label="t('admin.organization.list.cell.duplicate')"
            :title="t('admin.organization.list.cell.duplicateHint', row.pending_duplicate_count)"
          />
          <span v-if="row.absorbed_count > 0">
            {{ t('admin.organization.list.cell.absorbed', row.absorbed_count) }}
          </span>
        </p>
      </div>
    </template>

    <template #cell-country="{ row }">
      <template v-if="row.pays_nom">
        <p class="truncate text-sm text-text">{{ tr(row.pays_nom) }}</p>
        <p v-if="row.pays_iso3" class="font-mono text-xs text-text-subtle">{{ row.pays_iso3 }}</p>
      </template>
      <span v-else class="text-sm text-text-subtle">
        {{ t('admin.organization.list.cell.noCountry') }}
      </span>
    </template>

    <template #cell-type="{ row }">
      <UiBadge
        v-if="row.organization_type_label"
        size="sm"
        :dot-color="row.organization_type_color"
        :label="tr(row.organization_type_label)"
      />
      <span v-else class="font-mono text-xs text-text-subtle">
        {{ row.organization_type_code }}
      </span>
    </template>

    <template #cell-membres_actifs="{ row }">
      <p class="font-mono text-sm tabular-nums text-text">{{ row.membres_actifs }}</p>
      <p v-if="row.membres_en_attente > 0" class="text-xs text-warning">
        {{ t('admin.organization.list.cell.pendingMembers', { count: row.membres_en_attente }) }}
      </p>
    </template>

    <template #cell-propositions_deposees="{ row }">
      <span class="font-mono text-sm tabular-nums text-text">{{ row.propositions_deposees }}</span>
    </template>

    <template #cell-propositions_acceptees="{ row }">
      <span class="font-mono text-sm tabular-nums text-text">{{ row.propositions_acceptees }}</span>
    </template>

    <template #cell-ratio_acceptation="{ row }">
      <span
        v-if="ratioLabel(row)"
        class="font-mono text-sm tabular-nums text-text"
      >{{ ratioLabel(row) }}</span>
      <span
        v-else
        class="text-sm text-text-subtle"
        :title="t('admin.organization.list.cell.noRatioHint')"
      >{{ t('admin.organization.list.cell.noRatio') }}</span>
    </template>

    <template #cell-score_confiance="{ row }">
      <UiBadge
        :intent="trustIntent(row.score_confiance)"
        size="sm"
        :label="String(row.score_confiance)"
        :title="t('admin.organization.list.trust.detail')"
      />
    </template>

    <template #cell-derniere_activite="{ row }">
      <span v-if="row.derniere_activite" class="text-sm text-text">
        {{ date(row.derniere_activite, props.timezone) }}
      </span>
      <span v-else class="text-sm text-text-subtle">
        {{ t('admin.organization.list.cell.never') }}
      </span>
    </template>

    <template #empty>
      <slot name="empty" />
    </template>
  </UiTable>
</template>
