<script setup lang="ts">
/** Le bouton qui compte : dans la carte de l'activité et dans la barre qui la suit. */

export type TicketCallToAction = 'register' | 'waitlist' | 'watch' | 'join' | 'replay'

interface Props {
  action: TicketCallToAction
  replayUrl?: string | null
  /** Salle Zoom pleine page, entrée directe ; à défaut, l'ancre `#direct`. */
  joinTo?: string
}

const props = defineProps<Props>()
const emit = defineEmits<{ register: [] }>()

const { t } = useI18n()
</script>

<template>
  <UiButton
    v-if="props.action === 'register' || props.action === 'waitlist'"
    :variant="props.action === 'register' ? 'primary' : 'secondary'"
    size="lg"
    class="min-h-12!"
    @click="emit('register')"
  >
    {{ t(props.action === 'register' ? 'activity.ticket.register' : 'activity.ticket.waitlist') }}
  </UiButton>
  <!-- Ni `UiButton` ni sa variante `danger` : l'ancre reste sur la page, et le rouge du direct n'est pas celui d'une suppression. -->
  <a
    v-else-if="props.action === 'watch'"
    href="#direct"
    class="inline-flex min-h-12 items-center justify-center gap-2.5 rounded-md bg-live px-6 text-base font-bold whitespace-nowrap text-live-contrast"
  >
    <UiIcon name="broadcast" size="1.125rem" />
    {{ t('activity.ticket.watch') }}
  </a>
  <!-- Lien simple vers la salle, pas le routeur : la salle se charge d'un coup, et Zoom ne démarre qu'une fois par document. -->
  <a
    v-else-if="props.action === 'join'"
    :href="props.joinTo ?? '#direct'"
    class="inline-flex min-h-12 items-center justify-center gap-2.5 rounded-md bg-live px-6 text-base font-bold whitespace-nowrap text-live-contrast"
  >
    <UiIcon name="video" size="1.125rem" />
    {{ t('activity.ticket.join') }}
  </a>
  <UiButton
    v-else-if="props.replayUrl"
    variant="secondary"
    size="lg"
    icon="video"
    class="min-h-12!"
    :href="props.replayUrl"
  >
    {{ t('activity.ticket.replay') }}
  </UiButton>
</template>
