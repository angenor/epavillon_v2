<script setup lang="ts">
import type { ScheduleTrackBadge } from '~/types/views'
import type { SessionDisplayState } from '~/components/ui/StatusBadge.vue'

/**
 * LÉGENDE DES ÉTATS — visible sans survol, et jamais réduite à des couleurs.
 *
 * Le prompt l'exige, et la règle d'accessibilité l'imposerait de toute façon :
 * « un repère qui ne repose pas uniquement sur la couleur ». Chaque état porte
 * donc TROIS marques simultanées — une couleur, une pastille, et son nom écrit.
 * Une légende au survol n'existe ni au clavier, ni au doigt, ni à l'impression.
 *
 * LES SIX ÉTATS. Cinq viennent de `v_public_schedule.temporal_state`, calculés
 * en base. Le sixième — EN DIRECT — n'en fait pas partie et ne peut pas en
 * faire partie : il ne dépend pas de l'heure mais de la diffusion. Il est
 * conservé dans la légende parce qu'il se voit dans la programmation, mais il
 * est marqué à part.
 *
 * LES JOURNÉES SPÉCIALES sont une SECONDE dimension, pas un état de plus : une
 * activité peut être à venir ET appartenir à la journée finance. D'où le liseré
 * de couleur, distinct de la pastille d'état — et d'où le fait que leur couleur
 * vienne de la base (`programme_tracks.color_hex`), modifiable au back-office.
 */

interface Props {
  /** Journées spéciales présentes dans le programme affiché. */
  tracks: ScheduleTrackBadge[]
}

defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()

const STATES: SessionDisplayState[] = ['upcoming', 'ongoing', 'live', 'past', 'postponed', 'cancelled']
</script>

<template>
  <section
    class="rounded-md border border-border bg-surface-sunken px-4 py-3"
    :aria-label="t('event.public.programme.legend.title')"
  >
    <div class="flex flex-wrap items-center gap-x-5 gap-y-2">
      <p class="text-xs uppercase text-text-subtle" :style="{ letterSpacing: 'var(--tracking-caps)' }">
        {{ t('event.public.programme.legend.title') }}
      </p>
      <UiStatusBadge
        v-for="state in STATES"
        :key="state"
        :state="state"
        size="sm"
        :label="t(state === 'live' ? 'live-badge.label' : `session-card.state.${state}`)"
      />
    </div>

    <div v-if="tracks.length" class="mt-3 flex flex-wrap items-center gap-x-5 gap-y-2 border-t border-separator pt-3">
      <p class="text-xs uppercase text-text-subtle" :style="{ letterSpacing: 'var(--tracking-caps)' }">
        {{ t('event.public.programme.legend.tracks') }}
      </p>
      <span
        v-for="track in tracks"
        :key="track.slug"
        class="inline-flex items-center gap-2 border-l-4 pl-2 text-sm text-text-secondary"
        :style="track.color ? { borderLeftColor: track.color } : undefined"
      >
        {{ tr(track.title) }}
      </span>
    </div>
  </section>
</template>
