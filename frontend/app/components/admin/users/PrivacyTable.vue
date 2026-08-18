<script setup lang="ts">
import type { PrivacyRequestView } from '~/types/admin-users'
import type { TableColumn } from '~/types/ui'
import type { TimeZoneName } from '~/types/shared'

/**
 * LA FILE DES DEMANDES RGPD.
 *
 * L'ÉCHÉANCE EST LA COLONNE QUI COMMANDE TOUT. Une demande se traite en trente
 * jours — `privacy_requests.due_at`, dont le `DEFAULT` porte l'obligation
 * réglementaire — et une file triée par date d'arrivée ne dit pas laquelle
 * brûle. Les demandes en retard ouvrent donc la liste, en rouge.
 *
 * UNE DEMANDE CLOSE N'EST JAMAIS « EN RETARD ». Elle est traitée : son échéance
 * ne veut plus rien dire. Sans cette distinction, une file entièrement honorée
 * clignoterait en rouge et on cesserait de la regarder.
 *
 * TROIS NATURES, TROIS TRAITEMENTS. L'export rend une archive, la rectification
 * corrige une fiche, l'effacement appelle `anonymize_person()` — irréversible.
 * Les afficher sous une même étiquette « demande » ferait exécuter l'un pour
 * l'autre.
 */

interface Props {
  requests: PrivacyRequestView[]
  caption: string
  timezone: TimeZoneName
  loading?: boolean
}

defineProps<Props>()
const emit = defineEmits<{ handle: [request: PrivacyRequestView] }>()

const { t } = useI18n()
const { date } = useDateTime()

const columns = computed<TableColumn[]>(() => [
  { key: 'person_name', label: t('admin.user.privacy.columns.person') },
  { key: 'request_type', label: t('admin.user.privacy.columns.type') },
  { key: 'status', label: t('admin.user.privacy.columns.status') },
  { key: 'due_at', label: t('admin.user.privacy.columns.due'), hideBelow: 'sm' },
  { key: 'handled_by_name', label: t('admin.user.privacy.columns.handler'), hideBelow: 'lg' },
  { key: 'actions', label: t('admin.user.privacy.columns.actions'), align: 'end' },
])

const STATUS_INTENT = {
  received: 'info',
  in_progress: 'warning',
  completed: 'success',
  rejected: 'neutral',
} as const

function isOpen(request: PrivacyRequestView): boolean {
  return request.status === 'received' || request.status === 'in_progress'
}
</script>

<template>
  <UiTable
    :columns="columns"
    :rows="requests"
    row-key="id"
    row-label-key="person_name"
    :caption="caption"
    :loading="loading"
  >
    <template #cell-person_name="{ row }">
      <div class="min-w-0">
        <NuxtLink :to="useLocalePath()(`/admin/utilisateurs/${row.person_id}`)" class="font-medium">
          {{ row.person_name }}
        </NuxtLink>
        <p class="truncate text-sm text-text-muted">{{ row.person_email }}</p>
      </div>
    </template>

    <template #cell-request_type="{ row }">
      <UiBadge
        :intent="row.request_type === 'erasure' ? 'danger' : 'info'"
        size="sm"
        :label="t(`admin.user.privacy.type.${row.request_type}`)"
      />
    </template>

    <template #cell-status="{ row }">
      <UiBadge
        :intent="STATUS_INTENT[row.status]"
        size="sm"
        :label="t(`admin.user.privacy.status.${row.status}`)"
      />
    </template>

    <!-- L'ÉCHÉANCE, ET CE QU'IL EN RESTE. Une date seule oblige à compter. -->
    <template #cell-due_at="{ row }">
      <div :class="row.is_overdue && 'text-danger'">
        <p>{{ date(row.due_at, timezone) }}</p>
        <p v-if="row.is_overdue" class="text-sm font-medium">
          {{ t('admin.user.privacy.overdue', { count: Math.abs(row.days_left) }) }}
        </p>
        <p v-else-if="isOpen(row)" class="text-sm text-text-muted">
          {{ t('admin.user.privacy.daysLeft', { count: row.days_left }) }}
        </p>
      </div>
    </template>

    <template #cell-handled_by_name="{ row }">
      <span v-if="row.handled_by_name">{{ row.handled_by_name }}</span>
      <span v-else class="text-text-subtle">{{ t('admin.user.privacy.unassigned') }}</span>
    </template>

    <template #cell-actions="{ row }">
      <UiButton variant="secondary" size="sm" @click="emit('handle', row)">
        {{ isOpen(row) ? t('admin.user.privacy.handle') : t('admin.user.privacy.view') }}
      </UiButton>
    </template>

    <template #empty>
      <slot name="empty" />
    </template>
  </UiTable>
</template>
