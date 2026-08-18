<script setup lang="ts">
import type { ManagedIncident } from '~/types/admin-incidents'
import type { TableColumn } from '~/types/ui'
import type { TimeZoneName } from '~/types/shared'
import type { TaxonomyTerm } from '~/types/reference'

/**
 * LA LISTE DES MESSAGES D'INCIDENT.
 *
 * ELLE N'EST PAS TRIABLE, ET C'EST VOULU. Son ordre n'est pas une préférence
 * d'affichage : c'est l'ordre dans lequel l'équipe agit — ce qui parle en ce
 * moment, ce qui va parler, ce qui attend une décision, puis l'historique.
 * `live.event_incidents()` le rend déjà ainsi. Offrir un tri par date ferait
 * remonter un brouillon devant une panne en cours, pendant une COP.
 *
 * LA DÉPUBLICATION EST DANS LA LIGNE, PAS DANS UN MENU. C'est le geste pressé de
 * cet écran : la panne est réparée, le bandeau rouge doit disparaître du site
 * maintenant. Le reste — modifier, reprendre — passe par le menu d'actions.
 *
 * CINQ COLONNES, ET ELLES NE DISPARAISSENT PAS DANS LE MÊME ORDRE. Ce qui dit
 * QUOI et OÙ — le message, son état — tient jusqu'en 375 px ; la nature et la
 * fenêtre d'affichage se replient, et le repli les réaffiche sous le message
 * plutôt que de les perdre.
 *
 * LES DATES SONT DANS LE FUSEAU DE L'ÉDITION : une fenêtre d'affichage se lit là
 * où l'incident a lieu, jamais dans le fuseau du navigateur de qui publie.
 */

interface Props {
  rows: ManagedIncident[]
  caption: string
  timezone: TimeZoneName
  /** Termes de `incident_kind`, pour afficher un libellé plutôt qu'un code. */
  kinds: TaxonomyTerm[]
  /** `live.incident.publish` sur cette édition. */
  canPublish?: boolean
  loading?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
  open: [row: ManagedIncident]
  publish: [row: ManagedIncident]
  unpublish: [row: ManagedIncident]
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dateTime } = useDateTime()

const columns = computed<TableColumn[]>(() => [
  { key: 'message', label: t('admin.incident.list.columns.message') },
  { key: 'scope', label: t('admin.incident.list.columns.scope'), hideBelow: 'lg', width: '14rem' },
  { key: 'kind', label: t('admin.incident.list.columns.kind'), hideBelow: 'xl', width: '12rem' },
  { key: 'window', label: t('admin.incident.list.columns.window'), hideBelow: 'lg', width: '16rem' },
  { key: 'state', label: t('admin.incident.list.columns.state'), width: '9rem' },
  { key: 'actions', label: t('admin.incident.list.columns.actions'), align: 'end', width: '10rem' },
])

const SEVERITY_INTENT = {
  info: 'info',
  warning: 'warning',
  error: 'danger',
  critical: 'danger',
} as const

/** Le libellé d'une nature vient de la BASE — jamais d'un fichier i18n. */
function kindLabel(code: string): string {
  const term = props.kinds.find((entry) => entry.code === code)
  return term ? tr(term.label) : code
}

/**
 * La cible résolue — et RIEN pour un message global, qui n'en a pas.
 *
 * `live.event_incidents()` rend `target_label` nul dans ce cas ; répéter
 * « toute la plateforme » sous le libellé de portée, qui dit déjà exactement
 * cela, ferait deux fois la même phrase dans la même cellule.
 */
function scopeTarget(row: ManagedIncident): string | null {
  return row.target_label
}

/** Un message publié et non retiré peut être dépublié — les deux autres, non. */
function canUnpublish(row: ManagedIncident): boolean {
  return row.published_at !== null && row.unpublished_at === null
}
</script>

<template>
  <UiTable
    :columns="columns"
    :rows="rows"
    row-key="incident_id"
    :caption="caption"
    :loading="loading"
    sticky-header
    @row-click="(row) => emit('open', row)"
  >
    <template #cell-message="{ row }">
      <div class="min-w-0">
        <div class="flex flex-wrap items-center gap-2">
          <UiBadge
            :intent="SEVERITY_INTENT[row.severity]"
            size="sm"
            :label="t(`admin.incident.form.severity.option.${row.severity}`)"
          />
          <p v-if="row.title" class="truncate font-medium">{{ tr(row.title) }}</p>
          <p v-else class="truncate text-text-subtle italic">
            {{ t('admin.incident.list.row.noTitle') }}
          </p>
        </div>

        <p class="mt-1 line-clamp-2 max-w-(--measure) text-sm text-text-muted">{{ tr(row.message) }}</p>

        <!-- REPLI DES COLONNES MASQUÉES : sur écran étroit, la portée et la
             fenêtre ne disparaissent pas, elles descendent sous le message. -->
        <p class="mt-1 text-sm text-text-subtle lg:hidden">
          {{ t(`incident-banner.scope.${row.scope}`) }}
          <template v-if="scopeTarget(row)"> · {{ scopeTarget(row) }}</template>
        </p>

        <p v-if="row.unpublish_reason" class="mt-1 text-sm text-text-subtle">
          {{ t('admin.incident.list.row.unpublishReason', { reason: row.unpublish_reason }) }}
        </p>
      </div>
    </template>

    <template #cell-scope="{ row }">
      <p class="text-sm">{{ t(`incident-banner.scope.${row.scope}`) }}</p>
      <p v-if="scopeTarget(row)" class="truncate text-sm text-text-muted">{{ scopeTarget(row) }}</p>
    </template>

    <template #cell-kind="{ row }">
      <span class="text-sm">{{ kindLabel(row.kind_code) }}</span>
    </template>

    <template #cell-window="{ row }">
      <p class="text-sm">{{ t('admin.incident.list.row.from', { date: dateTime(row.display_from, timezone) }) }}</p>
      <p v-if="row.display_until" class="text-sm text-text-muted">
        {{ t('admin.incident.list.row.until', { date: dateTime(row.display_until, timezone) }) }}
      </p>
      <!-- Sans fin programmée, quelqu'un devra y penser : c'est le défaut de la
           v1, et il se signale ici plutôt qu'après coup. -->
      <p v-else class="flex items-center gap-1 text-sm text-warning">
        <UiIcon name="warning" size="0.85rem" />
        {{ t('admin.incident.list.row.openEnded') }}
      </p>
    </template>

    <template #cell-state="{ row }">
      <div class="min-w-0">
        <AdminIncidentsStateBadge :state="row.state" size="sm" />
        <p v-if="row.published_by_name" class="mt-1 truncate text-xs text-text-subtle">
          {{ row.published_by_name }}
        </p>
      </div>
    </template>

    <template #cell-actions="{ row }">
      <div class="flex items-center justify-end gap-1">
        <UiButton
          v-if="canPublish && canUnpublish(row)"
          variant="secondary"
          size="sm"
          icon="eye-off"
          @click.stop="emit('unpublish', row)"
        >
          {{ t('admin.incident.list.actions.unpublish') }}
        </UiButton>
        <UiButton
          v-else-if="canPublish && row.state === 'draft'"
          variant="secondary"
          size="sm"
          icon="broadcast"
          @click.stop="emit('publish', row)"
        >
          {{ t('admin.incident.list.actions.publish') }}
        </UiButton>
        <UiButton
          v-else-if="canPublish && row.state === 'unpublished'"
          variant="ghost"
          size="sm"
          icon="refresh"
          @click.stop="emit('publish', row)"
        >
          {{ t('admin.incident.list.actions.republish') }}
        </UiButton>

        <UiButton
          variant="ghost"
          size="sm"
          icon="edit"
          icon-only
          :label="t('admin.incident.list.actions.edit')"
          @click.stop="emit('open', row)"
        />
      </div>
    </template>

    <template #empty>
      <slot name="empty" />
    </template>
  </UiTable>
</template>
