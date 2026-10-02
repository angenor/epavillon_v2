<script setup lang="ts">
/** Le bouton qui compte, sur fond d'encre : dans le billet et dans la barre qui le suit. */

export type TicketCallToAction = 'register' | 'waitlist' | 'watch' | 'replay'

interface Props {
  action: TicketCallToAction
  replayUrl?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ register: [] }>()

const { t } = useI18n()
</script>

<template>
  <button
    v-if="props.action === 'register' || props.action === 'waitlist'"
    type="button"
    class="inline-flex h-13.5 cursor-pointer items-center justify-center rounded-md border-2 border-poster-today-strong px-6 text-[1.0625rem] font-bold whitespace-nowrap"
    :class="props.action === 'register' ? 'bg-poster-today-strong text-poster-on-today' : 'bg-transparent text-poster-today-strong'"
    @click="emit('register')"
  >
    {{ t(props.action === 'register' ? 'activity.ticket.register' : 'activity.ticket.waitlist') }}
  </button>
  <a
    v-else-if="props.action === 'watch'"
    href="#direct"
    class="inline-flex h-13.5 items-center justify-center gap-2.5 rounded-md bg-live px-6 text-[1.0625rem] font-bold whitespace-nowrap text-live-contrast"
  >
    <UiIcon name="broadcast" size="1.125rem" />
    {{ t('activity.ticket.watch') }}
  </a>
  <a
    v-else-if="props.replayUrl"
    :href="props.replayUrl"
    target="_blank"
    rel="noopener"
    class="inline-flex h-13.5 items-center justify-center gap-2.5 rounded-md border-2 border-poster-on-ink px-6 text-[1.0625rem] font-bold whitespace-nowrap"
  >
    <UiIcon name="video" size="1.125rem" />
    {{ t('activity.ticket.replay') }}
  </a>
</template>
