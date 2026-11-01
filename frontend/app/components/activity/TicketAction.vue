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
    :variant="props.action === 'register' ? 'primary' : 'secondary'"
    size="lg"
    class="min-h-12!"
    @click="emit('register')"
  >
    {{ label(props.action) }}
  </UiButton>
  <!-- Ni `UiButton` ni sa variante `danger` : l'ancre reste sur la page, et le rouge du direct n'est pas celui d'une suppression. -->
  <a
    v-else-if="props.action === 'watch'"
    href="#direct"
    class="inline-flex min-h-12 items-center justify-center gap-2.5 rounded-md bg-live px-6 font-condensed text-[1.05rem] font-semibold tracking-[0.05em] whitespace-nowrap text-live-contrast uppercase transition-[background-color,transform] hover:-translate-y-px hover:bg-live/90 motion-reduce:hover:translate-y-0"
  >
    <UiIcon name="broadcast" size="1.125rem" />
    {{ label('watch') }}
  </a>
  <!-- Lien simple vers la salle, pas le routeur : la salle se charge d'un coup, et Zoom ne démarre qu'une fois par document. -->
  <a
    v-else-if="props.action === 'join'"
    :href="props.joinTo ?? '#direct'"
    class="inline-flex min-h-12 items-center justify-center gap-2.5 rounded-md bg-live px-6 font-condensed text-[1.05rem] font-semibold tracking-[0.05em] whitespace-nowrap text-live-contrast uppercase transition-[background-color,transform] hover:-translate-y-px hover:bg-live/90 motion-reduce:hover:translate-y-0"
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
