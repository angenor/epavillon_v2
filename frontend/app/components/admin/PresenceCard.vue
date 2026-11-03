<script setup lang="ts">
import type { AdminPresence } from '~/types/admin-presence'
import { presenceBarWidth, secondsSince, sparklinePaths } from '~/utils/presence'

/**
 * « EN CE MOMENT » : combien de personnes sur la plateforme, et où. Relu toutes
 * les 30 s tant que la page est visible. Monté seulement pour qui détient
 * `analytics.presence.read` ; un 403 survenu depuis le retire.
 *
 * Une panne ne montre jamais un zéro : la carte dit « momentanément
 * indisponible », dès le premier échec sans relevé, ou quand le dernier relevé
 * a plus d'une minute et demie.
 */

const REFRESH_MS = 30_000
const STALE_S = 90
const CHART_W = 600
const CHART_H = 56

const { t } = useI18n()
const { timeWithZone } = useDateTime()
const api = useApi()

const presence = ref<AdminPresence | null>(null)
const receivedAt = ref<string | null>(null)
const failed = ref(false)
const forbidden = ref(false)
const now = ref(Date.now())

let refreshTimer: ReturnType<typeof setInterval> | null = null
let clockTimer: ReturnType<typeof setInterval> | null = null

async function load(): Promise<void> {
  try {
    presence.value = await api.adminPresence.read()
    receivedAt.value = new Date().toISOString()
    failed.value = false
  } catch (error) {
    failed.value = true
    if (error instanceof ForbiddenError) {
      forbidden.value = true
      stop()
    }
  }
  now.value = Date.now()
}

function stop(): void {
  if (refreshTimer) clearInterval(refreshTimer)
  if (clockTimer) clearInterval(clockTimer)
  refreshTimer = null
  clockTimer = null
}

function start(): void {
  stop()
  if (document.visibilityState !== 'visible' || forbidden.value) return
  void load()
  refreshTimer = setInterval(() => void load(), REFRESH_MS)
  clockTimer = setInterval(() => {
    now.value = Date.now()
  }, 1000)
}

const onVisibility = (): void => (document.visibilityState === 'visible' ? start() : stop())

onMounted(() => {
  document.addEventListener('visibilitychange', onVisibility)
  start()
})

onBeforeUnmount(() => {
  stop()
  document.removeEventListener('visibilitychange', onVisibility)
})

const age = computed(() => (receivedAt.value ? secondsSince(receivedAt.value, now.value) : 0))
const unavailable = computed(() => failed.value && (!presence.value || age.value > STALE_S))
const loading = computed(() => !presence.value && !failed.value)

const maxZone = computed(() => Math.max(0, ...(presence.value?.zones ?? []).map((z) => z.count)))
const chart = computed(() => sparklinePaths(presence.value?.last_hour ?? [], CHART_W, CHART_H, 6))
const chartLabel = computed(() => {
  const totals = (presence.value?.last_hour ?? []).map((s) => s.total)
  if (!totals.length) return ''
  return t('admin.dashboard.presence.hour.label', { min: Math.min(...totals), max: Math.max(...totals) })
})

// Le pic se lit à l'heure de qui regarde, et le dit.
const viewerZone = import.meta.client ? Intl.DateTimeFormat().resolvedOptions().timeZone : 'UTC'
const peakLabel = computed(() => {
  const peak = presence.value?.peak
  if (!peak) return ''
  return t('admin.dashboard.presence.hour.peak', { count: peak.total, time: timeWithZone(peak.at, viewerZone) })
})
</script>

<template>
  <section v-if="!forbidden" aria-labelledby="admin-presence-title">
    <div
      v-if="unavailable"
      class="flex items-start gap-3 rounded-lg border border-border bg-surface-raised px-5 py-4 text-sm text-text-secondary"
      role="status"
    >
      <span class="mt-1.5 size-2.5 shrink-0 rounded-full bg-neutral" aria-hidden="true" />
      <p>
        <strong id="admin-presence-title" class="block text-text">{{ t('admin.dashboard.presence.title') }}</strong>
        {{ t('admin.dashboard.presence.unavailable') }}
      </p>
    </div>

    <UiCard v-else flush>
      <div class="grid md:grid-cols-[minmax(0,300px)_minmax(0,1fr)]">
        <div class="border-b border-border-subtle px-6 py-5 md:border-r md:border-b-0">
          <h2 id="admin-presence-title" class="flex items-center gap-2 text-sm font-semibold text-text-secondary">
            <span class="presence-dot relative size-2.5 rounded-full bg-success" aria-hidden="true" />
            {{ t('admin.dashboard.presence.title') }}
          </h2>

          <UiLoadingState v-if="loading" class="mt-4" :lines="3" :label="t('admin.dashboard.presence.loading')" />

          <template v-else-if="presence">
            <p class="mt-2.5">
              <span
                class="block text-[3.4rem] leading-none font-bold tabular-nums"
                :class="presence.total ? 'text-heading' : 'text-text-muted'"
              >{{ presence.total }}</span>
              <span class="mt-1 block text-text-secondary">
                {{ t('admin.dashboard.presence.onPlatform', presence.total) }}
              </span>
            </p>
            <p class="mt-3.5 flex flex-wrap gap-x-4 gap-y-1.5 text-sm">
              <span>
                <strong class="tabular-nums">{{ presence.signed_in }}</strong>
                {{ t('admin.dashboard.presence.signedIn', presence.signed_in) }}
              </span>
              <span>
                <strong class="tabular-nums">{{ presence.visitors }}</strong>
                {{ t('admin.dashboard.presence.visitors', presence.visitors) }}
              </span>
            </p>
            <p class="mt-4 text-xs text-text-subtle" aria-live="off">
              {{ t('admin.dashboard.presence.updated', { seconds: age }) }}
            </p>
          </template>
        </div>

        <div class="min-w-0 px-6 py-5">
          <h3 class="mb-3 text-sm font-semibold text-text-secondary">{{ t('admin.dashboard.presence.where') }}</h3>

          <UiSkeletonLoader v-if="loading" variant="text" :lines="4" />

          <template v-else-if="presence">
            <p v-if="!presence.zones.length" class="text-sm text-text-muted">
              {{ t('admin.dashboard.presence.nobody') }}
            </p>
            <ul v-else class="grid gap-2.5">
              <li
                v-for="row in presence.zones"
                :key="row.zone"
                class="grid grid-cols-[7.5rem_minmax(0,1fr)_2.25rem] items-center gap-3 text-sm sm:grid-cols-[10.5rem_minmax(0,1fr)_2.75rem]"
              >
                <span class="min-w-0 leading-snug">{{ t(`admin.dashboard.presence.zones.${row.zone}`) }}</span>
                <span class="block h-2.5 overflow-hidden rounded-full bg-surface-sunken" aria-hidden="true">
                  <span class="block h-full rounded-full bg-accent" :style="{ width: `${presenceBarWidth(row.count, maxZone)}%` }" />
                </span>
                <strong class="text-right tabular-nums">{{ row.count }}</strong>
              </li>
            </ul>

            <div v-if="chart" class="mt-4.5">
              <h3 class="mb-2 flex flex-wrap justify-between gap-x-3 text-sm font-semibold text-text-secondary">
                {{ t('admin.dashboard.presence.hour.title') }}
                <span v-if="peakLabel" class="font-normal text-text-subtle">{{ peakLabel }}</span>
              </h3>
              <svg
                class="block h-14 w-full"
                :viewBox="`0 0 ${CHART_W} ${CHART_H}`"
                preserveAspectRatio="none"
                role="img"
                :aria-label="chartLabel"
              >
                <path :d="chart.area" class="presence-area" />
                <path
                  :d="chart.line"
                  fill="none"
                  stroke="var(--color-accent)"
                  stroke-width="2.5"
                  vector-effect="non-scaling-stroke"
                  stroke-linejoin="round"
                />
              </svg>
            </div>
          </template>
        </div>
      </div>
    </UiCard>
  </section>
</template>

<style scoped>
.presence-area {
  fill: color-mix(in srgb, var(--color-accent) 12%, transparent);
}

/* Une onde lente autour du point, jamais pour qui a demandé moins d'animations. */
@media (prefers-reduced-motion: no-preference) {
  .presence-dot::after {
    content: '';
    position: absolute;
    inset: -4px;
    border-radius: 9999px;
    border: 2px solid var(--color-success);
    opacity: 0;
    animation: presence-wave 2.4s ease-out infinite;
  }
}

@keyframes presence-wave {
  0% {
    transform: scale(0.6);
    opacity: 0.7;
  }
  100% {
    transform: scale(1.6);
    opacity: 0;
  }
}
</style>
