<script setup lang="ts">
import type { ProgrammeStripDay } from '~/types/event-programme'
import type { IsoDate, TimeZoneName } from '~/types/shared'

/**
 * La bande des jours : un gros chiffre par journée de programme, et sous lui
 * un carré par activité retenue, de la couleur de sa thématique — on lit d'un
 * coup d'œil quels jours sont chargés, et en quoi.
 */

interface Props {
  days: ProgrammeStripDay[]
  selected: IsoDate
  /** Jours de la semaine affichée, soulignés en vue semaine. */
  week?: IsoDate[]
  timezone: TimeZoneName
}

const props = defineProps<Props>()
const emit = defineEmits<{ select: [date: IsoDate] }>()

const { t, locale } = useI18n()
const { dayLong } = useDateTime()

const MAX_DOTS = 8

const weekSet = computed(() => new Set(props.week ?? []))

const at = (day: IsoDate) => `${day}T12:00:00Z`
const weekday = (day: IsoDate) =>
  new Intl.DateTimeFormat(locale.value, { weekday: 'short', timeZone: 'UTC' }).format(new Date(at(day)))
const month = (day: IsoDate) =>
  new Intl.DateTimeFormat(locale.value, { month: 'short', timeZone: 'UTC' }).format(new Date(at(day)))

const tiles = computed(() =>
  props.days.map((day, index) => ({
    ...day,
    selected: day.date === props.selected,
    inWeek: weekSet.value.has(day.date),
    weekday: weekday(day.date),
    number: day.date.slice(8),
    month: index === 0 || props.days[index - 1]?.date.slice(0, 7) !== day.date.slice(0, 7) ? month(day.date) : '',
    dots: day.colors.slice(0, MAX_DOTS),
    more: Math.max(day.colors.length - MAX_DOTS, 0),
    label: `${dayLong(at(day.date), props.timezone)}, ${t('programme.days.count', day.count)}`,
  })),
)

const scroller = useTemplateRef<HTMLElement>('scroller')

function reveal(): void {
  const box = scroller.value
  const tile = box?.querySelector<HTMLElement>('[aria-pressed="true"]')
  if (!box || !tile) return
  box.scrollLeft = tile.offsetLeft - (box.clientWidth - tile.offsetWidth) / 2
}

onMounted(reveal)
watch(() => props.selected, () => nextTick(reveal))
</script>

<template>
  <div
    ref="scroller"
    class="-mx-4 flex snap-x gap-2 overflow-x-auto px-4 pt-1 pb-3 sm:mx-0 sm:px-1"
    role="group"
    :aria-label="t('programme.days.label')"
  >
    <template v-for="tile in tiles" :key="tile.date">
      <div
        v-if="tile.gapBefore"
        class="flex w-10 shrink-0 items-center justify-center rounded-md border-2 border-dashed border-poster-line"
        aria-hidden="true"
      >
        <span
          class="font-poster-mono text-[0.6875rem] tracking-[0.08em] text-poster-ink-muted uppercase [writing-mode:vertical-rl] rotate-180"
        >
          {{ t('programme.days.gap', tile.gapBefore) }}
        </span>
      </div>

      <button
        type="button"
        class="relative flex h-30 min-w-22 flex-1 shrink-0 cursor-pointer snap-start flex-col justify-between rounded-md border-2 border-poster-ink px-3 py-2.5 text-left transition-transform"
        :class="
          tile.selected
            ? 'translate-x-[3px] translate-y-[3px] bg-poster-ink text-poster-on-ink'
            : [
                'shadow-poster-sm hover:-translate-y-0.5',
                tile.inWeek ? 'bg-poster-paper-sunken text-poster-ink' : 'bg-poster-paper-raised text-poster-ink',
              ]
        "
        :aria-pressed="tile.selected"
        :aria-label="tile.label"
        @click="emit('select', tile.date)"
      >
        <span class="flex w-full items-center justify-between gap-1 font-poster-mono text-xs font-semibold tracking-[0.08em] uppercase">
          <span>{{ tile.weekday }}<template v-if="tile.month"> · {{ tile.month }}</template></span>
          <span
            v-if="tile.isToday"
            class="rounded-sm bg-live px-1.5 py-0.5 text-[0.625rem] text-live-contrast"
          >
            {{ t('programme.days.today') }}
          </span>
        </span>
        <span
          class="font-poster text-6xl leading-[0.85] font-black font-stretch-[62%]"
          :class="tile.selected ? 'text-poster-on-ink-accent' : ''"
        >
          {{ tile.number }}
        </span>
        <span class="flex min-h-2.5 items-center gap-0.5" aria-hidden="true">
          <span
            v-for="(color, index) in tile.dots"
            :key="index"
            class="size-2.5 border border-current"
            :style="{ background: color ?? 'var(--color-poster-paper-sunken)' }"
          />
          <span v-if="tile.more" class="ml-0.5 font-poster-mono text-[0.625rem]">+{{ tile.more }}</span>
          <span v-if="!tile.count" class="font-poster-mono text-[0.6875rem]">—</span>
        </span>
      </button>
    </template>
  </div>
</template>
