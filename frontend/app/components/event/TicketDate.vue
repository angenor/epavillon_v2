<script setup lang="ts">
/**
 * La bande de date en tête du billet (journée spéciale, activité) : le numéro
 * du jour en grand, le jour et le mois à côté, à la couleur de la journée
 * (`--color-day-solid`, posée par `trackDayStyle`). Le créneau `icon` reçoit le
 * pictogramme de la journée.
 */

interface Props {
  /** Instant ou date ISO du jour montré. */
  date: string
  timezone: string
  /** Sous le mois : le nom de la journée spéciale, par exemple. */
  caption?: string | null
}

const props = withDefaults(defineProps<Props>(), { caption: null })

const { intlLocale } = useDateTime()

const parts = computed(() => {
  const date = new Date(props.date)
  const format = (options: Intl.DateTimeFormatOptions) =>
    new Intl.DateTimeFormat(intlLocale.value, { ...options, timeZone: props.timezone }).format(date)
  return {
    day: format({ day: 'numeric' }),
    weekday: format({ weekday: 'long' }),
    month: format({ month: 'long', year: 'numeric' }),
  }
})
</script>

<template>
  <div class="flex shrink-0 items-center gap-4 bg-(--color-day-solid) px-6 py-4 text-(--color-day-on-solid)">
    <b class="font-condensed text-[4rem] leading-[0.8] font-bold tabular-nums" aria-hidden="true">{{ parts.day }}</b>
    <p class="min-w-0 font-condensed text-lg leading-tight font-bold tracking-[0.04em] uppercase">
      <span class="block">{{ parts.weekday }}</span>
      <span class="block">{{ parts.month }}</span>
      <small v-if="props.caption" class="mt-0.5 block text-xs tracking-[0.06em] opacity-85">{{ props.caption }}</small>
    </p>
    <span class="ml-auto shrink-0"><slot name="icon" /></span>
  </div>
</template>
