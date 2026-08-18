<script setup lang="ts">
import type {
  EditionListFilters,
  EditionListRow,
  EditionListScreen,
  EditionSortKey,
} from '~/types/admin-events'
import type { EffectivePermission } from '~/types/identity'
import type { EventStatus } from '~/types/event/edition'
import type { SortDirection } from '~/types/ui'

/**
 * LISTE DES ÉDITIONS — `/admin/evenements`.
 *
 * L'ÉCRAN OÙ L'ON VOIT L'ÉTAT D'UNE CAMPAGNE D'UN COUP D'ŒIL : quelles éditions
 * existent, laquelle tient un pavillon, laquelle a son appel ouvert, laquelle a
 * publié sa programmation. C'est la porte d'entrée du référentiel : les six onglets
 * d'une édition s'ouvrent d'ici.
 *
 * CET ÉCRAN N'EST PAS FILTRÉ PAR LE SÉLECTEUR D'ÉDITION de la tête de page — il
 * les LISTE. Le sélecteur choisit l'édition sur laquelle travaillent le tableau de
 * bord, les propositions et le planificateur ; ici, ce serait absurde, une liste à
 * une ligne. Le périmètre d'administration s'applique quand même, par l'autre bout :
 * `api.adminEvents.list(scope)` ne rend que les éditions administrées (règle
 * métier n° 8).
 *
 * UNE PERSONNE DÉTACHÉE SUR UNE SEULE ÉDITION ne voit qu'elle, et rien ne laisse
 * deviner les autres — ni compteur « 1 sur 4 », ni mention de périmètre. Le
 * back-office se lit comme s'il n'existait qu'une COP, parce que pour cette
 * personne c'est le cas.
 *
 * CHAQUE LIGNE PORTE SON PROPRE FUSEAU. C'est la particularité de cet écran :
 * ailleurs, tout s'affiche dans le fuseau d'une édition retenue ; ici, la COP31 se
 * lit en heure de Belém et le cycle PACO en heure de Paris, sur la même page.
 *
 * TOUT L'ÉTAT VIT DANS L'URL : filtres, tri. Une liste filtrée se transmet, et le
 * jour où le filtrage part au serveur (B3) ces paramètres deviennent ceux de la
 * requête.
 *
 * QUATRE ÉTATS : chargement (lignes squelettes), vide (aucune édition dans le
 * périmètre, distinct d'aucun résultat après filtrage), erreur avec reprise, accès
 * refusé.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.events' }],
})

const { t } = useI18n()
const { tr } = useI18nText()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()

useHead(() => ({ title: t('admin.event.list.title') }))

await adminScope.ensureLoaded()

const {
  data: screen,
  status,
  error,
  refresh,
} = await useAsyncData<EditionListScreen | null>(
  'admin-edition-list',
  async () => (adminScope.canAdminister ? api.adminEvents.list(adminScope.scope) : null),
  { watch: [() => adminScope.canAdminister], lazy: true },
)

/**
 * Créer une édition demande `event.event.manage` sur la portée GLOBALE : une
 * édition n'existe pas encore, elle n'a donc aucun périmètre où la vérifier. C'est
 * la seule action de cet écran dont la portée ne soit pas celle d'une édition.
 */
const { data: granted } = await useAsyncData<EffectivePermission[]>(
  'admin-edition-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canCreate = computed(() => hasPermission(granted.value, 'event.event.manage'))

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

/** `undefined` : sans importance. `oui` / `non` : les deux autres états. */
function queryTristate(value: unknown): boolean | null {
  const raw = queryText(value)
  if (raw === 'oui') return true
  if (raw === 'non') return false
  return null
}

/** Les paramètres sont en FRANÇAIS : ils apparaissent dans une URL qu'on partage. */
const filters = computed<EditionListFilters>(() => ({
  search: queryText(route.query.q),
  series: queryList(route.query.serie),
  years: queryList(route.query.annee).map(Number).filter(Number.isFinite),
  statuses: queryList(route.query.statut) as EventStatus[],
  has_pavilion: queryTristate(route.query.pavillon),
  published: queryTristate(route.query.programmation),
}))

const SORT_PARAM: Record<string, EditionSortKey> = {
  titre: 'title',
  serie: 'series',
  annee: 'edition_year',
  dates: 'starts_at',
  lieu: 'location',
  statut: 'status',
  propositions: 'proposal_count',
  programmation: 'programme',
}
const PARAM_BY_SORT = Object.fromEntries(
  Object.entries(SORT_PARAM).map(([param, key]) => [key, param]),
) as Record<EditionSortKey, string>

/**
 * Tri par défaut : les DATES, décroissantes. C'est l'édition en préparation qu'on
 * vient voir — ouvrir sur la COP29, close depuis deux ans, obligerait à trier avant
 * de pouvoir lire quoi que ce soit. Même raisonnement que le sélecteur d'édition.
 */
const sortKey = computed<EditionSortKey>(() => SORT_PARAM[queryText(route.query.tri)] ?? 'starts_at')
const sortDirection = computed<SortDirection>(() =>
  queryText(route.query.sens) === 'asc' ? 'asc' : 'desc',
)

function updateQuery(patch: Record<string, string | null>): void {
  const next = { ...route.query }
  for (const [key, value] of Object.entries(patch)) {
    if (value === null || value === '') delete next[key]
    else next[key] = value
  }
  router.replace({ query: next })
}

function setFilters(value: EditionListFilters): void {
  updateQuery({
    q: value.search || null,
    serie: value.series.join(',') || null,
    annee: value.years.join(',') || null,
    statut: value.statuses.join(',') || null,
    pavillon: value.has_pavilion === null ? null : value.has_pavilion ? 'oui' : 'non',
    programmation: value.published === null ? null : value.published ? 'oui' : 'non',
  })
}

function setSort(key: string, direction: Exclude<SortDirection, null>): void {
  const mapped = SORT_PARAM[key] ?? (key as EditionSortKey)
  updateQuery({
    tri: PARAM_BY_SORT[mapped] === 'dates' && direction === 'desc' ? null : PARAM_BY_SORT[mapped],
    sens: direction === 'asc' ? 'asc' : null,
  })
}

// ---------------------------------------------------------------------------
// Lignes affichées
// ---------------------------------------------------------------------------

const allRows = computed<EditionListRow[]>(() => screen.value?.rows ?? [])

const filteredRows = computed(() => filterEditions(allRows.value, filters.value))

const sortedRows = computed(() =>
  sortEditions(filteredRows.value, sortKey.value, sortDirection.value, (row) =>
    row.series_name ? tr(row.series_name) : '',
  ),
)

const caption = computed(() =>
  t('admin.event.list.caption', {
    column: t('admin.event.list.columns.' + sortKey.value),
  }),
)

function openEdition(row: EditionListRow): void {
  navigateTo(localePath(`/admin/evenements/${row.id}`))
}
</script>

<template>
  <div class="mx-auto w-full max-w-[100rem]">
    <!-- ACCÈS REFUSÉ — aucun droit d'administration. Distinct d'un écran vide. -->
    <UiForbiddenState
      v-if="!adminScope.isLoading && !adminScope.canAdminister"
      :required-scope="t('admin.event.list.forbidden.scope')"
      action-to="/"
      :action-label="t('nav.admin.backToSite')"
    />

    <template v-else>
      <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">
            {{ t('admin.event.list.title') }}
          </h1>
          <p class="mt-1 max-w-(--measure) text-text-muted">
            {{ t('admin.event.list.subtitle') }}
          </p>
        </div>

        <UiButton
          v-if="canCreate"
          :to="localePath('/admin/evenements/nouveau')"
          icon="plus"
        >
          {{ t('admin.event.list.create') }}
        </UiButton>
      </header>

      <UiErrorState
        v-if="error"
        class="mt-8"
        :retry-label="t('common.actions.retry')"
        @retry="refresh()"
      />

      <UiEmptyState
        v-else-if="allRows.length === 0 && status !== 'pending'"
        class="mt-8"
        icon="calendar"
        :title="t('admin.event.list.empty.title')"
        :description="t('admin.event.list.empty.description')"
      />

      <template v-else>
        <!-- Les filtres n'apparaissent qu'à partir de trois éditions : sur un
             périmètre restreint, six sélecteurs au-dessus d'une ligne unique
             n'apportent rien et laissent croire qu'il y a plus à voir. -->
        <AdminEventsEditionFilters
          v-if="allRows.length > 2"
          class="mt-6"
          :filters="filters"
          :series="screen?.series ?? []"
          :years="screen?.years ?? []"
          :total="allRows.length"
          :shown="filteredRows.length"
          :disabled="status === 'pending'"
          @update:filters="setFilters"
        />

        <AdminEventsEditionsTable
          class="mt-4"
          :rows="sortedRows"
          :caption="caption"
          :sort-key="sortKey"
          :sort-direction="sortDirection"
          :loading="status === 'pending'"
          @sort="setSort"
          @open="openEdition"
        >
          <template #empty>
            <!-- Aucun résultat APRÈS filtrage : offrir de retirer les filtres,
                 pas de laisser croire qu'il n'y a aucune édition. -->
            <UiEmptyState
              icon="search"
              :title="t('admin.event.list.noResults.title')"
              :description="t('admin.event.list.noResults.description', { total: allRows.length })"
              :action-label="t('admin.event.list.noResults.action')"
              @action="setFilters(NO_EDITION_FILTERS)"
            />
          </template>
        </AdminEventsEditionsTable>
      </template>
    </template>
  </div>
</template>
