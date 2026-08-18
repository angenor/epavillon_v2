<script setup lang="ts">
import type { OrganizationActivityRow } from '~/types/admin-organizations'
import type { TableColumn } from '~/types/ui'
import type { TimeZoneName } from '~/types/shared'

/**
 * LES ACTIVITÉS D'UNE ORGANISATION — dossiers déposés et activités programmées.
 *
 * LE RÔLE COMPTE AUTANT QUE L'ACTIVITÉ. Une organisation qui figure douze fois
 * comme soutien n'a pas le même parcours qu'une organisation qui a porté douze
 * dossiers. C'est `programme.proposal_organizations.role` qui les distingue, et
 * la v1 en était incapable, faute d'avoir la table : « impossible de dire combien
 * d'activités une organisation avait réellement portées ».
 *
 * DEUX NATURES DANS UN SEUL TABLEAU, et c'est délibéré : le dossier et la séance
 * qui en découle racontent la même histoire, et les séparer en deux panneaux
 * obligerait à faire le rapprochement de tête. Une pastille dit laquelle est
 * laquelle.
 *
 * LE STATUT D'UN DOSSIER N'EST PAS TRADUIT ICI : `admin.proposals.status.*`
 * existe depuis A7, et le recopier ferait deux libellés pour un même état, qui
 * divergeraient au premier changement. Celui d'une SÉANCE, lui, n'avait aucun
 * bloc canonique — le planificateur nomme des actions, pas des états —, il est
 * donc déclaré dans le fichier de cet écran. Le jour où un autre écran en a
 * besoin, ce bloc remontera dans `_common.json` : c'est le sens du déplacement,
 * pas l'inverse.
 */

interface Props {
  activities: OrganizationActivityRow[]
  timezone: TimeZoneName
}

const props = defineProps<Props>()

const { t, te } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()

const columns = computed<TableColumn[]>(() => [
  { key: 'title', label: t('admin.organization.detail.activities.columns.title') },
  { key: 'event', label: t('admin.organization.detail.activities.columns.event'), hideBelow: 'lg' },
  { key: 'role', label: t('admin.organization.detail.activities.columns.role') },
  { key: 'status', label: t('admin.organization.detail.activities.columns.status') },
  { key: 'date', label: t('admin.organization.detail.activities.columns.date'), hideBelow: 'xl' },
])

/**
 * Libellé d'un statut. À défaut, le code brut : un libellé manquant se voit, et
 * vaut mieux qu'un état inventé.
 */
function statusLabel(row: OrganizationActivityRow): string {
  const key =
    row.kind === 'proposal'
      ? `admin.proposals.status.${row.status}`
      : `admin.organization.detail.activities.sessionStatus.${row.status}`
  return te(key) ? t(key) : row.status
}
</script>

<template>
  <section>
    <h2 class="text-lg font-semibold text-text">
      {{ t('admin.organization.detail.activities.title') }}
    </h2>
    <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
      {{ t('admin.organization.detail.activities.description') }}
    </p>

    <UiTable
      class="mt-4"
      :columns="columns"
      :rows="props.activities"
      row-key="id"
      :caption="t('admin.organization.detail.activities.caption')"
      visually-hidden-caption
      :hoverable="false"
    >
      <template #cell-title="{ row }">
        <p class="font-medium text-text">{{ tr(row.title) }}</p>
        <p class="mt-0.5 flex flex-wrap items-center gap-x-2 text-xs text-text-muted">
          <UiBadge
            size="sm"
            :intent="row.kind === 'session' ? 'info' : 'neutral'"
            :label="t('admin.organization.detail.activities.kind.' + row.kind)"
          />
          <span v-if="row.reference_code" class="font-mono">{{ row.reference_code }}</span>
        </p>
      </template>

      <template #cell-event="{ row }">
        <p class="truncate text-sm text-text">{{ tr(row.event_name) }}</p>
        <p class="font-mono text-xs text-text-subtle">{{ row.edition_year }}</p>
      </template>

      <template #cell-role="{ row }">
        <UiBadge
          size="sm"
          :intent="row.role === 'lead' ? 'info' : 'neutral'"
          :label="t('admin.organization.detail.activities.role.' + row.role)"
        />
      </template>

      <template #cell-status="{ row }">
        <span class="text-sm text-text">{{ statusLabel(row) }}</span>
      </template>

      <template #cell-date="{ row }">
        <span v-if="row.occurred_at" class="text-sm text-text">
          {{ date(row.occurred_at, props.timezone) }}
        </span>
        <span v-else class="text-sm text-text-subtle">—</span>
      </template>

      <template #empty>
        <UiEmptyState compact :title="t('admin.organization.detail.activities.empty')" />
      </template>
    </UiTable>
  </section>
</template>
