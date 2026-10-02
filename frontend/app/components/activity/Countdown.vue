<script setup lang="ts">
/**
 * Le compte à rebours, à la seconde. Il ne tourne qu'une fois monté : le rendu
 * serveur affiche « À venir », sans heure qui serait fausse à l'arrivée.
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
  if (seconds === null) return []
  const days = Math.floor(seconds / 86_400)
  const parts = [
    { key: 'h', value: Math.floor((seconds % 86_400) / 3600) },
    { key: 'm', value: Math.floor((seconds % 3600) / 60) },
    { key: 's', value: seconds % 60 },
  ].map((part) => ({ ...part, value: String(part.value).padStart(2, '0') }))
  return days ? [{ key: 'd', value: String(days) }, ...parts] : parts
})

const spoken = computed(() => {
  const seconds = left.value
  if (seconds === null) return ''
  return t('activity.countdown.spoken', {
    days: Math.floor(seconds / 86_400),
    hours: Math.floor((seconds % 86_400) / 3600),
    minutes: Math.floor((seconds % 3600) / 60),
  })
})
</script>

<template>
  <span
    v-if="!units.length"
    class="inline-flex h-7.5 items-center rounded border-2 border-poster-ink px-3 text-[0.8125rem] font-bold"
  >
    {{ t('session-card.state.upcoming') }}
  </span>
  <span v-else class="inline-flex items-center gap-2" role="timer" :aria-label="spoken">
    <span class="font-poster-mono text-[0.6875rem] font-semibold tracking-[0.08em] text-poster-ink-muted uppercase" aria-hidden="true">
      {{ t('activity.countdown.label') }}
    </span>
    <span class="inline-flex items-stretch overflow-hidden rounded-md border-2 border-poster-ink bg-poster-ink shadow-poster-sm" aria-hidden="true">
      <span
        v-for="(unit, index) in units"
        :key="unit.key"
        class="inline-flex items-baseline gap-0.5 px-2 py-1"
        :class="index ? 'border-l border-poster-on-ink-muted/40' : ''"
      >
        <span class="font-poster-mono text-lg leading-none font-semibold text-poster-on-ink-accent tabular-nums">{{ unit.value }}</span>
        <span class="font-poster-mono text-[0.625rem] text-poster-on-ink-muted">{{ t(`activity.countdown.${unit.key}`) }}</span>
      </span>
    </span>
  </span>
</template>
