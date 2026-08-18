<script setup lang="ts">
import type { OrganizationMemberRow } from '~/types/admin-organizations'
import type { Intent, TableColumn } from '~/types/ui'
import type { MembershipStatus } from '~/types/org'
import type { TimeZoneName } from '~/types/shared'

/**
 * LES MEMBRES D'UNE ORGANISATION.
 *
 * UNE ADHÉSION « EN ATTENTE » A DEUX ORIGINES OPPOSÉES, et ce tableau ne les
 * confond pas. `invited_at` renseigné : l'organisation a INVITÉ et attend la
 * personne. `invited_at` nul : la personne a DEMANDÉ et attend un référent. Le
 * même statut `pending` recouvre les deux, et le modèle porte la direction
 * exprès — les mélanger, c'est approuver une adhésion que l'intéressé n'a jamais
 * acceptée.
 *
 * LES DEMANDES D'ABORD. L'ordre du tableau suit ce qui se traite : ce qui attend,
 * puis ce qui vit, puis ce qui est révoqué. Un ordre alphabétique enterrerait la
 * seule ligne demandant une action au milieu de vingt autres.
 *
 * AUCUNE ACTION ICI. Approuver ou révoquer une adhésion appartient au référent de
 * l'organisation, depuis son espace (A5) : le back-office regarde, il ne se
 * substitue pas à une organisation pour décider qui en est membre.
 */

interface Props {
  members: OrganizationMemberRow[]
  timezone: TimeZoneName
}

const props = defineProps<Props>()

const { t } = useI18n()
const { date } = useDateTime()

const columns = computed<TableColumn[]>(() => [
  { key: 'person', label: t('admin.organization.detail.members.columns.person') },
  { key: 'role', label: t('admin.organization.detail.members.columns.role') },
  { key: 'status', label: t('admin.organization.detail.members.columns.status') },
  { key: 'since', label: t('admin.organization.detail.members.columns.since'), hideBelow: 'lg' },
])

/** `pending` est JAUNE : une attente demande de l'attention, ce n'est pas un échec. */
const STATUS_INTENT: Record<MembershipStatus, Intent> = {
  pending: 'warning',
  active: 'success',
  revoked: 'neutral',
}
</script>

<template>
  <section>
    <h2 class="text-lg font-semibold text-text">
      {{ t('admin.organization.detail.members.title') }}
    </h2>
    <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
      {{ t('admin.organization.detail.members.description') }}
    </p>

    <UiTable
      class="mt-4"
      :columns="columns"
      :rows="props.members"
      row-key="id"
      row-label-key="display_name"
      :caption="t('admin.organization.detail.members.caption')"
      visually-hidden-caption
      :hoverable="false"
    >
      <template #cell-person="{ row }">
        <p class="font-medium text-text">{{ row.display_name }}</p>
        <p class="text-xs text-text-muted">{{ row.primary_email }}</p>
        <p v-if="row.job_title" class="text-xs text-text-subtle">{{ row.job_title }}</p>
      </template>

      <template #cell-role="{ row }">
        <UiBadge size="sm" :label="t('admin.organization.detail.members.role.' + row.role)" />
        <p v-if="row.is_primary" class="mt-0.5 text-xs text-text-subtle">
          {{ t('admin.organization.detail.members.primary') }}
        </p>
      </template>

      <template #cell-status="{ row }">
        <UiBadge
          :intent="STATUS_INTENT[row.status]"
          size="sm"
          :label="t('admin.organization.detail.members.status.' + row.status)"
        />
        <!-- LA DIRECTION DE L'ATTENTE, en toutes lettres. Sans elle, un référent
             approuverait sa propre invitation. -->
        <p v-if="row.status === 'pending'" class="mt-0.5 text-xs text-text-muted">
          {{
            row.invited_at
              ? t('admin.organization.detail.members.invited', {
                  date: date(row.invited_at, props.timezone),
                })
              : t('admin.organization.detail.members.requested', {
                  date: date(row.created_at, props.timezone),
                })
          }}
        </p>
        <p v-else-if="row.revoked_at" class="mt-0.5 text-xs text-text-subtle">
          {{
            t('admin.organization.detail.members.revokedOn', {
              date: date(row.revoked_at, props.timezone),
            })
          }}
        </p>
      </template>

      <template #cell-since="{ row }">
        <span class="text-sm text-text">
          {{ date(row.approved_at ?? row.created_at, props.timezone) }}
        </span>
      </template>

      <template #empty>
        <UiEmptyState compact :title="t('admin.organization.detail.members.empty')" />
      </template>
    </UiTable>
  </section>
</template>
