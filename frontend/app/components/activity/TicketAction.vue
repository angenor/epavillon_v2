<script setup lang="ts">
/** Le bouton qui compte : dans la carte de l'activité et dans la barre qui la suit. */

export type TicketCallToAction = 'register' | 'waitlist' | 'watch' | 'join' | 'replay'

interface Props {
  action: TicketCallToAction
  replayUrl?: string | null
  /** Salle Zoom pleine page, entrée directe ; à défaut, l'ancre `#direct`. */
  joinTo?: string
  /** Libellé court : la barre collée en bas n'a pas la largeur du billet. */
  compact?: boolean
}

const props = withDefaults(defineProps<Props>(), { replayUrl: null, joinTo: undefined, compact: false })
const emit = defineEmits<{ register: [] }>()

const { t } = useI18n()

const label = (action: TicketCallToAction) => t(props.compact ? `activity.ticket.short.${action}` : `activity.ticket.${action}`)
</script>

<template>
  <UiButton
    v-if="props.action === 'register' || props.action === 'waitlist'"
    :variant="props.action === 'register' ? 'signup' : 'secondary'"
    size="lg"
    class="min-h-12!"
    @click="emit('register')"
  >
    {{ label(props.action) }}
  </UiButton>
  <UiButton v-else-if="props.action === 'watch'" variant="live" size="lg" icon="broadcast" class="min-h-12!" href="#direct">
    {{ label('watch') }}
  </UiButton>
  <!-- Lien simple vers la salle, pas le routeur : la salle se charge d'un coup, et Zoom ne démarre qu'une fois par document. -->
  <!-- Pas `UiButton` : il ouvrirait la salle dans un nouvel onglet ; même habit que sa variante « live ». -->
  <a
    v-else-if="props.action === 'join'"
    :href="props.joinTo ?? '#direct'"
    data-ui-button
    class="inline-flex min-h-12 items-center justify-center gap-2 rounded-md bg-live px-6 font-bold whitespace-nowrap text-live-contrast no-underline hover:bg-[color-mix(in_srgb,var(--color-live)_85%,var(--color-text))]"
  >
    <UiIcon name="video" size="1.125rem" />
    {{ label('join') }}
  </a>
  <UiButton
    v-else-if="props.replayUrl"
    variant="secondary"
    size="lg"
    icon="video"
    class="min-h-12!"
    :href="props.replayUrl"
  >
    {{ label('replay') }}
  </UiButton>
</template>
