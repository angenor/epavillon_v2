<script setup lang="ts">
import type { IncidentListScreen, IncidentPayload } from '~/types/admin-incidents'
import type { IncidentScope, IncidentSeverity } from '~/types/live'
import type { EffectivePermission } from '~/types/identity'

/**
 * PUBLIER UN MESSAGE D'INCIDENT — `/admin/incidents/nouveau`.
 *
 * PAGE ET NON TIROIR, POUR UNE RAISON PRÉCISE : l'aperçu en direct. Un bandeau
 * pleine largeur ne se juge pas dans un panneau de 28 rem — c'est justement ce
 * qu'on vient vérifier. La page tient les deux côte à côte sur écran large, et
 * l'un sous l'autre en dessous.
 *
 * LE RACCOURCI « SIGNALER UN DÉBORDEMENT » ARRIVE ICI, par l'URL :
 * `?portee=session&cible=<id>&nature=overrun`. Les paramètres sont en français
 * comme partout ailleurs, et ils PRÉ-REMPLISSENT sans rien décider : l'équipe
 * relit, ajuste l'heure de reprise, puis publie. Passer par l'URL plutôt que par
 * un état partagé a une conséquence utile — le raccourci fonctionne depuis
 * n'importe où, y compris un signet ou un message d'équipe.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.incidents', to: '/admin/incidents' }, { labelKey: 'admin.incident.form.titleNew' }],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const route = useRoute()
const localePath = useLocalePath()

useHead(() => ({ title: t('admin.incident.form.titleNew') }))

await adminScope.ensureLoaded()

const { data: screen, status } = await useAsyncData<IncidentListScreen | null>(
  'admin-incident-new',
  async () => {
    const eventId = adminScope.currentEventId
    if (!eventId) return null
    return api.adminIncidents.list(eventId, adminScope.scope)
  },
  { watch: [() => adminScope.currentEventId], lazy: true },
)

const { data: granted } = await useAsyncData<EffectivePermission[]>(
  'admin-incident-new-permissions',
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

// ---------------------------------------------------------------------------
// Le pré-remplissage venu de l'URL
// ---------------------------------------------------------------------------

const SCOPES: IncidentScope[] = ['global', 'event', 'event_day', 'session', 'organization']

function queryText(value: unknown): string {
  return typeof value === 'string' ? value : ''
}

/**
 * L'activité visée par un raccourci du poste de direct, avec son titre et son
 * créneau.
 *
 * Elle sert deux choses : nommer l'activité dans le bandeau d'information de la
 * page, et proposer une fin d'affichage sensée — un incident de séance ne dure
 * pas la journée, il se termine avec le créneau qu'il perturbe. L'équipe la
 * rallonge d'un geste si la panne s'éternise.
 */
const { data: session } = await useAsyncData(
  'admin-incident-session',
  async () => {
    const sessionId = queryText(route.query.cible)
    if (queryText(route.query.portee) !== 'session' || !sessionId) return null
    return api.adminIncidents.overrunTemplate(sessionId)
  },
  { default: () => null, watch: [() => route.query.cible], lazy: true },
)

/**
 * La gravité que le poste de direct suggère, par nature.
 *
 * Elle SUGGÈRE, elle ne décide pas : le formulaire la laisse changer. Un retard
 * et un débordement demandent attention sans être des échecs — jaune ; une panne
 * et une diffusion coupée sont des échecs — rouge. C'est la règle de couleur du
 * guide de style, appliquée là où elle se joue.
 */
const SUGGESTED_SEVERITY: Record<string, IncidentSeverity> = {
  delay: 'warning',
  overrun: 'warning',
  technical_issue: 'error',
  connection_issue: 'error',
}

const prefill = computed<Partial<IncidentPayload> | null>(() => {
  const scope = queryText(route.query.portee)
  if (!SCOPES.includes(scope as IncidentScope)) return null

  const target = queryText(route.query.cible)
  const kind = queryText(route.query.nature)

  return {
    scope: scope as IncidentScope,
    event_day_id: scope === 'event_day' ? target || null : null,
    session_id: scope === 'session' ? target || null : null,
    organization_id: scope === 'organization' ? target || null : null,
    incident_kind_code: kind || undefined,
    severity: SUGGESTED_SEVERITY[kind],
    // La fin du créneau perturbé : l'équipe la corrige si la panne s'allonge.
    display_until: session.value?.ends_at ?? null,
  } as Partial<IncidentPayload>
})

// ---------------------------------------------------------------------------
// Enregistrement
// ---------------------------------------------------------------------------

const submitting = ref(false)
const formError = ref<string | null>(null)

async function submit(payload: IncidentPayload): Promise<void> {
  const eventId = adminScope.currentEventId
  if (!eventId) return

  submitting.value = true
  formError.value = null

  try {
    const result = await api.adminIncidents.create(
      { ...payload, from_event_id: eventId },
      auth.person?.id ?? null,
    )

    if (result.status !== 'created' && result.status !== 'published') {
      formError.value = t(`admin.incident.form.error.${result.status}`)
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
      <header class="min-w-0">
        <h1 class="text-3xl leading-tight font-semibold text-balance">
          {{ t('admin.incident.form.titleNew') }}
        </h1>
        <p class="mt-1 max-w-(--measure) text-text-muted">{{ t('admin.incident.form.subtitle') }}</p>
      </header>

      <!-- LE RACCOURCI SE DIT. Arriver sur un formulaire déjà rempli sans savoir
           pourquoi est la meilleure façon de publier le mauvais message. -->
      <UiAlert
        v-if="session"
        class="mt-6"
        intent="info"
        compact
        :message="t('admin.incident.form.overrunNotice', { session: session.title })"
      />

      <UiLoadingState v-if="status === 'pending'" class="mt-8" />

      <AdminIncidentsForm
        v-else-if="screen"
        class="mt-6"
        :event-id="screen.event_id"
        :targets="screen.targets"
        :kinds="screen.kinds"
        :timezone="timezone"
        :zone-label="zoneLabel"
        :prefill="prefill"
        :submitting="submitting"
        :error="formError"
        @submit="submit"
        @cancel="navigateTo(localePath('/admin/incidents'))"
      />
    </template>
  </div>
</template>
