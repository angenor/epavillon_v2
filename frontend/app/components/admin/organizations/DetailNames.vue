<script setup lang="ts">
import type { OrganizationNameRow } from '~/types/admin-organizations'
import type { TableColumn } from '~/types/ui'
import type { TimeZoneName } from '~/types/shared'

/**
 * LES DÉNOMINATIONS D'UNE ORGANISATION.
 *
 * C'EST LE PANNEAU QUI PORTE LA RÈGLE MÉTIER N° 1. Chercher « IFDD » ou
 * « Institut de la Francophonie pour le développement durable » doit ramener la
 * même fiche : ce tableau montre pourquoi c'est possible — le sigle n'est pas un
 * champ à côté du nom, c'est une dénomination de plein droit, indexée comme lui.
 *
 * DEUX LIGNES NE SE MODIFIENT PAS : le nom légal et le sigle, recopiés par
 * `tg_sync_organization_names`. Elles suivent la fiche, et l'écran le dit plutôt
 * que d'offrir une action qui n'aurait aucun effet.
 *
 * « MASQUÉE » NE VEUT PAS DIRE « IGNORÉE ». Une dénomination non confirmée SERT
 * TOUJOURS la recherche — c'est ce qui permet de retrouver une fiche par une
 * faute d'orthographe connue. Elle ne s'affiche simplement pas. Laisser croire le
 * contraire ferait supprimer des variantes utiles.
 */

interface Props {
  names: OrganizationNameRow[]
  timezone: TimeZoneName
  /** L'action de confirmation est-elle ouverte ? `org.organization.manage`. */
  canManage: boolean
  busy?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ toggle: [name: OrganizationNameRow] }>()

const { t } = useI18n()
const { date } = useDateTime()

const columns = computed<TableColumn[]>(() => [
  { key: 'name', label: t('admin.organization.detail.names.columns.name') },
  { key: 'kind', label: t('admin.organization.detail.names.columns.kind') },
  { key: 'confirmed', label: t('admin.organization.detail.names.columns.confirmed') },
  {
    key: 'origin',
    label: t('admin.organization.detail.names.columns.origin'),
    hideBelow: 'lg',
  },
])
</script>

<template>
  <section>
    <h2 class="text-lg font-semibold text-text">
      {{ t('admin.organization.detail.names.title') }}
    </h2>
    <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
      {{ t('admin.organization.detail.names.description') }}
    </p>

    <UiTable
      class="mt-4"
      :columns="columns"
      :rows="props.names"
      row-key="id"
      row-label-key="name"
      :caption="t('admin.organization.detail.names.caption')"
      visually-hidden-caption
      :hoverable="false"
    >
      <template #cell-name="{ row }">
        <p class="font-medium text-text">{{ row.name }}</p>
        <p v-if="row.locale" class="font-mono text-xs text-text-subtle">{{ row.locale }}</p>
      </template>

      <template #cell-kind="{ row }">
        <UiBadge size="sm" :label="t('admin.organization.detail.names.kind.' + row.kind)" />
      </template>

      <template #cell-confirmed="{ row }">
        <div class="flex flex-wrap items-center gap-2">
          <UiBadge
            :intent="row.is_confirmed ? 'success' : 'neutral'"
            size="sm"
            :label="
              t(
                row.is_confirmed
                  ? 'admin.organization.detail.names.confirmed'
                  : 'admin.organization.detail.names.unconfirmed',
              )
            "
            :title="t('admin.organization.detail.names.confirmedHint')"
          />
          <!-- Le nom légal et le sigle suivent la fiche : aucune action ici. -->
          <UiButton
            v-if="props.canManage && !row.is_derived"
            variant="link"
            size="sm"
            :disabled="props.busy"
            @click="emit('toggle', row)"
          >
            {{
              t(
                row.is_confirmed
                  ? 'admin.organization.detail.names.unconfirm'
                  : 'admin.organization.detail.names.confirm',
              )
            }}
          </UiButton>
        </div>
      </template>

      <template #cell-origin="{ row }">
        <p v-if="row.is_derived" class="text-xs text-text-subtle" :title="t('admin.organization.detail.names.derivedHint')">
          {{ t('admin.organization.detail.names.derived') }}
        </p>
        <template v-else>
          <p class="text-sm text-text">{{ row.created_by_name ?? '—' }}</p>
          <p class="text-xs text-text-subtle">{{ date(row.created_at, props.timezone) }}</p>
        </template>
      </template>

      <template #empty>
        <UiEmptyState compact :title="t('admin.organization.detail.names.empty')" />
      </template>
    </UiTable>
  </section>
</template>
