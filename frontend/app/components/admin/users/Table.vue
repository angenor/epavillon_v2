<script setup lang="ts">
import type { UserListRow, UserSortKey } from '~/types/admin-users'
import type { SortDirection, TableColumn } from '~/types/ui'
import type { TimeZoneName } from '~/types/shared'

/**
 * LA LISTE DES UTILISATEURS.
 *
 * SEPT COLONNES, ET ELLES NE DISPARAISSENT PAS DANS LE MÊME ORDRE. Ce qui
 * IDENTIFIE une personne — nom, adresse, statut — tient jusqu'en 375 px ; ce qui
 * la SITUE — organisation, pays, dernière connexion — se replie. Les RÔLES, eux,
 * restent au plus tard possible : ce sont eux qu'on vient lire.
 *
 * TROIS PASTILLES DE RÔLE AU PLUS, puis « +N » — la règle n° 3 du guide de style,
 * écrite pour les thématiques et qui vaut ici pour la même raison : au-delà,
 * elles cessent d'informer et la ligne devient illisible.
 *
 * « JAMAIS CONNECTÉ » ET « AUCUN COMPTE » NE SE DISENT PAS PAREIL. La personne et
 * le compte sont deux choses distinctes dans le modèle, et les confondre ferait
 * relancer quelqu'un qui n'a jamais reçu de quoi se connecter — une intervenante
 * saisie par un tiers, une invitation en attente.
 *
 * LES DATES SONT DANS LE FUSEAU DE LA PERSONNE CONNECTÉE : un compte n'appartient
 * à aucune édition, il n'a donc pas de fuseau propre.
 */

interface Props {
  rows: UserListRow[]
  caption: string
  sortKey: UserSortKey
  sortDirection: SortDirection
  timezone: TimeZoneName
  /** Le panneau d'attribution est-il ouvrable ? `identity.role.assign`. */
  canAssign?: boolean
  loading?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
  sort: [key: string, direction: Exclude<SortDirection, null>]
  open: [row: UserListRow]
  assign: [row: UserListRow]
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()

const columns = computed<TableColumn[]>(() => [
  { key: 'display_name', label: t('admin.user.list.columns.display_name'), sortable: true },
  { key: 'organization', label: t('admin.user.list.columns.organization'), sortable: true, hideBelow: 'lg' },
  { key: 'country', label: t('admin.user.list.columns.country'), sortable: true, hideBelow: 'xl' },
  { key: 'roles', label: t('admin.user.list.columns.roles'), sortable: true, width: '22rem' },
  { key: 'last_login_at', label: t('admin.user.list.columns.last_login_at'), sortable: true, hideBelow: 'lg' },
  { key: 'status', label: t('admin.user.list.columns.status'), sortable: true },
  { key: 'actions', label: t('admin.user.list.columns.actions'), align: 'end', width: '4rem' },
])

const STATUS_INTENT = {
  active: 'success',
  suspended: 'warning',
  blocked: 'danger',
  anonymized: 'neutral',
} as const

/** Trois rôles au plus ; les suivants se replient. */
const MAX_ROLE_BADGES = 3
</script>

<template>
  <UiTable
    :columns="columns"
    :rows="rows"
    row-key="person_id"
    row-label-key="display_name"
    :caption="caption"
    :sort-key="sortKey"
    :sort-direction="sortDirection"
    :loading="loading"
    sticky-header
    @sort="(key, direction) => emit('sort', key, direction)"
    @row-click="(row) => emit('open', row)"
  >
    <template #cell-display_name="{ row }">
      <div class="min-w-0">
        <p class="truncate font-medium">{{ row.display_name }}</p>
        <p class="flex items-center gap-1.5 truncate text-sm text-text-muted">
          <span class="truncate">{{ row.primary_email }}</span>
          <!-- Adresse non vérifiée : la personne ne peut pas se connecter. -->
          <UiIcon
            v-if="row.email_verified_at === null && row.has_account"
            name="warning"
            size="0.9rem"
            class="shrink-0 text-warning"
            :aria-label="t('admin.user.list.cell.emailUnverified')"
          />
        </p>
        <p v-if="row.job_title" class="truncate text-sm text-text-subtle lg:hidden">{{ row.job_title }}</p>
      </div>
    </template>

    <template #cell-organization="{ row }">
      <span v-if="row.organization_name" class="block truncate">
        {{ row.organization_acronym ?? row.organization_name }}
      </span>
      <span v-else class="text-text-subtle">{{ t('admin.user.list.cell.noOrganization') }}</span>
    </template>

    <template #cell-country="{ row }">
      <span v-if="row.country_name">{{ tr(row.country_name) }}</span>
      <span v-else class="text-text-subtle">—</span>
    </template>

    <!-- LES RÔLES, AVEC LEUR PORTÉE. Une pastille par attribution, jamais par
         rôle : « Administrateur » deux fois, sur deux éditions, sont deux
         attributions différentes et doivent se voir comme telles. -->
    <template #cell-roles="{ row }">
      <div v-if="row.roles.length" class="flex min-w-0 flex-wrap items-center gap-1">
        <AdminUsersRoleBadge
          v-for="assignment in row.roles.slice(0, MAX_ROLE_BADGES)"
          :key="assignment.id"
          :assignment="assignment"
          size="sm"
        />
        <UiBadge
          v-if="row.roles.length > MAX_ROLE_BADGES"
          size="sm"
          :label="`+${row.roles.length - MAX_ROLE_BADGES}`"
          :title="row.roles.slice(MAX_ROLE_BADGES).map((assignment) => tr(assignment.role_label)).join(', ')"
        />
      </div>
      <span v-else class="text-sm text-text-subtle">{{ t('admin.user.list.cell.noRole') }}</span>
    </template>

    <template #cell-last_login_at="{ row }">
      <span v-if="row.last_login_at">{{ date(row.last_login_at, timezone) }}</span>
      <!-- Les deux absences ne se disent pas pareil : sans compte, on ne peut
           pas se connecter ; avec un compte inutilisé, on ne l'a pas fait. -->
      <span v-else-if="!row.has_account" class="text-sm text-text-subtle italic">
        {{ t('admin.user.list.cell.noAccount') }}
      </span>
      <span v-else class="text-sm text-text-subtle">{{ t('admin.user.list.cell.neverLogged') }}</span>
    </template>

    <template #cell-status="{ row }">
      <div class="flex flex-wrap items-center gap-1">
        <UiBadge
          :intent="STATUS_INTENT[row.status]"
          size="sm"
          :label="t(`admin.user.status.${row.status}`)"
          :title="row.status_reason ?? undefined"
        />
        <UiIcon
          v-if="row.locked_until"
          name="lock"
          size="0.9rem"
          class="text-warning"
          :aria-label="t('admin.user.list.cell.locked')"
        />
        <UiIcon
          v-if="row.mfa_enabled"
          name="shield-check"
          size="0.9rem"
          class="text-success"
          :aria-label="t('admin.user.list.cell.mfa')"
        />
        <UiBadge
          v-if="row.open_privacy_request"
          intent="info"
          size="sm"
          :label="t(`admin.user.privacy.type.${row.open_privacy_request}`)"
        />
      </div>
    </template>

    <template #cell-actions="{ row }">
      <UiButton
        v-if="canAssign"
        variant="ghost"
        size="sm"
        icon="plus"
        icon-only
        :label="t('admin.user.roles.panel.open', { name: row.display_name })"
        @click.stop="emit('assign', row)"
      />
    </template>

    <template #empty>
      <slot name="empty" />
    </template>
  </UiTable>
</template>
