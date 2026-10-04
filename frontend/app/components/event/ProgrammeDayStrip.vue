<script setup lang="ts">
import type { ProgrammeStripDay } from '~/types/event-programme'
import type { IsoDate, TimeZoneName } from '~/types/shared'

/**
 * La bande « Aller à la journée », figée au-dessus de la liste : une case par
 * journée, un point par activité retenue. La journée lue dans la liste s'y
 * allume ; un clic y fait défiler la liste.
 */

interface Props {
  days: ProgrammeStripDay[]
  selected: IsoDate
  timezone: TimeZoneName
  pastHidden: boolean
  /** Le bouton des journées passées n'a de sens que s'il en existe. */
  hasPast: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ select: [date: IsoDate]; togglePast: [] }>()

const { t, locale } = useI18n()
const { dayLong } = useDateTime()

const MAX_DOTS = 5

const at = (day: IsoDate) => `${day}T12:00:00Z`
const format = (day: IsoDate, options: Intl.DateTimeFormatOptions) =>
  new Intl.DateTimeFormat(locale.value, { ...options, timeZone: 'UTC' }).format(new Date(at(day)))

const today = computed(
  () => props.days.find((day) => day.isToday)?.date ?? dayKeyInZone(new Date(), props.timezone),
)

const tiles = computed(() =>
  props.days.map((day, index) => {
    const month = day.date.slice(0, 7)
    return {
      date: day.date,
      month: index === 0 || props.days[index - 1]?.date.slice(0, 7) !== month ? format(day.date, { month: 'long' }) : null,
      selected: day.date === props.selected,
      past: day.date < today.value,
      weekday: format(day.date, { weekday: 'short' }),
      number: format(day.date, { day: 'numeric' }),
      dots: Math.min(day.count, MAX_DOTS),
      label: `${dayLong(at(day.date), props.timezone)}, ${t('programme.days.count', { count: day.count }, day.count)}`,
    }
  }),
)

const scroller = useTemplateRef<HTMLElement>('scroller')

function reveal(): void {
  const box = scroller.value
  const tile = box?.querySelector<HTMLElement>('[aria-current="date"]')
  if (!box || !tile || box.scrollWidth <= box.clientWidth) return
  box.scrollLeft = tile.offsetLeft - (box.clientWidth - tile.offsetWidth) / 2
}

onMounted(reveal)
watch(() => props.selected, () => nextTick(reveal))
</script>

<template>
  <nav
    class="flex flex-col gap-1 border-b border-border-subtle bg-surface py-3.5 font-sans lg:flex-row lg:items-center lg:gap-4"
    :aria-label="t('programme.days.jump')"
  >
    <div ref="scroller" class="-mx-4 flex min-w-0 items-center gap-1 overflow-x-auto px-4 py-0.5 sm:mx-0 sm:px-0.5 lg:flex-1">
      <template v-for="(tile, index) in tiles" :key="tile.date">
        <span
          v-if="tile.month"
          class="mr-3 shrink-0 text-xs text-text-muted uppercase"
          :class="{ 'ml-3': index > 0 }"
          :style="{ letterSpacing: 'var(--tracking-caps)' }"
        >
          {{ tile.month }}
        </span>
        <button
          type="button"
          class="flex h-14 w-15 shrink-0 cursor-pointer flex-col items-center justify-center rounded-lg border transition-colors"
          :class="
            tile.selected
              ? 'border-text bg-text font-bold text-surface'
              : ['border-transparent hover:border-border', tile.past ? 'text-text-subtle' : 'text-text']
          "
          :aria-current="tile.selected ? 'date' : undefined"
          :aria-label="tile.label"
          @click="emit('select', tile.date)"
        >
          <span class="text-[11px] font-normal tracking-[0.06em] uppercase opacity-80">{{ tile.weekday }}</span>
          <span class="text-[20px] leading-[1.1] tabular-nums">{{ tile.number }}</span>
          <span class="mt-0.75 flex h-1 gap-0.5" aria-hidden="true">
            <span v-for="dot in tile.dots" :key="dot" class="size-1 rounded-full bg-current opacity-70" />
          </span>
        </button>
      </template>
    </div>

    <button
      v-if="props.hasPast"
      type="button"
      class="inline-flex min-h-11 shrink-0 cursor-pointer items-center self-start text-sm font-bold text-accent hover:underline lg:self-auto"
      @click="emit('togglePast')"
    >
      {{ t(props.pastHidden ? 'programme.days.showPast' : 'programme.days.hidePast') }}
    </button>
  </nav>
</template>
