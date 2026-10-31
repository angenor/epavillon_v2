<script setup lang="ts">
import type { OrganizationMemberRow } from '~/types/admin-organizations'
import type { Intent, TableColumn } from '~/types/ui'
import type { MembershipRole, MembershipStatus } from '~/types/org'
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
 * L'ÉQUIPE AGIT QUAND L'ORGANISATION NE LE PEUT PAS (31/10). Le back-office ne
 * faisait que regarder : une organisation sans référent n'avait alors plus
 * personne pour accepter une demande, l'IFDD comprise. Avec la permission de
 * gestion en portée globale, l'équipe accepte une demande en fixant son rôle et
 * change le rôle d'un membre actif. Une invitation reste sans bouton : elle
 * attend la personne (écart n° 33). Retirer son rôle au dernier référent
 * demande une confirmation, que l'API exige aussi.
 */

interface Props {
  members: OrganizationMemberRow[]
  timezone: TimeZoneName
  /** Gestion des organisations en portée globale. */
  canManage?: boolean
  busy?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
  decide: [row: OrganizationMemberRow, approved: boolean, role?: MembershipRole]
  setRole: [row: OrganizationMemberRow, role: MembershipRole, confirmLastManager: boolean]
}>()

const { t } = useI18n()
const { date } = useDateTime()

const columns = computed<TableColumn[]>(() => [
  { key: 'person', label: t('admin.organization.detail.members.columns.person') },
  { key: 'role', label: t('admin.organization.detail.members.columns.role') },
  { key: 'status', label: t('admin.organization.detail.members.columns.status') },
  { key: 'since', label: t('admin.organization.detail.members.columns.since'), hideBelow: 'lg' },
  ...(props.canManage ? [{ key: 'actions', label: t('admin.organization.detail.members.columns.actions'), align: 'end' as const }] : []),
])

const activeManagers = computed(
  () => props.members.filter((row) => row.status === 'active' && row.role === 'manager').length,
)

const isRequest = (row: OrganizationMemberRow) => row.status === 'pending' && row.invited_at === null

// --- La fenêtre : accepter une demande, ou changer un rôle -------------------

const ROLES: MembershipRole[] = ['manager', 'member', 'contributor']
const target = ref<OrganizationMemberRow | null>(null)
const choice = ref<MembershipRole>('member')

const dialogOpen = computed({
  get: () => target.value !== null,
  set: (open: boolean) => {
    if (!open) target.value = null
  },
})
const accepting = computed(() => target.value !== null && isRequest(target.value))

const roleOptions = computed(() =>
  ROLES.map((role) => ({
    value: role,
    label: t(`admin.organization.detail.members.role.${role}`),
    description: t(`admin.organization.detail.members.dialog.hint.${role}`),
  })),
)

const lastManager = computed(
  () =>
    !accepting.value &&
    target.value?.role === 'manager' &&
    choice.value !== 'manager' &&
    activeManagers.value === 1,
)
const unchanged = computed(() => !accepting.value && target.value?.role === choice.value)

function openAccept(row: OrganizationMemberRow): void {
  target.value = row
  // Sans référent, la demande acceptée est d'abord celle d'un référent.
  choice.value = activeManagers.value === 0 ? 'manager' : row.role
}

function openRole(row: OrganizationMemberRow): void {
  target.value = row
  choice.value = row.role
}

function confirm(): void {
  const row = target.value
  if (!row || unchanged.value) return
  if (accepting.value) emit('decide', row, true, choice.value)
  else emit('setRole', row, choice.value, lastManager.value)
  target.value = null
}

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

    <UiAlert
      v-if="activeManagers === 0"
      class="mt-4"
      intent="warning"
      :title="t('admin.organization.detail.members.noManager.title')"
      :message="t('admin.organization.detail.members.noManager.message')"
    />

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

      <template #cell-actions="{ row }">
        <span class="flex flex-wrap justify-end gap-2">
          <template v-if="isRequest(row)">
            <UiButton variant="primary" size="sm" :disabled="props.busy" @click="openAccept(row)">
              {{ t('admin.organization.detail.members.accept') }}
            </UiButton>
            <UiButton variant="ghost" size="sm" :disabled="props.busy" @click="emit('decide', row, false)">
              {{ t('admin.organization.detail.members.decline') }}
            </UiButton>
          </template>
          <span v-else-if="row.status === 'pending'" class="max-w-56 text-right text-xs text-text-muted">
            {{ t('admin.organization.detail.members.awaitingPerson') }}
          </span>
          <UiButton v-else-if="row.status === 'active'" variant="secondary" size="sm" :disabled="props.busy" @click="openRole(row)">
            {{ t('admin.organization.detail.members.changeRole') }}
          </UiButton>
        </span>
      </template>

      <template #empty>
        <UiEmptyState compact :title="t('admin.organization.detail.members.empty')" />
      </template>
    </UiTable>

    <UiModal
      v-model:open="dialogOpen"
      size="sm"
      :title="
        target
          ? t(`admin.organization.detail.members.dialog.${accepting ? 'acceptTitle' : 'roleTitle'}`, { name: target.display_name })
          : ''
      "
      :description="
        target
          ? accepting
            ? t('admin.organization.detail.members.dialog.acceptIntro', { name: target.display_name })
            : t('admin.organization.detail.members.dialog.roleIntro', {
                role: t(`admin.organization.detail.members.role.${target.role}`),
              })
          : ''
      "
    >
      <UiRadio
        v-model="choice"
        :options="roleOptions"
        :label="t('admin.organization.detail.members.dialog.roleLabel')"
      />
      <UiAlert
        v-if="lastManager"
        class="mt-4"
        intent="warning"
        :title="t('admin.organization.detail.members.dialog.lastManager.title')"
        :message="t('admin.organization.detail.members.dialog.lastManager.message')"
        live
      />

      <template #footer>
        <UiButton variant="secondary" @click="target = null">
          {{ t('admin.organization.detail.members.dialog.cancel') }}
        </UiButton>
        <UiButton :variant="lastManager ? 'danger' : 'primary'" :disabled="unchanged" @click="confirm">
          {{
            t(
              `admin.organization.detail.members.dialog.${accepting ? 'accept' : lastManager ? 'forceSave' : 'save'}`,
            )
          }}
        </UiButton>
      </template>
    </UiModal>
  </section>
</template>
