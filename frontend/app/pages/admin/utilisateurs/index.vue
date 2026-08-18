<script setup lang="ts">
import type {
  GrantRolePayload,
  RoleAssignmentOptions,
  UserListFilters,
  UserListRow,
  UserListScreen,
  UserSortKey,
} from '~/types/admin-users'
import type { EffectivePermission, PersonStatus, ScopeType } from '~/types/identity'
import type { SortDirection } from '~/types/ui'

/**
 * LISTE DES UTILISATEURS — `/admin/utilisateurs`.
 *
 * L'ÉCRAN QUI RÉPARE LE DÉFAUT STRUCTUREL DE LA V1 : huit rôles GLOBAUX, dans un
 * ENUM. Être « révisionniste » valait pour toutes les COP, et confier un
 * webinaire à son responsable avait imposé de développer une page
 * d'administration séparée, dans l'urgence et en partie codée en dur, pour que
 * cette personne n'ait pas accès au reste. Ici, c'est une attribution de rôle :
 * même back-office, même code, périmètre restreint.
 *
 * LA PORTÉE EST DONC LE SUJET, ET NON UN CHAMP. Elle apparaît dans la colonne des
 * rôles — une pastille par attribution, jamais par rôle —, dans un FILTRE qui lui
 * est propre, et dans le panneau d'attribution, où elle occupe l'étape centrale.
 *
 * DEUX PERMISSIONS, DEUX PORTES. `identity.person.read`, quelle que soit sa
 * portée, ouvre la liste — qui est alors FILTRÉE sur les éditions administrées,
 * et l'écran le dit. `identity.role.assign` ouvre le panneau, et elle s'exige SUR
 * LA PORTÉE VISÉE : une coordonnatrice détachée sur la COP31 attribue sur la
 * COP31, et nulle part ailleurs.
 *
 * TOUT L'ÉTAT VIT DANS L'URL : filtres, tri. Une liste filtrée se transmet, et le
 * jour où le filtrage part au serveur (B1) ces paramètres deviennent ceux de la
 * requête.
 *
 * QUATRE ÉTATS : chargement (lignes squelettes), vide (aucune personne dans le
 * périmètre, distinct d'aucun résultat après filtrage), erreur avec reprise,
 * accès refusé.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.users' }],
})

const { t } = useI18n()
const { tr } = useI18nText()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()

useHead(() => ({ title: t('admin.user.list.title') }))

await adminScope.ensureLoaded()

/** Un compte n'appartient à aucune édition : les dates se lisent dans le fuseau
 *  de la personne connectée, à défaut en UTC — jamais celui du navigateur, qui
 *  varierait d'un poste à l'autre sans que rien ne le dise. */
const timezone = computed(() => auth.person?.timezone ?? 'UTC')

const { data: granted } = await useAsyncData<EffectivePermission[]>(
  'admin-users-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

/** Ouvre l'écran, quelle que soit la portée : la liste, elle, est filtrée. */
const canRead = computed(() => hasPermissionOnAnyScope(granted.value, 'identity.person.read'))
/** Ouvre le panneau. La portée exacte est vérifiée choix par choix. */
const canAssign = computed(() => hasPermissionOnAnyScope(granted.value, 'identity.role.assign'))
/** La file RGPD ne se découpe pas par édition : portée GLOBALE exigée. */
const canHandlePrivacy = computed(() => hasPermission(granted.value, 'identity.person.manage'))

const {
  data: screen,
  status,
  error,
  refresh,
} = await useAsyncData<UserListScreen | null>(
  'admin-user-list',
  async () => (canRead.value ? api.adminUsers.list(adminScope.scope) : null),
  { watch: [canRead, () => adminScope.scope], lazy: true },
)

// ---------------------------------------------------------------------------
// Filtres et tri — portés par l'URL
// ---------------------------------------------------------------------------

function queryText(value: unknown): string {
  return typeof value === 'string' ? value : ''
}

function queryList(value: unknown): string[] {
  if (Array.isArray(value)) return value.flatMap((entry) => String(entry).split(',')).filter(Boolean)
  if (typeof value === 'string') return value.split(',').filter(Boolean)
  return []
}

/** Les paramètres sont en FRANÇAIS : ils apparaissent dans une URL qu'on partage. */
const filters = computed<UserListFilters>(() => {
  const rawScope = queryText(route.query.portee)
  const [scopeType, scopeId] = rawScope.split(':')

  return {
    search: queryText(route.query.q),
    roles: queryList(route.query.role),
    scope_type: rawScope ? ((scopeType ?? null) as ScopeType | null) : null,
    scope_id: scopeId || null,
    statuses: queryList(route.query.statut) as PersonStatus[],
    countries: queryList(route.query.pays),
    organizations: queryList(route.query.organisation),
    without_role: queryText(route.query.sansRole) === 'oui',
    without_account: queryText(route.query.sansCompte) === 'oui',
  }
})

const SORT_PARAM: Record<string, UserSortKey> = {
  nom: 'display_name',
  adresse: 'primary_email',
  organisation: 'organization',
  pays: 'country',
  roles: 'roles',
  connexion: 'last_login_at',
  statut: 'status',
}
const PARAM_BY_SORT = Object.fromEntries(
  Object.entries(SORT_PARAM).map(([param, key]) => [key, param]),
) as Record<UserSortKey, string>

/**
 * Tri par défaut : LA DERNIÈRE CONNEXION, DÉCROISSANTE. Cet écran s'ouvre pour
 * agir sur des comptes vivants ; l'ordre alphabétique ne dit pas par où
 * commencer.
 */
const sortKey = computed<UserSortKey>(() => SORT_PARAM[queryText(route.query.tri)] ?? 'last_login_at')
const sortDirection = computed<SortDirection>(() => (queryText(route.query.sens) === 'asc' ? 'asc' : 'desc'))

function updateQuery(patch: Record<string, string | null>): void {
  const next = { ...route.query }
  for (const [key, value] of Object.entries(patch)) {
    if (value === null || value === '') delete next[key]
    else next[key] = value
  }
  router.replace({ query: next })
}

function setFilters(value: UserListFilters): void {
  updateQuery({
    q: value.search || null,
    role: value.roles.join(',') || null,
    portee:
      value.scope_type === null
        ? null
        : value.scope_type === 'global'
          ? 'global'
          : `${value.scope_type}:${value.scope_id ?? ''}`,
    statut: value.statuses.join(',') || null,
    pays: value.countries.join(',') || null,
    organisation: value.organizations.join(',') || null,
    sansRole: value.without_role ? 'oui' : null,
    sansCompte: value.without_account ? 'oui' : null,
  })
}

function setSort(key: string, direction: Exclude<SortDirection, null>): void {
  const mapped = SORT_PARAM[key] ?? (key as UserSortKey)
  updateQuery({
    // Le tri par défaut ne s'écrit pas dans l'URL.
    tri: mapped === 'last_login_at' && direction === 'desc' ? null : PARAM_BY_SORT[mapped],
    sens: direction === 'asc' ? 'asc' : null,
  })
}

// ---------------------------------------------------------------------------
// Lignes affichées
// ---------------------------------------------------------------------------

const allRows = computed<UserListRow[]>(() => screen.value?.rows ?? [])
const filteredRows = computed(() => filterUsers(allRows.value, filters.value))
const sortedRows = computed(() => sortUsers(filteredRows.value, sortKey.value, sortDirection.value))

const caption = computed(() =>
  t('admin.user.list.caption', { column: t('admin.user.list.columns.' + sortKey.value) }),
)

/**
 * Les cibles offertes au FILTRE de portée.
 *
 * Elles viennent des attributions réellement présentes dans la liste, et non du
 * catalogue des éditions : proposer de filtrer sur une COP où personne n'a de
 * rôle donne un filtre qui ne rend jamais rien.
 */
const scopeTargets = computed(() => {
  const seen = new Map<string, { value: string; label: string; scope_type: ScopeType }>()

  for (const row of allRows.value) {
    for (const assignment of row.roles) {
      if (assignment.scope_type === 'global' || !assignment.scope_id || !assignment.scope_label) continue
      seen.set(`${assignment.scope_type}:${assignment.scope_id}`, {
        value: assignment.scope_id,
        label: tr(assignment.scope_label),
        scope_type: assignment.scope_type,
      })
    }
  }

  return [...seen.values()].sort((a, b) => a.label.localeCompare(b.label, 'fr'))
})

function openUser(row: UserListRow): void {
  navigateTo(localePath(`/admin/utilisateurs/${row.person_id}`))
}

// ---------------------------------------------------------------------------
// Le panneau d'attribution
// ---------------------------------------------------------------------------

const panelRow = ref<UserListRow | null>(null)
const panelOpen = ref(false)
const submitting = ref(false)
const panelError = ref<string | null>(null)

const { data: roleOptions } = await useAsyncData<RoleAssignmentOptions | null>(
  'admin-user-role-options',
  async () => (canAssign.value ? api.adminUsers.roleOptions(granted.value) : null),
  { default: () => null, watch: [canAssign, granted], lazy: true },
)

/** Permissions de la personne VISÉE — pour annoncer ce qu'elle gagnera. */
const { data: targetPermissions, refresh: refreshTarget } = await useAsyncData<EffectivePermission[]>(
  'admin-user-target-permissions',
  async () => (panelRow.value ? api.identity.permissions(panelRow.value.person_id) : []),
  { default: () => [], watch: [panelRow], lazy: true },
)

function openPanel(row: UserListRow): void {
  panelRow.value = row
  panelError.value = null
  panelOpen.value = true
}

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
</script>

<template>
  <div class="mx-auto w-full max-w-[100rem]">
    <!-- ACCÈS REFUSÉ — la permission de lecture manque, quelle que soit sa
         portée. Distinct d'un écran vide. -->
    <UiForbiddenState
      v-if="!adminScope.isLoading && !canRead"
      :required-scope="t('admin.user.list.forbidden.scope')"
      action-to="/"
      :action-label="t('nav.admin.backToSite')"
    />

    <template v-else>
      <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">
            {{ t('admin.user.list.title') }}
          </h1>
          <p class="mt-1 max-w-(--measure) text-text-muted">{{ t('admin.user.list.subtitle') }}</p>
        </div>

        <UiButton
          v-if="canHandlePrivacy"
          variant="secondary"
          icon="shield-check"
          :to="localePath('/admin/utilisateurs/rgpd')"
        >
          {{ t('admin.user.list.privacy.link') }}
          <span v-if="screen && screen.open_privacy_requests > 0" class="ml-2">
            <UiBadge intent="warning" size="sm" :label="String(screen.open_privacy_requests)" />
          </span>
        </UiButton>
      </header>

      <UiErrorState
        v-if="error"
        class="mt-8"
        :retry-label="t('common.actions.retry')"
        @retry="refresh()"
      />

      <template v-else>
        <!-- LE PÉRIMÈTRE, DIT EN CLAIR. Une liste restreinte qui se tait laisse
             croire que la plateforme compte huit personnes. -->
        <UiAlert
          v-if="screen?.scoped_to_events"
          class="mt-6"
          intent="info"
          compact
          :message="t('admin.user.list.scoped.notice')"
        />

        <UiAlert
          v-else-if="screen && screen.restricted_accounts > 0"
          class="mt-6"
          intent="warning"
          compact
          :message="t('admin.user.list.restricted.notice', { count: screen.restricted_accounts })"
        />

        <UiEmptyState
          v-if="allRows.length === 0 && status !== 'pending'"
          class="mt-8"
          icon="users"
          :title="t('admin.user.list.empty.title')"
          :description="t('admin.user.list.empty.description')"
        />

        <template v-else>
          <AdminUsersFilters
            v-if="allRows.length > 2"
            class="mt-6"
            :filters="filters"
            :roles="screen?.roles ?? []"
            :countries="screen?.countries ?? []"
            :organizations="screen?.organizations ?? []"
            :scope-targets="scopeTargets"
            :total="allRows.length"
            :shown="filteredRows.length"
            :disabled="status === 'pending'"
            @update:filters="setFilters"
          />

          <AdminUsersTable
            class="mt-4"
            :rows="sortedRows"
            :caption="caption"
            :sort-key="sortKey"
            :sort-direction="sortDirection"
            :timezone="timezone"
            :can-assign="canAssign"
            :loading="status === 'pending'"
            @sort="setSort"
            @open="openUser"
            @assign="openPanel"
          >
            <template #empty>
              <UiEmptyState
                icon="search"
                filtered
                :title="t('admin.user.list.noResults.title')"
                :description="t('admin.user.list.noResults.description', { total: allRows.length })"
                :action-label="t('admin.user.list.noResults.action')"
                @action="setFilters(NO_USER_FILTERS)"
              />
            </template>
          </AdminUsersTable>
        </template>
      </template>

      <AdminUsersRolePanel
        v-if="panelRow"
        v-model:open="panelOpen"
        :person-id="panelRow.person_id"
        :person-name="panelRow.display_name"
        :options="roleOptions"
        :assignments="panelRow.roles"
        :target-permissions="targetPermissions"
        :submitting="submitting"
        :error="panelError"
        @submit="grantRole"
      />
    </template>
  </div>
</template>
