<script setup lang="ts">
/**
 * Le compte à rebours du billet, à la seconde. Il ne tourne qu'une fois monté :
 * le rendu serveur pose des tirets, sans heure qui serait fausse à l'arrivée.
 */

interface Props {
  startsAt: string
}

const props = defineProps<Props>()
const emit = defineEmits<{ elapsed: [] }>()

const { t } = useI18n()

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

const units = computed(() => {
  const seconds = left.value
  const unit = (key: string, count: number, digits = 2) => ({
    key,
    count,
    value: seconds === null ? '––' : String(count).padStart(digits, '0'),
  })
  const days = seconds === null ? 2 : Math.floor(seconds / 86_400)
  const parts = [
    unit('h', Math.floor(((seconds ?? 0) % 86_400) / 3600)),
    unit('m', Math.floor(((seconds ?? 0) % 3600) / 60)),
    unit('s', (seconds ?? 0) % 60),
  ]
  return days ? [unit('d', days, 1), ...parts] : parts
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
  <div class="flex" role="timer" :aria-label="spoken">
    <div
      v-for="(unit, index) in units"
      :key="unit.key"
      class="flex flex-1 flex-col items-center gap-1.5"
      :class="index ? 'border-l border-poster-on-ink-muted/30' : ''"
      aria-hidden="true"
    >
      <span class="font-poster text-[3.25rem] leading-[0.85] font-black text-poster-on-ink-accent tabular-nums font-stretch-[62%]">
        {{ unit.value }}
      </span>
      <span class="font-poster-mono text-[0.6875rem] font-semibold tracking-[0.08em] text-poster-on-ink-muted uppercase">
        {{ t(`activity.countdown.${unit.key}`, unit.count) }}
      </span>
    </div>
  </div>
</template>
