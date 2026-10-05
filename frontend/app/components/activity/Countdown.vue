<script setup lang="ts">
/**
 * Le compte à rebours de la carte, à la seconde. Il ne tourne qu'une fois monté :
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

const cells = computed(() => {
  const seconds = left.value
  const pad = (value: number) => String(value).padStart(2, '0')
  if (seconds === null) {
    return (['h', 'm', 's'] as const).map((unit) => ({ unit, value: '--', label: t(`activity.countdown.${unit}`, 2) }))
  }
  const days = Math.floor(seconds / 86_400)
  const hours = Math.floor((seconds % 86_400) / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const list = [
    { unit: 'h', value: pad(hours), label: t('activity.countdown.h', hours) },
    { unit: 'm', value: pad(minutes), label: t('activity.countdown.m') },
    { unit: 's', value: pad(seconds % 60), label: t('activity.countdown.s') },
  ]
  return days ? [{ unit: 'd', value: String(days), label: t('activity.countdown.d', days) }, ...list] : list
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
  <div role="timer" :aria-label="spoken">
    <p class="text-xs text-text-muted uppercase" :style="{ letterSpacing: 'var(--tracking-caps)' }" aria-hidden="true">
      {{ t('activity.ticket.startsIn') }}
    </p>
    <div
      class="mt-3 grid grid-cols-[repeat(var(--cells),minmax(0,1fr))]"
      :style="{ '--cells': String(cells.length) }"
      aria-hidden="true"
    >
      <span
        v-for="cell in cells"
        :key="cell.unit"
        class="flex flex-col items-center border-l border-border-subtle first:border-l-0"
      >
        <span class="relative block h-11 w-full overflow-hidden text-center text-[40px] leading-[44px] font-light tabular-nums" :class="cell.unit === 's' ? 'text-accent' : 'text-text'">
          <Transition name="countdown-tick">
            <span :key="cell.value" class="block">{{ cell.value }}</span>
          </Transition>
        </span>
        <span class="mt-1.5 text-[11px] text-text-muted uppercase" :style="{ letterSpacing: 'var(--tracking-caps)' }">{{ cell.label }}</span>
      </span>
    </div>
  </div>
</template>

<style scoped>
/* Le chiffre qui change monte et cède la place au suivant, comme un compteur mécanique. */
.countdown-tick-enter-active,
.countdown-tick-leave-active {
  transition:
    transform 320ms ease,
    opacity 320ms ease;
}
.countdown-tick-leave-active {
  position: absolute;
  inset-inline: 0;
  top: 0;
}
.countdown-tick-enter-from {
  transform: translateY(60%);
  opacity: 0;
}
.countdown-tick-leave-to {
  transform: translateY(-60%);
  opacity: 0;
}

@media (prefers-reduced-motion: reduce) {
  .countdown-tick-enter-active,
  .countdown-tick-leave-active {
    transition: none;
  }
}
</style>
