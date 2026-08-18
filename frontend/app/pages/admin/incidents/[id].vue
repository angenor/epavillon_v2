<script setup lang="ts">
import type { IncidentListScreen, IncidentPayload, ManagedIncident } from '~/types/admin-incidents'
import type { EffectivePermission } from '~/types/identity'

/**
 * MODIFIER UN MESSAGE D'INCIDENT — `/admin/incidents/<id>`.
 *
 * MÊME FORMULAIRE QUE LA PUBLICATION, chargé sur un message existant. Un
 * brouillon se relit et se corrige avant de parler ; un message en ligne se
 * corrige aussi — une heure de reprise qui glisse, une salle qui change encore.
 *
 * CE QUE CETTE PAGE MONTRE EN PLUS : la trace de publication. Qui a publié,
 * quand, qui a retiré et pourquoi. C'est l'historique que `live.incidents` porte
 * dans ses colonnes, et le seul endroit où on le lit en entier.
 *
 * REPUBLIER EFFACE LA DÉPUBLICATION, comme le fait `live.publish_incident()` en
 * remettant `unpublished_at`, `unpublished_by` et `unpublish_reason` à NULL. Un
 * message rétabli n'est pas un message qui reste marqué comme retiré.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.incidents', to: '/admin/incidents' }, { labelKey: 'admin.incident.form.titleEdit' }],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const route = useRoute()
const localePath = useLocalePath()
const { dateTime } = useDateTime()

useHead(() => ({ title: t('admin.incident.form.titleEdit') }))

await adminScope.ensureLoaded()

const incidentId = computed(() => String(route.params.id ?? ''))

const { data: screen, status } = await useAsyncData<IncidentListScreen | null>(
  'admin-incident-edit-screen',
  async () => {
    const eventId = adminScope.currentEventId
    if (!eventId) return null
    return api.adminIncidents.list(eventId, adminScope.scope)
  },
  { watch: [() => adminScope.currentEventId], lazy: true },
)

const { data: incident, refresh } = await useAsyncData<ManagedIncident | null>(
  'admin-incident-edit',
  async () => {
    const eventId = adminScope.currentEventId
    if (!eventId) return null
    return api.adminIncidents.byId(incidentId.value, eventId, adminScope.scope)
  },
  { default: () => null, watch: [incidentId, () => adminScope.currentEventId], lazy: true },
)

const { data: granted } = await useAsyncData<EffectivePermission[]>(
  'admin-incident-edit-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canPublish = computed(() =>
  hasPermission(granted.value, 'live.incident.publish', adminScope.currentEventId),
)

const timezone = computed(() => screen.value?.timezone ?? 'UTC')
const zoneLabel = computed(
  () => screen.value?.zone_label?.trim() || timeZoneCityLabel(timezone.value),
)

const submitting = ref(false)
const formError = ref<string | null>(null)

async function submit(payload: IncidentPayload): Promise<void> {
  const eventId = adminScope.currentEventId
  if (!eventId) return

  submitting.value = true
  formError.value = null

  try {
    const result = await api.adminIncidents.update(
      { ...payload, incident_id: incidentId.value, from_event_id: eventId },
      auth.person?.id ?? null,
      granted.value,
    )

    if (result.status !== 'updated' && result.status !== 'published') {
      formError.value = t(`admin.incident.form.error.${result.status}`)
      await refresh()
      return
    }

    await navigateTo(localePath('/admin/incidents'))
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <div class="mx-auto w-full max-w-[100rem]">
    <UiForbiddenState
      v-if="!adminScope.isLoading && !canPublish && status !== 'pending'"
      :required-scope="t('admin.incident.form.forbidden.scope')"
      action-to="/admin/incidents"
      :action-label="t('admin.incident.form.notFound.action')"
    />

    <template v-else>
      <UiLoadingState v-if="status === 'pending'" />

      <UiEmptyState
        v-else-if="!incident"
        icon="search"
        :title="t('admin.incident.form.notFound.title')"
        :description="t('admin.incident.form.notFound.description')"
        :action-label="t('admin.incident.form.notFound.action')"
        @action="navigateTo(localePath('/admin/incidents'))"
      />

      <template v-else>
        <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
          <div class="min-w-0">
            <h1 class="text-3xl leading-tight font-semibold text-balance">
              {{ t('admin.incident.form.titleEdit') }}
            </h1>
            <p class="mt-1 max-w-(--measure) text-text-muted">{{ t('admin.incident.form.subtitle') }}</p>
          </div>

          <AdminIncidentsStateBadge :state="incident.state" />
        </header>

        <!-- LA TRACE DE PUBLICATION — qui a parlé au public, quand, et qui a
             retiré le message. Trois lignes, pas un journal : ce que porte la
             table, rien de plus. -->
        <div class="mt-5 flex flex-wrap gap-x-6 gap-y-1 text-sm text-text-muted">
          <p v-if="incident.published_at">
            {{ incident.published_by_name
              ? t('admin.incident.list.row.publishedBy', {
                name: incident.published_by_name,
                date: dateTime(incident.published_at, timezone),
              })
              : t('admin.incident.list.row.publishedAt', {
                date: dateTime(incident.published_at, timezone),
              }) }}
          </p>
          <p v-else>{{ t('admin.incident.list.row.neverPublished') }}</p>

          <p v-if="incident.unpublished_at">
            {{ t('admin.incident.list.row.unpublishedBy', {
              name: incident.unpublished_by_name ?? '—',
              date: dateTime(incident.unpublished_at, timezone),
            }) }}
          </p>
          <p v-if="incident.unpublish_reason">
            {{ t('admin.incident.list.row.unpublishReason', { reason: incident.unpublish_reason }) }}
          </p>
        </div>

        <AdminIncidentsForm
          v-if="screen"
          class="mt-6"
          :incident="incident"
          :event-id="screen.event_id"
          :targets="screen.targets"
          :kinds="screen.kinds"
          :timezone="timezone"
          :zone-label="zoneLabel"
          :submitting="submitting"
          :error="formError"
          @submit="submit"
          @cancel="navigateTo(localePath('/admin/incidents'))"
        />
      </template>
    </template>
  </div>
</template>
