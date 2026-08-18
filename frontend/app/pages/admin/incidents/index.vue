<script setup lang="ts">
import type {
  IncidentFilters,
  IncidentListScreen,
  IncidentState,
  ManagedIncident,
} from '~/types/admin-incidents'
import type { IncidentScope, IncidentSeverity } from '~/types/live'
import type { EffectivePermission } from '~/types/identity'

/**
 * LES MESSAGES D'INCIDENT — `/admin/incidents`.
 *
 * L'ÉCRAN DU DIRECT. Une activité déborde, un intervenant tarde, une diffusion
 * s'interrompt : il faut le dire au public tout de suite, et le retirer dès que
 * c'est réglé. Tout ce qui est ici sert l'un ou l'autre de ces deux gestes.
 *
 * LA LISTE N'EST PAS TRIABLE — voir l'en-tête d'`AdminIncidentsTable`. Son ordre
 * est celui de l'action, et `live.event_incidents()` le rend déjà ainsi.
 *
 * DÉPUBLIER EST DANS LA LIGNE. Le bouton n'ouvre qu'un champ, facultatif : le
 * motif enrichit l'historique, il ne conditionne pas le retrait. La ligne ne
 * disparaît jamais — `live.incidents` n'a pas de suppression, seulement
 * `unpublished_at`, `unpublished_by` et `unpublish_reason`.
 *
 * UNE SEULE PERMISSION : `live.incident.publish`, SUR L'ÉDITION. Un compte
 * détaché sur la COP31 publie sur la COP31 et nulle part ailleurs. Sans elle,
 * l'écran reste consultable — lire ce qui est affiché au public n'est pas un
 * privilège —, mais aucune action n'est offerte.
 *
 * QUATRE ÉTATS : chargement (lignes squelettes), vide (aucun message, ce qui est
 * la situation NORMALE), erreur avec reprise, accès refusé hors périmètre.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.incidents' }],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()

useHead(() => ({ title: t('admin.incident.list.title') }))

await adminScope.ensureLoaded()

const {
  data: screen,
  status,
  error,
  refresh,
} = await useAsyncData<IncidentListScreen | null>(
  'admin-incidents',
  async () => {
    const eventId = adminScope.currentEventId
    if (!eventId) return null
    return api.adminIncidents.list(eventId, adminScope.scope)
  },
  { watch: [() => adminScope.currentEventId], lazy: true },
)

const { data: granted } = await useAsyncData<EffectivePermission[]>(
  'admin-incidents-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

/** La permission se vérifie SUR L'ÉDITION affichée, jamais globalement. */
const canPublish = computed(() =>
  hasPermission(granted.value, 'live.incident.publish', adminScope.currentEventId),
)

const timezone = computed(() => screen.value?.timezone ?? 'UTC')
const zoneLabel = computed(
  () => screen.value?.zone_label?.trim() || timeZoneCityLabel(timezone.value),
)

// ---------------------------------------------------------------------------
// Filtres — portés par l'URL
// ---------------------------------------------------------------------------

function queryList(value: unknown): string[] {
  if (Array.isArray(value)) return value.flatMap((entry) => String(entry).split(',')).filter(Boolean)
  if (typeof value === 'string') return value.split(',').filter(Boolean)
  return []
}

/** Les paramètres sont en FRANÇAIS : ils apparaissent dans une URL qu'on partage. */
const filters = computed<IncidentFilters>(() => ({
  search: typeof route.query.q === 'string' ? route.query.q : '',
  states: queryList(route.query.etat) as IncidentState[],
  severities: queryList(route.query.gravite) as IncidentSeverity[],
  scopes: queryList(route.query.portee) as IncidentScope[],
  kinds: queryList(route.query.nature),
}))

function setFilters(value: IncidentFilters): void {
  const next = { ...route.query }
  const patch: Record<string, string | null> = {
    q: value.search || null,
    etat: value.states.join(',') || null,
    gravite: value.severities.join(',') || null,
    portee: value.scopes.join(',') || null,
    nature: value.kinds.join(',') || null,
  }
  for (const [key, entry] of Object.entries(patch)) {
    if (entry === null) delete next[key]
    else next[key] = entry
  }
  router.replace({ query: next })
}

const allRows = computed<ManagedIncident[]>(() => screen.value?.rows ?? [])
const rows = computed(() => filterIncidents(allRows.value, filters.value))

// ---------------------------------------------------------------------------
// Écritures
// ---------------------------------------------------------------------------

const unpublishTarget = ref<ManagedIncident | null>(null)
const unpublishOpen = ref(false)
const submitting = ref(false)
const writeError = ref<string | null>(null)

function openUnpublish(row: ManagedIncident): void {
  unpublishTarget.value = row
  writeError.value = null
  unpublishOpen.value = true
}

async function unpublish(reason: string | null): Promise<void> {
  const target = unpublishTarget.value
  const eventId = adminScope.currentEventId
  if (!target || !eventId) return

  submitting.value = true
  writeError.value = null

  try {
    const result = await api.adminIncidents.unpublish(
      { incident_id: target.incident_id, reason },
      eventId,
      auth.person?.id ?? null,
      granted.value,
    )

    if (result.status !== 'unpublished') {
      writeError.value = t(`admin.incident.list.error.${result.status}`)
      return
    }

    unpublishOpen.value = false
    await refresh()
  } finally {
    submitting.value = false
  }
}

/** Publier un brouillon, ou rétablir un message retiré : le même appel. */
async function publish(row: ManagedIncident): Promise<void> {
  const eventId = adminScope.currentEventId
  if (!eventId) return

  submitting.value = true
  writeError.value = null

  try {
    const result = await api.adminIncidents.publish(
      row.incident_id,
      eventId,
      auth.person?.id ?? null,
      granted.value,
    )

    if (result.status !== 'published') {
      writeError.value = t(`admin.incident.list.error.${result.status}`)
      return
    }

    await refresh()
  } finally {
    submitting.value = false
  }
}

function openIncident(row: ManagedIncident): void {
  navigateTo(localePath(`/admin/incidents/${row.incident_id}`))
}
</script>

<template>
  <div class="mx-auto w-full max-w-[100rem]">
    <!-- ACCÈS REFUSÉ — aucune édition administrée. Distinct d'un écran vide. -->
    <UiForbiddenState
      v-if="!adminScope.isLoading && !adminScope.canAdminister"
      :required-scope="t('admin.incident.list.forbidden.scope')"
      action-to="/"
      :action-label="t('nav.admin.backToSite')"
    />

    <template v-else>
      <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">
            {{ t('admin.incident.list.title') }}
          </h1>
          <p class="mt-1 max-w-(--measure) text-text-muted">
            {{ t('admin.incident.list.subtitle') }}
          </p>
        </div>

        <UiButton
          v-if="canPublish"
          icon="plus"
          :to="localePath('/admin/incidents/nouveau')"
        >
          {{ t('admin.incident.list.new') }}
        </UiButton>
      </header>

      <UiErrorState
        v-if="error"
        class="mt-8"
        :retry-label="t('common.actions.retry')"
        @retry="refresh()"
      />

      <template v-else>
        <UiAlert v-if="writeError" class="mt-6" intent="danger" live :message="writeError" />

        <!-- CE QUI PARLE EN CE MOMENT, DIT EN CLAIR. Un tableau de sept lignes
             ne montre pas d'un coup d'œil qu'un bandeau rouge est en ligne. -->
        <UiAlert
          v-else-if="screen && screen.counts.active > 0"
          class="mt-6"
          intent="info"
          compact
          :message="t('admin.incident.list.counts.active', { count: screen.counts.active })"
        />

        <!-- LE POSTE DE DIRECT, EN TÊTE. C'est de là qu'on part presque toujours :
             une activité se tient, quelque chose ne va pas. La liste des
             messages, elle, sert à relire et à retirer. -->
        <AdminIncidentsLiveDesk
          v-if="screen"
          class="mt-6"
          :desk="screen.desk"
          :timezone="timezone"
          :zone-label="zoneLabel"
          :can-publish="canPublish"
        />

        <UiEmptyState
          v-if="allRows.length === 0 && status !== 'pending'"
          class="mt-8"
          icon="broadcast"
          :title="t('admin.incident.list.empty.title')"
          :description="t('admin.incident.list.empty.description')"
          :action-label="canPublish ? t('admin.incident.list.new') : undefined"
          @action="navigateTo(localePath('/admin/incidents/nouveau'))"
        />

        <template v-else>
          <AdminIncidentsFilters
            v-if="allRows.length > 2 && screen"
            class="mt-6"
            :filters="filters"
            :counts="screen.counts"
            :kinds="screen.kinds"
            :total="allRows.length"
            :shown="rows.length"
            :disabled="status === 'pending'"
            @update:filters="setFilters"
          />

          <AdminIncidentsTable
            class="mt-4"
            :rows="rows"
            :caption="t('admin.incident.list.caption')"
            :timezone="timezone"
            :kinds="screen?.kinds ?? []"
            :can-publish="canPublish"
            :loading="status === 'pending'"
            @open="openIncident"
            @publish="publish"
            @unpublish="openUnpublish"
          >
            <template #empty>
              <UiEmptyState
                icon="search"
                filtered
                :title="t('admin.incident.list.noResults.title')"
                :description="t('admin.incident.list.noResults.description', { total: allRows.length })"
                :action-label="t('admin.incident.list.noResults.action')"
                @action="setFilters(NO_INCIDENT_FILTERS)"
              />
            </template>
          </AdminIncidentsTable>
        </template>
      </template>

      <AdminIncidentsUnpublishDialog
        v-model:open="unpublishOpen"
        :incident="unpublishTarget"
        :submitting="submitting"
        :error="writeError"
        @submit="unpublish"
      />
    </template>
  </div>
</template>
