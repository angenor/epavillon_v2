<script setup lang="ts">
import type { ProgrammeStripDay } from '~/types/event-programme'
import type { IsoDate, TimeZoneName } from '~/types/shared'

/**
 * La bande des jours de la liste, figée en haut : un chiffre par journée de
 * programme et un carré par activité retenue, de la couleur de sa thématique.
 * Le jour lu dans la liste s'y allume ; un clic y fait défiler la liste.
 */

interface Props {
  days: ProgrammeStripDay[]
  selected: IsoDate
  timezone: TimeZoneName
}

const props = defineProps<Props>()
const emit = defineEmits<{ select: [date: IsoDate] }>()

const { t, locale } = useI18n()
const { dayLong } = useDateTime()

const MAX_DOTS = 8

const at = (day: IsoDate) => `${day}T12:00:00Z`
const weekday = (day: IsoDate) =>
  new Intl.DateTimeFormat(locale.value, { weekday: 'short', timeZone: 'UTC' }).format(new Date(at(day)))

const tiles = computed(() =>
  props.days.map((day) => ({
    ...day,
    selected: day.date === props.selected,
    weekday: weekday(day.date),
    number: day.date.slice(8),
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
    class="-mx-4 flex snap-x gap-1.5 overflow-x-auto px-4 pt-1 pb-2 sm:mx-0 sm:px-0.5"
    role="group"
    :aria-label="t('programme.days.label')"
  >
    <template v-for="tile in tiles" :key="tile.date">
      <span
        v-if="tile.gapBefore"
        class="mx-1 w-0 shrink-0 self-stretch border-l-2 border-dashed border-poster-line"
        :title="t('programme.days.gap', tile.gapBefore)"
        aria-hidden="true"
      />
      <button
        type="button"
        class="flex h-17 min-w-19 flex-1 shrink-0 cursor-pointer snap-start flex-col justify-between rounded-md border-2 border-poster-ink px-2.5 py-1.5 text-left transition-transform"
        :class="
          tile.selected
            ? 'translate-x-[2px] translate-y-[2px] bg-poster-ink text-poster-on-ink'
            : 'bg-poster-paper-raised text-poster-ink shadow-[2px_2px_0_var(--color-poster-ink)] hover:-translate-y-0.5'
        "
        :aria-pressed="tile.selected"
        :aria-label="tile.label"
        @click="emit('select', tile.date)"
      >
        <span class="flex w-full items-baseline justify-between gap-1">
          <span
            class="font-poster text-[2rem] leading-[0.9] font-black font-stretch-[62%]"
            :class="tile.selected ? 'text-poster-on-ink-accent' : ''"
          >
            {{ tile.number }}
          </span>
          <span
            class="font-poster-mono text-[0.625rem] font-semibold tracking-[0.08em] uppercase"
            :class="tile.isToday && !tile.selected ? 'text-live' : ''"
          >
            {{ tile.weekday }}
          </span>
        </span>
        <span class="flex min-h-2 items-center gap-0.5" aria-hidden="true">
          <span
            v-for="(color, index) in tile.dots"
            :key="index"
            class="size-[7px] border border-current"
            :style="{ background: color ?? 'var(--color-poster-paper-sunken)' }"
          />
          <span v-if="tile.more" class="ml-0.5 font-poster-mono text-[0.625rem]">+{{ tile.more }}</span>
        </span>
      </button>
    </template>
  </div>
</template>
