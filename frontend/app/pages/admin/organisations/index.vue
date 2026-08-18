<script setup lang="ts">
import type {
  OrganizationListFilters,
  OrganizationListRow,
  OrganizationListScreen,
  OrganizationSortKey,
} from '~/types/admin-organizations'
import type { EffectivePermission } from '~/types/identity'
import type { OrganizationStatus } from '~/types/org'
import type { SortDirection } from '~/types/ui'

/**
 * LISTE DES ORGANISATIONS — `/admin/organisations`.
 *
 * L'ÉCRAN DU RÉFÉRENTIEL, celui par lequel on tient le défaut n° 1 de la v1. Il
 * répond en une requête à la demande du cadrage : « leurs activités, leurs
 * membres, nombre d'activités validées, ratio » — c'est
 * `analytics.mv_organization_scorecard`, qui existait dans le modèle depuis le
 * premier jour et n'était affichée nulle part.
 *
 * TRI PAR SCORE DE CONFIANCE CROISSANT PAR DÉFAUT, ce que demande le prompt : la
 * première ligne est toujours la fiche qui mérite un regard. Une liste
 * alphabétique de plusieurs centaines d'entrées ne dit pas par où commencer, et
 * c'est ainsi que les fiches douteuses restent des années en place.
 *
 * LE PÉRIMÈTRE D'ADMINISTRATION S'APPLIQUE PAR L'AUTRE BOUT. Une organisation
 * n'appartient à aucune édition : la règle métier n° 8 ne peut pas filtrer sur
 * `event_id`. Elle se lit ici en deux temps — la permission
 * `org.organization.read`, quelle que soit sa portée, ouvre l'écran ; la liste,
 * elle, ne montre que les organisations ayant déposé ou tenu une activité dans
 * les éditions administrées. L'écran le DIT quand il a restreint : une liste de
 * six lignes qui se tairait laisserait croire que la plateforme compte six
 * organisations, et le premier réflexe serait d'en créer une septième.
 *
 * TOUT L'ÉTAT VIT DANS L'URL : filtres, tri. Une liste filtrée se transmet, et le
 * jour où le filtrage part au serveur (B3) ces paramètres deviennent ceux de la
 * requête.
 *
 * QUATRE ÉTATS : chargement (lignes squelettes), vide (aucune fiche dans le
 * périmètre, distinct d'aucun résultat après filtrage), erreur avec reprise,
 * accès refusé.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.organizations' }],
})

const { t } = useI18n()
const { tr } = useI18nText()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()

useHead(() => ({ title: t('admin.organization.list.title') }))

await adminScope.ensureLoaded()

/**
 * Les dates de cet écran — création, dernière activité — n'appartiennent à
 * aucune édition et n'ont donc pas de fuseau propre. Elles se lisent dans celui
 * de la personne connectée, comme le ferait n'importe quel outil
 * d'administration ; à défaut, en UTC plutôt que dans le fuseau du navigateur,
 * qui varierait d'un poste à l'autre sans que rien ne le dise.
 */
const timezone = computed(() => auth.person?.timezone ?? 'UTC')

const { data: granted } = await useAsyncData<EffectivePermission[]>(
  'admin-organizations-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

/** Ouvre l'écran, quelle que soit la portée : la liste, elle, est filtrée. */
const canRead = computed(() => hasPermissionOnAnyScope(granted.value, 'org.organization.read'))
/** La fusion déplace des rattachements partout : portée GLOBALE exigée. */
const canMerge = computed(() => hasPermission(granted.value, 'org.organization.merge'))

const {
  data: screen,
  status,
  error,
  refresh,
} = await useAsyncData<OrganizationListScreen | null>(
  'admin-organization-list',
  async () => (canRead.value ? api.adminOrganizations.list(adminScope.scope) : null),
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

function queryTristate(value: unknown): boolean | null {
  const raw = queryText(value)
  if (raw === 'oui') return true
  if (raw === 'non') return false
  return null
}

/** Les paramètres sont en FRANÇAIS : ils apparaissent dans une URL qu'on partage. */
const filters = computed<OrganizationListFilters>(() => ({
  search: queryText(route.query.q),
  countries: queryList(route.query.pays),
  types: queryList(route.query.type),
  statuses: queryList(route.query.statut) as OrganizationStatus[],
  verified: queryTristate(route.query.verifiee),
  has_duplicate: queryTristate(route.query.doublon),
  max_trust_score: route.query.confiance === 'faible' ? LOW_TRUST_SCORE : null,
}))

const SORT_PARAM: Record<string, OrganizationSortKey> = {
  nom: 'legal_name',
  pays: 'country',
  type: 'type',
  membres: 'membres_actifs',
  deposees: 'propositions_deposees',
  acceptees: 'propositions_acceptees',
  ratio: 'ratio_acceptation',
  confiance: 'score_confiance',
  activite: 'derniere_activite',
}
const PARAM_BY_SORT = Object.fromEntries(
  Object.entries(SORT_PARAM).map(([param, key]) => [key, param]),
) as Record<OrganizationSortKey, string>

/**
 * Tri par défaut : LE SCORE DE CONFIANCE, CROISSANT. C'est la demande du prompt,
 * et le seul tri qui ouvre l'écran sur du travail à faire plutôt que sur une
 * lettre de l'alphabet.
 */
const sortKey = computed<OrganizationSortKey>(
  () => SORT_PARAM[queryText(route.query.tri)] ?? 'score_confiance',
)
const sortDirection = computed<SortDirection>(() =>
  queryText(route.query.sens) === 'desc' ? 'desc' : 'asc',
)

function updateQuery(patch: Record<string, string | null>): void {
  const next = { ...route.query }
  for (const [key, value] of Object.entries(patch)) {
    if (value === null || value === '') delete next[key]
    else next[key] = value
  }
  router.replace({ query: next })
}

function setFilters(value: OrganizationListFilters): void {
  updateQuery({
    q: value.search || null,
    pays: value.countries.join(',') || null,
    type: value.types.join(',') || null,
    statut: value.statuses.join(',') || null,
    verifiee: value.verified === null ? null : value.verified ? 'oui' : 'non',
    doublon: value.has_duplicate === null ? null : value.has_duplicate ? 'oui' : 'non',
    confiance: value.max_trust_score === null ? null : 'faible',
  })
}

function setSort(key: string, direction: Exclude<SortDirection, null>): void {
  const mapped = SORT_PARAM[key] ?? (key as OrganizationSortKey)
  updateQuery({
    // Le tri par défaut ne s'écrit pas dans l'URL : une adresse propre pour la
    // vue d'ouverture, un paramètre pour tout écart.
    tri: mapped === 'score_confiance' && direction === 'asc' ? null : PARAM_BY_SORT[mapped],
    sens: direction === 'desc' ? 'desc' : null,
  })
}

// ---------------------------------------------------------------------------
// Lignes affichées
// ---------------------------------------------------------------------------

const allRows = computed<OrganizationListRow[]>(() => screen.value?.rows ?? [])

const filteredRows = computed(() => filterOrganizations(allRows.value, filters.value))

const sortedRows = computed(() =>
  sortOrganizations(filteredRows.value, sortKey.value, sortDirection.value, (row) =>
    row.organization_type_label ? tr(row.organization_type_label) : row.organization_type_code,
  ),
)

const caption = computed(() =>
  t('admin.organization.list.caption', {
    column: t('admin.organization.list.columns.' + sortKey.value),
  }),
)

function openOrganization(row: OrganizationListRow): void {
  navigateTo(localePath(`/admin/organisations/${row.organization_id}`))
}
</script>

<template>
  <div class="mx-auto w-full max-w-[100rem]">
    <!-- ACCÈS REFUSÉ — la permission de lecture manque, quelle que soit sa
         portée. Distinct d'un écran vide. -->
    <UiForbiddenState
      v-if="!adminScope.isLoading && !canRead"
      :required-scope="t('admin.organization.list.forbidden.scope')"
      action-to="/"
      :action-label="t('nav.admin.backToSite')"
    />

    <template v-else>
      <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">
            {{ t('admin.organization.list.title') }}
          </h1>
          <p class="mt-1 max-w-(--measure) text-text-muted">
            {{ t('admin.organization.list.subtitle') }}
          </p>
        </div>

        <!-- LE PONT VERS LA FILE. Il porte son décompte : une file vide ne se
             visite pas, et une file qui compte douze paires doit se voir depuis
             la liste. -->
        <UiButton
          v-if="canMerge"
          variant="secondary"
          icon="copy"
          :to="localePath('/admin/organisations/doublons')"
        >
          {{ t('admin.organization.list.duplicates.link') }}
          <span v-if="screen && screen.pending_duplicates > 0" class="ml-2">
            <UiBadge
              intent="warning"
              size="sm"
              :label="String(screen.pending_duplicates)"
            />
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
             croire que la plateforme se limite à ce qu'elle montre. -->
        <UiAlert
          v-if="screen?.scoped_to_events"
          class="mt-6"
          intent="info"
          compact
          :message="t('admin.organization.list.scoped.notice')"
        />

        <UiEmptyState
          v-if="allRows.length === 0 && status !== 'pending'"
          class="mt-8"
          icon="building"
          :title="t('admin.organization.list.empty.title')"
          :description="t('admin.organization.list.empty.description')"
        />

        <template v-else>
          <AdminOrganizationsFilters
            v-if="allRows.length > 2"
            class="mt-6"
            :filters="filters"
            :countries="screen?.countries ?? []"
            :types="screen?.types ?? []"
            :total="allRows.length"
            :shown="filteredRows.length"
            :disabled="status === 'pending'"
            @update:filters="setFilters"
          />

          <AdminOrganizationsTable
            class="mt-4"
            :rows="sortedRows"
            :caption="caption"
            :sort-key="sortKey"
            :sort-direction="sortDirection"
            :timezone="timezone"
            :loading="status === 'pending'"
            @sort="setSort"
            @open="openOrganization"
          >
            <template #empty>
              <!-- Aucun résultat APRÈS filtrage : offrir de retirer les filtres,
                   pas de laisser croire qu'il n'y a aucune organisation. -->
              <UiEmptyState
                icon="search"
                filtered
                :title="t('admin.organization.list.noResults.title')"
                :description="t('admin.organization.list.noResults.description', { total: allRows.length })"
                :action-label="t('admin.organization.list.noResults.action')"
                @action="setFilters(NO_ORGANIZATION_FILTERS)"
              />
            </template>
          </AdminOrganizationsTable>
        </template>
      </template>
    </template>
  </div>
</template>
