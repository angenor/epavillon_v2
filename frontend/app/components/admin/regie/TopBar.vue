<script setup lang="ts">
import type { IsoDate, TimeZoneName } from '~/types/shared'

interface Props {
  title: string
  days: IsoDate[]
  day: IsoDate
  today: IsoDate
  timezone: TimeZoneName
  zoneLabel: string | null
  nowMs: number
  /** Ce qui passe à l'antenne, nul sans direct. */
  onAir: string | null
  loading?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'select-day': [day: IsoDate] }>()

const { t } = useI18n()
const dt = useDateTime()

// Un jour civil, sans fuseau : midi UTC le garde le même partout.
const dayFormat = computed(() => new Intl.DateTimeFormat(dt.intlLocale.value, { weekday: 'short', day: 'numeric', timeZone: 'UTC' }))
const dayLabel = (day: IsoDate): string => dayFormat.value.format(new Date(`${day}T12:00:00Z`))

const clock = computed(() => dt.time(props.nowMs, props.timezone))
const zone = computed(() =>
  t('admin.regie.top.zone', {
    zone: dt.zoneLabel(props.timezone, props.zoneLabel ?? undefined),
    offset: dt.zoneOffsetShort(props.timezone, props.nowMs),
  }),
)
</script>

<template>
  <header class="bg-surface-inverse text-text-on-inverse">
    <div class="mx-auto flex max-w-[90rem] flex-wrap items-center gap-x-6 gap-y-3 px-4 py-3 sm:px-6">
      <h1 class="heading-condensed text-[1.7rem]">
        {{ t('admin.regie.top.title') }}
        <span class="text-accent-on-inverse">{{ props.title }}</span>
      </h1>

      <div class="flex flex-wrap items-center gap-1" role="group" :aria-label="t('admin.regie.top.days')">
        <button
          v-for="day in props.days"
          :key="day"
          type="button"
          class="min-h-(--target-min) cursor-pointer rounded-md border border-border-on-inverse px-3 font-condensed text-base font-semibold tracking-[0.04em] uppercase transition-colors duration-(--duration-fast) disabled:cursor-wait"
          :class="day === props.day ? 'bg-surface-inverse-selected text-text-on-inverse-selected' : 'hover:bg-surface-inverse-raised'"
          :aria-current="day === props.day ? 'date' : undefined"
          :disabled="props.loading"
          @click="emit('select-day', day)"
        >
          {{ dayLabel(day) }}
          <span v-if="day === props.today" class="sr-only">{{ t('admin.regie.top.today') }}</span>
        </button>
      </div>

      <span
        v-if="props.onAir"
        class="inline-flex items-center gap-2 rounded-full bg-live px-3.5 py-2 font-condensed text-base font-bold tracking-[0.05em] text-live-contrast uppercase"
      >
        <span class="regie-pulse size-2.5 shrink-0 rounded-full bg-current" aria-hidden="true" />
        {{ t('admin.regie.top.onAir', { name: props.onAir }) }}
      </span>

      <div class="ml-auto text-right font-condensed leading-none">
        <b class="block text-[2.4rem] font-bold tabular-nums" aria-live="off">{{ clock }}</b>
        <small class="mt-1 block text-[0.8rem] font-semibold tracking-[0.05em] text-text-on-inverse-muted uppercase">{{ zone }}</small>
      </div>
    </div>
  </header>
</template>

<style scoped>
/* Pulsation lente (WCAG 2.3.1), neutralisée avec « moins d'animations ». */
.regie-pulse {
  animation: regie-pulse 1.6s ease-in-out infinite;
}
@keyframes regie-pulse {
  50% { opacity: 0.35; }
}
@media (prefers-reduced-motion: reduce) {
  .regie-pulse { animation: none; }
}
</style>
