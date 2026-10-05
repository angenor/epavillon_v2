<script setup lang="ts">
/**
 * Le compte à rebours de la carte, sur une ligne. Il ne tourne qu'une fois monté :
 * le rendu serveur pose un tiret, sans heure qui serait fausse à l'arrivée.
 */

interface Props {
  startsAt: string
}

const props = defineProps<Props>()
const emit = defineEmits<{ elapsed: [] }>()

const { t } = useI18n()
const { intlLocale } = useDateTime()

const now = ref<number | null>(null)
let clock: ReturnType<typeof setInterval> | undefined

onMounted(() => {
  now.value = Date.now()
  clock = setInterval(() => (now.value = Date.now()), 1000)
})
onBeforeUnmount(() => clearInterval(clock))

const left = computed(() => (now.value === null ? null : Math.max(0, Math.floor((Date.parse(props.startsAt) - now.value) / 1000))))

watch(left, (seconds) => {
  if (seconds !== 0) return
  clearInterval(clock)
  emit('elapsed')
})

// Les unités abrégées viennent d'Intl : « 403 j » en français, « 403 days » en anglais.
const unit = (value: number, name: 'day' | 'hour' | 'minute' | 'second') =>
  new Intl.NumberFormat(intlLocale.value, { style: 'unit', unit: name, unitDisplay: 'short' }).format(value)

const compact = computed(() => {
  const seconds = left.value
  if (seconds === null) return '–'
  const days = Math.floor(seconds / 86_400)
  const hours = Math.floor((seconds % 86_400) / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const parts = [unit(hours, 'hour'), unit(minutes, 'minute')]
  if (days) return [unit(days, 'day'), ...parts].join(' ')
  // Dans la dernière heure, la seconde rend le départ sensible.
  return hours ? parts.join(' ') : [unit(minutes, 'minute'), unit(seconds % 60, 'second')].join(' ')
})

const spoken = computed(() => {
  const seconds = left.value
  if (seconds === null) return t('session-card.state.upcoming')
  return t('activity.countdown.spoken', {
    days: Math.floor(seconds / 86_400),
    hours: Math.floor((seconds % 86_400) / 3600),
    minutes: Math.floor((seconds % 3600) / 60),
  })
})
</script>

<template>
  <p class="flex flex-wrap items-baseline justify-between gap-x-3 text-[0.8125rem] text-text-muted" role="timer" :aria-label="spoken">
    <span aria-hidden="true">{{ t('activity.ticket.startsIn') }}</span>
    <span class="text-xl font-light text-text tabular-nums" aria-hidden="true">{{ compact }}</span>
  </p>
</template>
