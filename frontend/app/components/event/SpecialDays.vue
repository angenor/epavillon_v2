<script setup lang="ts">
import type { ProgrammeTrack } from '~/types/event/edition'
import type { TimeZoneName } from '~/types/shared'

/**
 * LES JOURNÉES SPÉCIALES — `event.programme_tracks`, celles qui sont publiées.
 *
 * UNE JOURNÉE SPÉCIALE N'EST PAS UN JOUR DU CALENDRIER. C'est un fil composé à
 * la main par l'IFDD parmi les activités retenues (règle métier n° 7) : elle
 * peut n'occuper qu'une demi-journée, deux fils peuvent partager un même jour,
 * et un fil peut déborder sur le lendemain. Les dates affichées ici sont donc
 * annoncées comme INDICATIVES — c'est le mot du modèle, pas une précaution de
 * rédaction : `starts_on` « ne contraint pas le rattachement des sessions ».
 *
 * LA COULEUR VIENT DE LA DONNÉE (`programme_tracks.color_hex`), modifiable au
 * back-office, et elle est rendue en LISERÉ et en POINT, jamais en fond : une
 * teinte saisie par un administrateur n'a aucune garantie de contraste avec le
 * texte. Le fil qui n'en porte pas s'affiche sans, sans couleur de remplacement.
 */

interface Props {
  /** Fils publiés de l'édition, journées spéciales d'abord. */
  tracks: ProgrammeTrack[]
  timezone: TimeZoneName
  zoneLabel?: string
  /** Nombre d'activités rattachées, par identifiant de fil. */
  sessionCounts?: Record<string, number>
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dateRange, date } = useDateTime()

function period(track: ProgrammeTrack): string {
  if (!track.starts_on) return ''
  if (!track.ends_on || track.ends_on === track.starts_on) {
    return date(`${track.starts_on}T12:00:00Z`, props.timezone)
  }
  return dateRange(`${track.starts_on}T12:00:00Z`, `${track.ends_on}T12:00:00Z`, props.timezone)
}
</script>

<template>
  <section v-if="props.tracks.length" aria-labelledby="journees-titre">
    <h2 id="journees-titre" class="font-display text-2xl sm:text-3xl">
      {{ t('event.public.specialDays.title') }}
    </h2>
    <p class="mt-3 text-sm text-text-muted" :style="{ maxWidth: 'var(--measure)' }">
      {{ t('event.public.specialDays.description') }}
    </p>

    <ul class="mt-6 grid list-none gap-4 p-0 md:grid-cols-2">
      <li
        v-for="track in props.tracks"
        :key="track.id"
        class="relative overflow-hidden rounded-xl border border-border bg-surface-raised p-5 pl-6"
      >
        <!-- LE LISERÉ PORTE LA COULEUR DE LA DONNÉE, jamais le fond : une teinte
             saisie au back-office n'a aucune garantie de contraste avec le
             texte. Le fil qui n'en porte pas s'affiche sans, sans remplacement. -->
        <span
          v-if="track.color_hex"
          class="absolute inset-y-0 left-0 w-1.5"
          :style="{ backgroundColor: track.color_hex }"
          aria-hidden="true"
        />

        <h3 class="font-display text-lg">{{ tr(track.title) }}</h3>
        <p v-if="track.subtitle" class="mt-1 text-sm font-semibold text-text-secondary">
          {{ tr(track.subtitle) }}
        </p>
        <p v-if="track.description" class="mt-3 text-sm text-text-secondary">
          {{ tr(track.description) }}
        </p>

        <div class="mt-4 flex flex-wrap items-center gap-x-3 gap-y-2">
          <p v-if="period(track)" class="flex items-center gap-1.5 text-sm text-text-muted">
            <UiIcon name="calendar" size="0.95rem" class="shrink-0" />
            <span>
              {{ period(track) }}
              <span class="text-text-subtle">· {{ t('event.public.specialDays.indicative') }}</span>
            </span>
          </p>
          <UiBadge v-if="props.sessionCounts?.[track.id]" size="sm" :dot-color="track.color_hex">
            {{
              t(
                'event.public.specialDays.activityCount',
                { count: props.sessionCounts[track.id] },
                props.sessionCounts[track.id] ?? 0,
              )
            }}
          </UiBadge>
        </div>
      </li>
    </ul>
  </section>
</template>
