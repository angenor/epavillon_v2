<script setup lang="ts">
import type { ScheduledMailBatchRow, ScheduledMailScreen, ScheduledMailType } from '~/types/admin-scheduled-mails'
import type { EffectivePermission } from '~/types/identity'
import type { TableColumn } from '~/types/ui'

/**
 * LES COURRIELS PROGRAMMÉS, `/admin/courriels/programmes` (04/11, maquette
 * `docs/maquettes/courriels-a-venir/`). Les rappels déjà en file de l'édition :
 * le prochain départ décompté à la seconde, puis les types (rappel de journée,
 * rappels d'activité par décalage) qui se déplient en envois. « Voir » ouvre
 * le panneau d'un envoi : son texte, celui du type, et ce qui partira.
 *
 * Le décompte se cale sur l'heure du serveur (`observed_at`) : l'écart avec
 * l'horloge du poste est mesuré à chaque relevé.
 *
 * Gardé par `engagement.mailing.send` SUR L'ÉDITION choisie (règle n° 8).
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.mailings', to: '/admin/courriels' }, { labelKey: 'admin.mailing.scheduled.crumb' }],
})

const { t, locale } = useI18n()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const { intlLocale } = useDateTime()

useHead(() => ({ title: t('admin.mailing.scheduled.title') }))

await adminScope.ensureLoaded()

const { data: granted, status: grantStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-scheduled-mails-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canSend = computed(() =>
  hasPermission(granted.value, 'engagement.mailing.send', adminScope.currentEventId),
)

// --- L'heure du serveur ---------------------------------------------------------
const offsetMs = ref(0)
const now = ref(Date.now())
let ticker: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  ticker = setInterval(() => {
    now.value = Date.now() + offsetMs.value
  }, 1000)
})
onBeforeUnmount(() => {
  if (ticker) clearInterval(ticker)
})

const {
  data: screen,
  error,
  refresh,
} = await useAsyncData<ScheduledMailScreen | null>(
  'admin-scheduled-mails',
  async () => {
    const eventId = adminScope.currentEventId
    if (!eventId || !canSend.value) return null
    const fresh = await api.adminMailings.scheduled.list(eventId, adminScope.scope)
    offsetMs.value = clockOffsetMs(fresh.observed_at, Date.now())
    now.value = Date.now() + offsetMs.value
    return fresh
  },
  { watch: [() => adminScope.currentEventId, canSend, locale], lazy: true, server: false },
)

const types = computed<ScheduledMailType[]>(() => screen.value?.types ?? [])
const timezone = computed(() => adminScope.currentEvent?.timezone ?? 'UTC')
const zone = computed(() => adminScope.currentEvent?.city || timeZoneCityLabel(timezone.value))

// Le départ passé, la liste se relit : l'envoi parti n'y est plus.
const nextParts = computed(() => (screen.value?.next ? countdownParts(screen.value.next.run_at, now.value) : null))
watch(
  () => nextParts.value?.total === 0,
  (gone) => {
    if (gone) setTimeout(() => void refresh(), 5000)
  },
)

// --- Libellés -------------------------------------------------------------------
function offsetLabel(minutes: number): string {
  const { unit, count } = offsetUnit(minutes)
  return t(`admin.mailing.scheduled.offset.${unit}`, { count }, count)
}

function typeTitle(type: ScheduledMailType): string {
  return type.kind === 'day'
    ? t('admin.mailing.scheduled.type.day')
    : t('admin.mailing.scheduled.type.activity', { offset: offsetLabel(type.offset_minutes ?? 0) })
}

function nextTitle(row: ScheduledMailBatchRow): string {
  const offset = offsetLabel(row.offset_minutes)
  return row.kind === 'day'
    ? t('admin.mailing.scheduled.next.dayType', { offset })
    : t('admin.mailing.scheduled.type.activity', { offset })
}

function batchLabel(row: ScheduledMailBatchRow): string {
  return t('admin.mailing.scheduled.batches.label', { title: row.title, offset: offsetLabel(row.offset_minutes) })
}

function delay(target: string, withSeconds: boolean): string {
  const p = countdownParts(target, now.value)
  if (p.total === 0) return t('admin.mailing.scheduled.delay.now')
  const parts = [
    p.days ? t('admin.mailing.scheduled.delay.d', { n: p.days }) : '',
    p.days || p.hours ? t('admin.mailing.scheduled.delay.h', { n: p.hours }) : '',
    t('admin.mailing.scheduled.delay.m', { n: p.minutes }),
    withSeconds ? t('admin.mailing.scheduled.delay.s', { n: p.seconds }) : '',
  ].filter(Boolean)
  return t('admin.mailing.scheduled.delay.in', { delay: parts.join(' ') })
}

function when(value: string): string {
  return shortDayTime(value, timezone.value, intlLocale.value)
}

function pad(value: number): string {
  return String(value).padStart(2, '0')
}

// --- Types dépliés, envoi choisi --------------------------------------------------
const expanded = ref<Set<string>>(new Set())
watch(
  types,
  (list) => {
    if (expanded.value.size === 0 && list[0]) expanded.value = new Set([list[0].key])
  },
  { immediate: true },
)

function toggle(key: string): void {
  const next = new Set(expanded.value)
  if (next.has(key)) next.delete(key)
  else next.add(key)
  expanded.value = next
}

const selected = ref<{ key: string; scope: 'type' | 'batch' } | null>(null)
const panel = ref<HTMLElement | null>(null)

const selectedRow = computed<ScheduledMailBatchRow | null>(() => {
  const key = selected.value?.key
  if (!key) return null
  for (const type of types.value) {
    const row = type.batches.find((b) => b.key === key)
    if (row) return row
  }
  return null
})

async function open(row: ScheduledMailBatchRow, scope: 'type' | 'batch'): Promise<void> {
  selected.value = { key: row.key, scope }
  if (!expanded.value.has(row.type_key)) toggle(row.type_key)
  await nextTick()
  panel.value?.scrollIntoView({ behavior: 'smooth', block: 'start' })
}

function editType(type: ScheduledMailType): void {
  const first = type.batches[0]
  if (first) void open(first, 'type')
}

const columns = computed<TableColumn[]>(() => [
  { key: 'departure', label: t('admin.mailing.scheduled.batches.departure'), width: '12rem' },
  { key: 'batch', label: t('admin.mailing.scheduled.batches.batch') },
  { key: 'recipients', label: t('admin.mailing.scheduled.batches.recipients'), hideOnMobile: true, align: 'end' },
  { key: 'action', label: t('admin.mailing.scheduled.batches.action'), align: 'end', width: '5.5rem' },
])

const isLoadingAccess = computed(() => adminScope.isLoading || grantStatus.value === 'pending')
const scheduledCount = computed(() => screen.value?.total ?? null)
</script>

<template>
  <div class="mx-auto w-full max-w-[100rem]">
    <UiForbiddenState
      v-if="!adminScope.isLoading && !adminScope.canAdminister"
      :required-scope="t('admin.mailing.list.forbidden')"
      action-to="/"
      :action-label="t('nav.admin.backToSite')"
    />

    <template v-else>
      <header class="min-w-0">
        <h1 class="text-3xl leading-tight font-semibold text-balance">{{ t('admin.mailing.scheduled.title') }}</h1>
        <p class="mt-1 max-w-(--measure) text-text-muted">
          {{ t('admin.mailing.scheduled.subtitle', { zone: t(zoneFormulaKey('zoneOf', zone), { zone }) }) }}
        </p>
      </header>

      <AdminMailingsSectionTabs v-if="canSend" class="mt-6" current="scheduled" :scheduled-count="scheduledCount" />

      <UiLoadingState v-if="isLoadingAccess" class="mt-8" variant="text" :lines="4" />

      <UiForbiddenState
        v-else-if="!canSend"
        class="mt-8"
        :required-scope="t('admin.mailing.list.forbidden')"
        action-to="/admin"
        :action-label="t('admin.mailing.common.backToDashboard')"
      />

      <UiErrorState v-else-if="error" class="mt-8" :description="apiErrorMessage(error, t)" @retry="refresh()" />

      <div v-else-if="!screen" class="mt-8 grid gap-3">
        <UiSkeletonLoader v-for="n in 4" :key="n" height="5rem" />
      </div>

      <UiEmptyState
        v-else-if="types.length === 0"
        class="mt-8"
        icon="mail"
        :title="t('admin.mailing.scheduled.empty.title')"
        :description="t('admin.mailing.scheduled.empty.description')"
      />

      <template v-else>
        <section
          v-if="screen.next && nextParts"
          class="mt-8 flex flex-wrap items-center justify-between gap-x-8 gap-y-4 rounded-lg border border-border bg-surface-raised px-4 py-5 shadow-xs sm:px-6"
          :aria-label="t('admin.mailing.scheduled.next.label')"
        >
          <div class="min-w-0">
            <p class="text-sm text-text-muted">{{ t('admin.mailing.scheduled.next.label') }}</p>
            <p class="mt-0.5 text-lg font-semibold text-heading">
              {{ nextTitle(screen.next) }}
            </p>
            <p class="text-sm text-text-secondary">
              {{ t('admin.mailing.scheduled.next.detail', { title: screen.next.title, count: screen.next.recipients, when: when(screen.next.run_at) }, screen.next.recipients) }}
            </p>
          </div>
          <div class="flex gap-2 sm:gap-2.5" role="timer" aria-live="off" :aria-label="t('admin.mailing.scheduled.next.timer')">
            <div
              v-for="unit in (['days', 'hours', 'minutes', 'seconds'] as const)"
              :key="unit"
              class="min-w-14 rounded-md bg-surface-sunken px-2 py-2 text-center sm:min-w-16"
            >
              <b class="block text-2xl leading-none font-bold text-heading tabular-nums sm:text-3xl">
                {{ unit === 'days' ? nextParts.days : pad(nextParts[unit]) }}
              </b>
              <small class="mt-1 block text-xs text-text-muted">{{ t(`admin.mailing.scheduled.next.${unit}`) }}</small>
            </div>
          </div>
        </section>

        <div class="mt-8 grid gap-3">
          <article
            v-for="type in types"
            :key="type.key"
            class="overflow-hidden rounded-lg border border-border bg-surface-raised shadow-xs"
          >
            <header class="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-x-6 gap-y-3 px-4 py-3.5 sm:px-5 lg:grid-cols-[minmax(0,1fr)_auto_auto_auto]">
              <div class="flex min-w-0 items-start gap-2">
                <UiButton
                  variant="ghost"
                  :icon="expanded.has(type.key) ? 'chevron-up' : 'chevron-down'"
                  icon-only
                  :label="t(expanded.has(type.key) ? 'admin.mailing.scheduled.type.hide' : 'admin.mailing.scheduled.type.show', { type: typeTitle(type) })"
                  :aria-expanded="expanded.has(type.key)"
                  @click="toggle(type.key)"
                />
                <div class="min-w-0 pt-2">
                  <h2 class="text-base font-semibold text-heading">{{ typeTitle(type) }}</h2>
                  <p v-if="type.kind === 'day'" class="text-sm text-text-muted">{{ t('admin.mailing.scheduled.type.dayDescription') }}</p>
                  <p v-if="type.has_custom_text" class="text-xs text-text-subtle">{{ t('admin.mailing.scheduled.type.custom') }}</p>
                </div>
              </div>
              <p class="text-right text-sm text-text-secondary">
                {{ t('admin.mailing.scheduled.type.count', { count: type.count }, type.count) }}
              </p>
              <p class="text-sm text-text-secondary lg:text-right">
                {{ t('admin.mailing.scheduled.type.next') }}
                <b class="block font-semibold text-text tabular-nums">{{ delay(type.next_run_at, false) }}</b>
              </p>
              <div class="col-span-full lg:col-span-1">
                <UiButton variant="secondary" icon="edit" @click="editType(type)">
                  {{ t('admin.mailing.scheduled.type.edit') }}
                </UiButton>
              </div>
            </header>

            <div v-if="expanded.has(type.key)" class="border-t border-border-subtle">
              <UiTable
                :columns="columns"
                :rows="type.batches"
                row-key="key"
                row-label-key="title"
                :caption="t('admin.mailing.scheduled.batches.caption', { type: typeTitle(type) })"
                visually-hidden-caption
                @row-click="(row: ScheduledMailBatchRow) => open(row, 'batch')"
              >
                <template #cell-departure="{ row }">
                  <span class="block font-semibold tabular-nums" :class="row.key === selected?.key ? 'text-accent' : ''">{{ when(row.run_at) }}</span>
                  <span class="block text-xs text-text-muted tabular-nums">{{ delay(row.run_at, true) }}</span>
                </template>
                <template #cell-batch="{ row }">
                  <span class="text-sm">{{ batchLabel(row) }}</span>
                  <UiBadge v-if="row.has_own_text" class="ml-1.5" intent="warning" solid :label="t('admin.mailing.scheduled.batches.ownText')" />
                </template>
                <template #cell-recipients="{ row }">
                  <span class="text-sm tabular-nums">{{ row.recipients }}</span>
                </template>
                <template #cell-action="{ row }">
                  <UiButton
                    variant="link"
                    :aria-label="t('admin.mailing.scheduled.batches.viewFor', { label: batchLabel(row) })"
                    @click.stop="open(row, 'batch')"
                  >
                    {{ t('admin.mailing.scheduled.batches.view') }}
                  </UiButton>
                </template>
              </UiTable>
            </div>
          </article>
        </div>

        <div v-if="selected && selectedRow && adminScope.currentEventId" ref="panel" class="mt-8 scroll-mt-24">
          <AdminMailingsScheduledPanel
            :batch-key="selected.key"
            :event-id="adminScope.currentEventId"
            :initial-scope="selected.scope"
            :when="when(selectedRow.run_at)"
            :label="t('admin.mailing.scheduled.panelLabel', { title: selectedRow.title, offset: offsetLabel(selectedRow.offset_minutes) })"
            @saved="refresh()"
            @close="selected = null"
          />
        </div>
      </template>
    </template>
  </div>
</template>
