<script setup lang="ts">
import type { OrganizationVisits, VisitEvent, VisitMember } from '~/types/admin-organizations'
import type { Intent, SelectOption } from '~/types/ui'
import type { TimeZoneName } from '~/types/shared'

/**
 * L'ONGLET « VISITES » (02/11/2026). Répond à « quelqu'un de cette organisation
 * est-il venu dans son espace, et a-t-il été bloqué ? » en croisant connexions,
 * pages ouvertes, modifications et refus. Lu à l'ouverture de l'onglet
 * seulement : la fiche n'a pas à payer cette lecture.
 */

interface Props {
  organizationId: string
  timezone: TimeZoneName
}

const props = defineProps<Props>()

const { t, te } = useI18n()
const { tr } = useI18nText()
const api = useApi()
const localePath = useLocalePath()
const { date, dateTime, dayLong, time, zoneLabel } = useDateTime()

const days = ref('30')
const person = ref('')

const { data, status, error, refresh } = await useAsyncData<OrganizationVisits | null>(
  () => `admin-organization-visits-${props.organizationId}-${days.value}`,
  () => api.adminOrganizations.visits(props.organizationId, Number(days.value)),
  { lazy: true, watch: [days] },
)

const periodOptions = computed<SelectOption[]>(() => [
  { value: '7', label: t('admin.organization.detail.visits.period.days', { n: 7 }) },
  { value: '30', label: t('admin.organization.detail.visits.period.days', { n: 30 }) },
  { value: '0', label: t('admin.organization.detail.visits.period.all') },
])

const members = computed<VisitMember[]>(() => data.value?.members ?? [])
const nameOf = computed(() => new Map(members.value.map((m) => [m.person_id, m.display_name])))
const personOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.organization.detail.visits.allMembers') },
  ...members.value.map((m) => ({ value: m.person_id, label: m.display_name })),
])

const events = computed<VisitEvent[]>(() =>
  (data.value?.events ?? []).filter((e) => !person.value || e.person_id === person.value),
)

const latest = <T,>(rows: T[], at: (row: T) => string | null): T | null =>
  rows.reduce<T | null>((best, row) => {
    const value = at(row)
    return value && (!best || value > (at(best) ?? '')) ? row : best
  }, null)

const lastVisitor = computed(() => latest(members.value, (m) => m.last_visit_at))
const lastConnected = computed(() => latest(members.value, (m) => m.last_connection_at))
const refusals = computed(() => (data.value?.events ?? []).filter((e) => e.kind === 'refusal').length)
const hasVisits = computed(() => (data.value?.events ?? []).some((e) => e.kind === 'visit'))

/** Jour local de chaque visite, par personne : une connexion sans visite ce jour-là le dit. */
const visitedDays = computed(() => {
  const seen = new Set<string>()
  for (const e of data.value?.events ?? []) {
    if (e.kind === 'visit') seen.add(`${e.person_id}|${dayKeyInZone(e.at, props.timezone)}`)
  }
  return seen
})
const visitedThatDay = (personId: string, at: string) =>
  visitedDays.value.has(`${personId}|${dayKeyInZone(at, props.timezone)}`)

const lastConnectionNote = computed(() => {
  const m = lastConnected.value
  if (!m?.last_connection_at) return ''
  return visitedThatDay(m.person_id, m.last_connection_at)
    ? m.display_name
    : t('admin.organization.detail.visits.summary.withoutSpace', { name: m.display_name })
})

const dayGroups = computed(() => {
  const groups: { key: string; at: string; events: VisitEvent[] }[] = []
  for (const e of events.value) {
    const key = dayKeyInZone(e.at, props.timezone)
    const last = groups[groups.length - 1]
    if (last?.key === key) last.events.push(e)
    else groups.push({ key, at: e.at, events: [e] })
  }
  return groups
})

/** Début de ce qu'on peut affirmer : la fenêtre, ou le début de l'enregistrement s'il est plus récent. */
const knownSince = computed(() => {
  const since = data.value?.recording_since
  const start = data.value?.window_start
  if (!since) return null
  return start && start > since ? start : since
})

const KIND_INTENT: Record<VisitEvent['kind'], Intent> = {
  connection: 'neutral',
  visit: 'info',
  change: 'success',
  refusal: 'warning',
}

function hours(e: VisitEvent): string {
  return e.until
    ? t('admin.organization.detail.visits.range', { start: time(e.at, props.timezone), end: time(e.until, props.timezone) })
    : time(e.at, props.timezone)
}

function roleLabel(role: string): string {
  const key = `admin.organization.detail.members.role.${role}`
  return te(key) ? t(key) : t('admin.organization.detail.visits.former')
}

function tableLabel(table: string): string {
  const key = `admin.organization.detail.visits.tables.${table}`
  return te(key) ? t(key) : table
}

function changeText(e: VisitEvent): string {
  const what = [...new Set(e.tables.map(tableLabel))].join(', ')
  return e.proposal_reference
    ? t('admin.organization.detail.visits.change.proposal', { reference: e.proposal_reference, what })
    : what
}

const proposalLink = (id: string) => localePath(`/admin/propositions/${id}`)
const journalLink = (requestId: string) => localePath({ path: '/admin/journal', query: { demande: requestId } })
</script>

<template>
  <section class="space-y-8">
    <div v-if="status === 'pending' && !data" class="space-y-3">
      <UiSkeletonLoader height="5rem" />
      <UiSkeletonLoader height="12rem" />
    </div>

    <UiErrorState
      v-else-if="error"
      :description="t('admin.organization.detail.visits.error')"
      @retry="refresh()"
    />

    <template v-else-if="data">
      <div class="grid gap-3 md:grid-cols-3">
        <div class="rounded-lg border border-border p-4">
          <p class="text-sm text-text-secondary">{{ t('admin.organization.detail.visits.summary.lastVisit') }}</p>
          <p class="mt-1 text-lg font-semibold tabular-nums">
            {{ lastVisitor?.last_visit_at ? dateTime(lastVisitor.last_visit_at, timezone) : t('admin.organization.detail.visits.never') }}
          </p>
          <p v-if="lastVisitor" class="text-sm text-text-secondary">
            {{ lastVisitor.display_name }}<template v-if="lastVisitor.last_visit_reference">, {{ lastVisitor.last_visit_reference }}</template>
          </p>
        </div>
        <div class="rounded-lg border border-border p-4">
          <p class="text-sm text-text-secondary">{{ t('admin.organization.detail.visits.summary.lastConnection') }}</p>
          <p class="mt-1 text-lg font-semibold tabular-nums">
            {{ lastConnected?.last_connection_at ? dateTime(lastConnected.last_connection_at, timezone) : t('admin.organization.detail.visits.never') }}
          </p>
          <p v-if="lastConnectionNote" class="text-sm text-text-secondary">{{ lastConnectionNote }}</p>
        </div>
        <div
          class="rounded-lg border p-4"
          :class="refusals > 0 ? 'border-warning-border bg-warning-surface' : 'border-border'"
        >
          <p class="text-sm text-text-secondary">{{ t('admin.organization.detail.visits.summary.refusals') }}</p>
          <p class="mt-1 text-lg font-semibold tabular-nums" :class="refusals > 0 && 'text-warning'">
            {{ t('admin.organization.detail.visits.summary.refusalCount', { n: refusals }, refusals) }}
          </p>
        </div>
      </div>

      <div>
        <h2 class="text-lg font-semibold">{{ t('admin.organization.detail.visits.members.title') }}</h2>
        <p class="mb-3 text-sm text-text-secondary">{{ zoneLabel(timezone) }}</p>
        <div class="overflow-x-auto rounded-lg border border-border">
          <table class="w-full text-sm">
            <thead class="bg-surface-sunken text-left text-xs uppercase tracking-wide text-text-secondary">
              <tr>
                <th class="px-3 py-2">{{ t('admin.organization.detail.visits.members.person') }}</th>
                <th class="px-3 py-2">{{ t('admin.organization.detail.visits.members.lastConnection') }}</th>
                <th class="px-3 py-2">{{ t('admin.organization.detail.visits.members.lastVisit') }}</th>
                <th class="px-3 py-2">{{ t('admin.organization.detail.visits.members.visitDays') }}</th>
                <th class="px-3 py-2">{{ t('admin.organization.detail.visits.members.changes') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="m in members" :key="m.person_id" class="border-t border-border-subtle align-top tabular-nums">
                <td class="px-3 py-3">
                  <span class="font-semibold">{{ m.display_name }}</span>
                  <span class="block text-xs text-text-muted">{{ roleLabel(m.role) }}</span>
                </td>
                <td class="px-3 py-3" :class="!m.last_connection_at && 'text-text-muted'">
                  {{ m.last_connection_at ? dateTime(m.last_connection_at, timezone) : t('admin.organization.detail.visits.neverConnected') }}
                </td>
                <td class="px-3 py-3" :class="!m.last_visit_at && 'text-text-muted'">
                  {{ m.last_visit_at ? dateTime(m.last_visit_at, timezone) : t('admin.organization.detail.visits.never') }}
                  <span v-if="m.last_visit_at" class="block text-xs text-text-muted">
                    {{ m.last_visit_reference ?? t('admin.organization.detail.visits.space') }}
                  </span>
                </td>
                <td class="px-3 py-3" :class="m.visit_days === 0 && 'text-text-muted'">{{ m.visit_days }}</td>
                <td class="px-3 py-3" :class="m.changes === 0 && 'text-text-muted'">
                  {{ m.changes || t('admin.organization.detail.visits.none') }}
                  <span v-if="m.last_change_at" class="block text-xs text-text-muted">
                    {{ t('admin.organization.detail.visits.members.lastChange', { date: date(m.last_change_at, timezone) }) }}
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <div>
        <h2 class="mb-3 text-lg font-semibold">{{ t('admin.organization.detail.visits.timeline') }}</h2>
        <div class="mb-4 flex flex-wrap gap-2">
          <UiSelect v-model="days" :options="periodOptions" size="sm" :label="t('admin.organization.detail.visits.period.label')" hide-optional class="w-48" />
          <UiSelect v-model="person" :options="personOptions" size="sm" :label="t('admin.organization.detail.visits.person')" hide-optional class="w-60" />
        </div>

        <div v-if="!hasVisits" class="mb-6 rounded-lg border border-dashed border-border-strong p-6 text-center text-text-secondary">
          <p class="font-semibold text-text">
            {{ knownSince
              ? t('admin.organization.detail.visits.empty.since', { date: dayLong(knownSince, timezone) })
              : t('admin.organization.detail.visits.empty.never') }}
          </p>
          <p class="mt-1 text-sm">{{ t('admin.organization.detail.visits.empty.hint') }}</p>
        </div>

        <div v-for="day in dayGroups" :key="day.key" class="mb-5">
          <h3 class="mb-2 border-b border-border pb-1 font-semibold first-letter:uppercase">{{ dayLong(day.at, timezone) }}</h3>
          <ol>
            <li
              v-for="(e, i) in day.events"
              :key="`${e.kind}-${e.at}-${i}`"
              class="grid gap-x-3 gap-y-0.5 border-b border-dashed border-border-subtle py-2 text-sm last:border-0 md:grid-cols-[8rem_11rem_minmax(0,1fr)]"
            >
              <span class="whitespace-nowrap tabular-nums text-text-secondary">{{ hours(e) }}</span>
              <span class="font-semibold">{{ nameOf.get(e.person_id) ?? '' }}</span>
              <span>
                <UiBadge
                  :intent="e.kind === 'refusal' && (e.status ?? 0) >= 500 ? 'danger' : KIND_INTENT[e.kind]"
                  size="sm"
                  solid
                  class="mr-2"
                  :label="t(`admin.organization.detail.visits.kind.${e.kind === 'refusal' && (e.status ?? 0) >= 500 ? 'failure' : e.kind}`)"
                />
                <template v-if="e.kind === 'connection'">
                  <span class="text-text-secondary">
                    {{ visitedThatDay(e.person_id, e.at) ? e.device : t('admin.organization.detail.visits.withoutSpace') }}
                  </span>
                </template>
                <template v-else-if="e.kind === 'visit'">
                  <template v-if="e.proposal_id">
                    <NuxtLink :to="proposalLink(e.proposal_id)" class="font-semibold text-text-link hover:underline">
                      {{ t('admin.organization.detail.visits.proposal', { reference: e.proposal_reference ?? '' }) }}
                    </NuxtLink>
                    <template v-if="e.proposal_title"> « {{ tr(e.proposal_title) }} »</template>
                  </template>
                  <template v-else>{{ t('admin.organization.detail.visits.space') }}</template>
                  <span class="text-text-secondary"> · {{ t('admin.organization.detail.visits.times', { n: e.count ?? 1 }, e.count ?? 1) }}</span>
                </template>
                <template v-else-if="e.kind === 'change'">{{ changeText(e) }}</template>
                <template v-else>
                  <span class="mt-1 block rounded-md bg-warning-surface px-3 py-2">
                    {{ t('admin.organization.detail.visits.refusal', { message: e.message ?? e.status }) }}
                    <NuxtLink v-if="e.request_id" :to="journalLink(e.request_id)" class="font-semibold text-text-link hover:underline">
                      {{ t('admin.organization.detail.visits.openJournal') }}
                    </NuxtLink>
                  </span>
                </template>
              </span>
            </li>
          </ol>
        </div>
      </div>
    </template>
  </section>
</template>
