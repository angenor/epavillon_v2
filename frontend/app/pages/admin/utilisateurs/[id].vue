<script setup lang="ts">
import type {
  GrantRolePayload,
  RoleAssignmentOptions,
  RoleAssignmentView,
  SetPersonStatusPayload,
  UserDetail,
} from '~/types/admin-users'
import type { EffectivePermission } from '~/types/identity'
import type { TabItem } from '~/types/ui'

/**
 * FICHE D'UNE PERSONNE — `/admin/utilisateurs/<id>`.
 *
 * QUATRE ONGLETS, ET LE TROISIÈME EST CELUI QUE LE PROMPT DEMANDE EN PROPRE :
 * « un écran montrant les permissions effectives d'une personne — voici ce
 * qu'elle peut faire, et où ». Il vit ici plutôt qu'à une adresse séparée parce
 * qu'on ne l'ouvre jamais dans le vide : on y arrive en regardant les rôles de
 * quelqu'un, et le trajet entre les deux doit coûter un clic.
 *
 *   Profil        la personne, ses comptes, son statut, ses consentements
 *   Rôles         les attributions EN COURS, avec le geste pour les retirer
 *   Permissions   ce qu'elle peut faire, et où — plus ce qu'elle ne peut pas
 *   Historique    tout ce qui a été accordé puis retiré, chronologiquement
 *
 * LECTURE SEULE HORS PÉRIMÈTRE. Un administrateur détaché peut avoir à consulter
 * une fiche sans pouvoir y toucher — vérifier une adresse, comprendre un rôle.
 * Refuser l'accès entier serait excessif ; laisser les boutons actifs le serait
 * davantage. La fiche porte `in_scope`, et l'écran s'y règle en le DISANT.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.users', to: '/admin/utilisateurs' }, { labelKey: 'admin.user.detail.breadcrumb' }],
})

const { t } = useI18n()
const { tr } = useI18nText()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const route = useRoute()
const localePath = useLocalePath()

const personId = computed(() => String(route.params.id))
const activeTab = ref('profile')

await adminScope.ensureLoaded()

const timezone = computed(() => auth.person?.timezone ?? 'UTC')

const { data: granted } = await useAsyncData<EffectivePermission[]>(
  'admin-user-detail-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canRead = computed(() => hasPermissionOnAnyScope(granted.value, 'identity.person.read'))
const canAssign = computed(() => hasPermissionOnAnyScope(granted.value, 'identity.role.assign'))
const canManage = computed(() => hasPermissionOnAnyScope(granted.value, 'identity.person.manage'))

const {
  data: user,
  status,
  error,
  refresh,
} = await useAsyncData<UserDetail | null>(
  `admin-user-${personId.value}`,
  async () => (canRead.value ? api.adminUsers.detail(personId.value, adminScope.scope) : null),
  { watch: [canRead, personId, () => adminScope.scope], lazy: true },
)

useHead(() => ({ title: user.value?.display_name ?? t('admin.user.detail.title') }))

/** Hors périmètre : la fiche se lit, elle ne s'écrit pas. */
const isEditable = computed(() => Boolean(user.value?.in_scope))

const tabs = computed<TabItem[]>(() => [
  { value: 'profile', label: t('admin.user.detail.tabs.profile') },
  { value: 'roles', label: t('admin.user.detail.tabs.roles'), count: user.value?.assignments.length },
  { value: 'permissions', label: t('admin.user.detail.tabs.permissions'), count: user.value?.permissions.total },
  { value: 'history', label: t('admin.user.detail.tabs.history'), count: user.value?.history.length },
])

/** Noms des éditions administrées, pour la phrase de périmètre. */
const administeredLabels = computed(() => {
  const ids = user.value?.permissions.administered.event_ids ?? []
  return ids.map((id) => {
    const assignment = user.value?.assignments.find(
      (entry) => entry.scope_type === 'event' && entry.scope_id === id,
    )
    return assignment?.scope_label ? tr(assignment.scope_label) : id
  })
})

// ---------------------------------------------------------------------------
// Attribuer
// ---------------------------------------------------------------------------

const panelOpen = ref(false)
const panelError = ref<string | null>(null)
const submitting = ref(false)

const { data: roleOptions } = await useAsyncData<RoleAssignmentOptions | null>(
  'admin-user-detail-role-options',
  async () => (canAssign.value ? api.adminUsers.roleOptions(granted.value) : null),
  { default: () => null, watch: [canAssign, granted], lazy: true },
)

const { data: targetPermissions, refresh: refreshTarget } = await useAsyncData<EffectivePermission[]>(
  `admin-user-target-${personId.value}`,
  async () => api.identity.permissions(personId.value),
  { default: () => [], watch: [personId], lazy: true },
)

async function grantRole(payload: GrantRolePayload): Promise<void> {
  submitting.value = true
  panelError.value = null

  try {
    const result = await api.adminUsers.grantRole(payload, auth.person?.id ?? null, granted.value)
    if (result.status !== 'granted') {
      panelError.value = t(`admin.user.roles.error.${result.status}`)
      return
    }
    panelOpen.value = false
    await Promise.all([refresh(), refreshTarget()])
  } finally {
    submitting.value = false
  }
}

// ---------------------------------------------------------------------------
// Retirer
// ---------------------------------------------------------------------------

const revoking = ref<RoleAssignmentView | null>(null)
const revokeOpen = ref(false)
const revokeError = ref<string | null>(null)

function askRevoke(assignment: RoleAssignmentView): void {
  revoking.value = assignment
  revokeError.value = null
  revokeOpen.value = true
}

async function revokeRole(reason: string): Promise<void> {
  if (!revoking.value) return
  submitting.value = true
  revokeError.value = null

  try {
    const result = await api.adminUsers.revokeRole(
      { assignment_id: revoking.value.id, reason },
      auth.person?.id ?? null,
      granted.value,
    )
    if (result.status !== 'revoked') {
      revokeError.value = t(`admin.user.roles.error.${result.status}`)
      return
    }
    revokeOpen.value = false
    await Promise.all([refresh(), refreshTarget()])
  } finally {
    submitting.value = false
  }
}

// ---------------------------------------------------------------------------
// Suspendre, bloquer, rétablir
// ---------------------------------------------------------------------------

const statusOpen = ref(false)
const statusError = ref<string | null>(null)

async function setStatus(payload: Omit<SetPersonStatusPayload, 'person_id'>): Promise<void> {
  submitting.value = true
  statusError.value = null

  try {
    const result = await api.adminUsers.setStatus(
      { ...payload, person_id: personId.value },
      auth.person?.id ?? null,
      adminScope.scope,
    )
    if (result.status !== 'saved') {
      statusError.value = t(`admin.user.status.error.${result.status}`)
      return
    }
    statusOpen.value = false
    await refresh()
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <div class="mx-auto w-full max-w-[80rem]">
    <UiForbiddenState
      v-if="!adminScope.isLoading && !canRead"
      :required-scope="t('admin.user.list.forbidden.scope')"
      action-to="/admin/utilisateurs"
      :action-label="t('admin.user.detail.backToList')"
    />

    <UiErrorState v-else-if="error" :retry-label="t('common.actions.retry')" @retry="refresh()" />

    <div v-else-if="status === 'pending' && !user" class="space-y-4">
      <UiSkeletonLoader height="4rem" />
      <UiSkeletonLoader height="18rem" />
    </div>

    <UiEmptyState
      v-else-if="!user"
      icon="users"
      :title="t('admin.user.detail.missing.title')"
      :description="t('admin.user.detail.missing.description')"
      :action-label="t('admin.user.detail.backToList')"
      :action-to="localePath('/admin/utilisateurs')"
    />

    <template v-else>
      <header class="flex flex-wrap items-start justify-between gap-x-6 gap-y-3">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">{{ user.display_name }}</h1>
          <p class="mt-1 text-text-muted">
            {{ [user.job_title, user.organization_name].filter(Boolean).join(' · ') || user.primary_email }}
          </p>

          <div class="mt-3 flex flex-wrap gap-1.5">
            <AdminUsersRoleBadge
              v-for="assignment in user.assignments"
              :key="assignment.id"
              :assignment="assignment"
              size="sm"
            />
            <span v-if="user.assignments.length === 0" class="text-sm text-text-subtle">
              {{ t('admin.user.list.cell.noRole') }}
            </span>
          </div>
        </div>

        <div class="flex flex-wrap gap-3">
          <UiButton
            v-if="canManage && isEditable"
            variant="secondary"
            :icon="user.status === 'active' ? 'ban' : 'check'"
            @click="statusOpen = true"
          >
            {{
              user.status === 'active'
                ? t('admin.user.status.dialog.open.restrict')
                : t('admin.user.status.dialog.open.restore')
            }}
          </UiButton>
          <UiButton v-if="canAssign && isEditable" icon="plus" @click="panelOpen = true">
            {{ t('admin.user.roles.panel.openShort') }}
          </UiButton>
        </div>
      </header>

      <!-- HORS PÉRIMÈTRE : la fiche se lit, elle ne s'écrit pas — et on le dit. -->
      <UiAlert
        v-if="!isEditable"
        class="mt-6"
        intent="info"
        compact
        :message="t('admin.user.detail.readOnly')"
      />

      <UiTabs
        v-model="activeTab"
        class="mt-6"
        :items="tabs"
        :label="t('admin.user.detail.tabs.label')"
      >
        <div class="pt-6">
          <AdminUsersProfileCard v-if="activeTab === 'profile'" :user="user" :timezone="timezone" />

          <AdminUsersAssignmentList
            v-else-if="activeTab === 'roles'"
            :assignments="user.assignments"
            :timezone="timezone"
            :can-revoke="canAssign && isEditable"
            @revoke="askRevoke"
          />

          <AdminUsersPermissionMatrix
            v-else-if="activeTab === 'permissions'"
            :view="user.permissions"
            :administered-labels="administeredLabels"
          />

          <AdminUsersHistoryList v-else :entries="user.history" :timezone="timezone" />
        </div>
      </UiTabs>

      <!-- LES DEMANDES RGPD DE CETTE PERSONNE, sur la fiche : c'est là qu'on les
           cherche quand on part de quelqu'un plutôt que de la file. -->
      <section v-if="user.privacy_requests.length" class="mt-10">
        <h2 class="font-display text-lg font-semibold">{{ t('admin.user.detail.privacy.title') }}</h2>
        <ul class="mt-3 space-y-2">
          <li
            v-for="request in user.privacy_requests"
            :key="request.id"
            class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-border bg-surface-raised p-3 text-sm"
          >
            <span class="flex flex-wrap items-center gap-2">
              <UiBadge
                :intent="request.request_type === 'erasure' ? 'danger' : 'info'"
                size="sm"
                :label="t(`admin.user.privacy.type.${request.request_type}`)"
              />
              <span>{{ t(`admin.user.privacy.status.${request.status}`) }}</span>
            </span>
            <NuxtLink :to="localePath('/admin/utilisateurs/rgpd')" class="text-accent">
              {{ t('admin.user.detail.privacy.link') }}
            </NuxtLink>
          </li>
        </ul>
      </section>

      <AdminUsersRolePanel
        v-model:open="panelOpen"
        :person-id="user.person_id"
        :person-name="user.display_name"
        :options="roleOptions"
        :assignments="user.assignments"
        :target-permissions="targetPermissions"
        :submitting="submitting"
        :error="panelError"
        @submit="grantRole"
      />

      <AdminUsersRevokeDialog
        v-model:open="revokeOpen"
        :assignment="revoking"
        :person-name="user.display_name"
        :submitting="submitting"
        :error="revokeError"
        @confirm="revokeRole"
      />

      <AdminUsersStatusDialog
        v-model:open="statusOpen"
        :user="user"
        :submitting="submitting"
        :error="statusError"
        @submit="setStatus"
      />
    </template>
  </div>
</template>
