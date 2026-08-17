<script setup lang="ts">
import type { ProgrammeDay } from '~/types/event-programme'
import type { PublicScheduleRow } from '~/types/views'
import type { TimeZoneName } from '~/types/shared'

/**
 * VUE GRILLE — la liste dense de la programmation, groupée par jour.
 *
 * C'est la vue par défaut, et ce choix se défend : elle est lisible sur un
 * téléphone, elle se cherche au clavier, elle s'imprime, et elle ne demande
 * aucune notion d'échelle de temps. Le calendrier place les créneaux, la grille
 * les LIT — sur une COP, on cherche d'abord « qu'est-ce qui parle d'adaptation »
 * avant « qu'y a-t-il à 14 h ».
 *
 * ELLE NE RECOMPOSE RIEN : chaque carte reçoit une ligne de
 * `programme.v_public_schedule` telle quelle, et `UiSessionCard` en tire l'état,
 * l'organisation, son pays, les thématiques et la couverture. Le regroupement
 * par jour est fait sur la date civile DANS LE FUSEAU DE L'ÉDITION : une séance
 * de 23 h à Belém appartient au 12 novembre, quelle que soit l'heure du visiteur.
 */

interface Props {
  days: ProgrammeDay[]
  timezone: TimeZoneName
  zoneLabel?: string
  /** Titres de journées, par date — `event.event_days.title`. */
  dayTitles?: Record<string, string>
  /** Séance mise en avant (sélection partagée avec la vue calendrier). */
  selectedId?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ select: [session: PublicScheduleRow] }>()

const { t } = useI18n()
const { date } = useDateTime()

/** Libellé de jour : « jeudi 12 novembre 2027 », plus le titre quand il y en a un. */
function dayLabel(day: ProgrammeDay): string {
  return date(`${day.date}T12:00:00Z`, props.timezone)
}
</script>

<template>
  <div class="flex flex-col gap-8">
    <section v-for="day in props.days" :key="day.date" :aria-labelledby="`jour-${day.date}`">
      <!-- L'en-tête de jour reste visible au défilement : dans une liste de
           quarante activités, on perd vite le fil de la journée qu'on lit. -->
      <div class="sticky top-0 z-10 -mx-1 bg-surface/95 px-1 py-2 backdrop-blur-sm">
        <h3 :id="`jour-${day.date}`" class="font-display text-lg">
          {{ dayLabel(day) }}
          <span v-if="props.dayTitles?.[day.date]" class="text-text-muted">
            · {{ props.dayTitles[day.date] }}
          </span>
        </h3>
        <p class="text-xs text-text-subtle">
          {{ t('programme.grid.dayCount', { count: day.sessions.length }, day.sessions.length) }}
        </p>
      </div>

      <ul class="mt-3 grid list-none gap-4 p-0">
        <li v-for="session in day.sessions" :key="session.id">
          <UiSessionCard
            :session="session"
            :zone-label="props.zoneLabel"
            :cancelled-reason="null"
            :class="props.selectedId === session.id ? 'ring-2 ring-accent' : ''"
          >
            <template #actions>
              <UiButton
                variant="secondary"
                size="sm"
                :label="t('programme.grid.details')"
                @click="emit('select', session)"
              />
            </template>
          </UiSessionCard>
        </li>
      </ul>
    </section>
  </div>
</template>
