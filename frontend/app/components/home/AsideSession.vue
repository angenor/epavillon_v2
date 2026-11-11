<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { IsoDateTime } from '~/types/shared'

/**
 * UNE ACTIVITÉ DU PANNEAU « À VENIR » : l'heure en grand, le jour abrégé, le
 * titre et qui l'organise. Ni salle ni thématiques (arbitrage du 03/10).
 *
 * L'HEURE EST CELLE DE LA SÉANCE, dans son fuseau. Quand le panneau ne nomme
 * pas une édition commune (et donc son fuseau), la ligne porte le sien.
 *
 * UN SEUL DIRECT À LA FOIS (règle n° 4) : `UiLiveBadge` ne s'affiche que pour la
 * séance déclarée par `useLiveSession()` ; une autre séance en cours porte
 * « En cours », en jaune.
 *
 * L'HEURE SUR UN MARQUE-PAGE (maquette `docs/maquettes/a-venir-post-it/`, 02/11)
 * qui dépasse du bord gauche du panneau : papier à la couleur de la journée
 * spéciale, rouge pour le direct, tenu par un ruban ou une épingle selon le jour.
 *
 * EN DIRECT (maquette `docs/maquettes/direct-post-it/`, 11/11) : « Direct » au
 * lieu de l'heure prévue, qui se lirait comme une annonce, la durée écoulée et
 * une jauge jusqu'à la fin prévue ; dessous, « Rejoindre le direct » et la fin
 * en clair. Comme le bandeau, la durée part de l'heure prévue.
 */

export type AsideFastening = 'tape' | 'pin'

/** Un peu de travers, comme posé à la main ; jamais deux voisins du même côté. */
const TILTS = ['-2.5deg', '1.8deg', '-1.2deg']

interface Props {
  session: PublicScheduleRow
  /** Destination : la page de l'activité, ou sa journée spéciale. */
  to?: string
  /** Ville de l'édition (« Belém ») ; la ville tirée de l'identifiant IANA n'est qu'un repli, sans accent. */
  zoneLabel?: string
  /** Sigle de l'édition, seulement quand le panneau en mêle plusieurs. */
  editionLabel?: string | null
  /** Écrire le fuseau sur la ligne : le panneau ne le nomme pas. */
  showZone?: boolean
  fastening?: AsideFastening
  /** Rang dans le panneau : donne l'inclinaison du marque-page. */
  index?: number
  /** Heure de composition de l'API : la durée du direct se rend pareil au serveur et au navigateur. */
  now?: IsoDateTime
}

const props = withDefaults(defineProps<Props>(), { showZone: false, fastening: 'tape', index: 0 })

const { t } = useI18n()
const { tr } = useI18nText()
const { intlLocale, time, zoneLabel } = useDateTime()
const { isLive } = useLiveSession()

const title = computed(() => tr(props.session.title))

const { partsOf, namesOf } = useJointSession()

/** Une séance commune nomme toutes ses organisations, comme dans le programme. */
const organization = computed(() => {
  const parts = partsOf(props.session)
  if (parts.length > 1) return namesOf(parts)
  return scheduleOrganizationLabel(props.session)
})

/** « vendredi 13 » : le marque-page a la place de l'écrire en entier. */
const day = computed(() =>
  new Intl.DateTimeFormat(intlLocale.value, {
    weekday: 'long',
    day: 'numeric',
    timeZone: props.session.timezone,
  }).format(new Date(props.session.starts_at)),
)

const live = computed(() => isLive(props.session.id))
const now = useLiveClock(props.now)

/** « depuis 45 min », « depuis 1 h 05 » : le marque-page n'a pas la place d'« en cours ». */
const since = computed(() => {
  const minutes = Math.floor((now.value - Date.parse(props.session.starts_at)) / 60_000)
  if (minutes < 1) return t('home.aside.live.justStarted')
  if (minutes < 60) return t('home.aside.live.since', { minutes })
  const rest = minutes % 60
  return rest
    ? t('home.aside.live.sinceHours', { hours: Math.floor(minutes / 60), minutes: String(rest).padStart(2, '0') })
    : t('home.aside.live.sinceHoursExact', { hours: minutes / 60 })
})

/** Part écoulée du créneau prévu ; un direct qui déborde reste plein. */
const elapsed = computed(() => {
  const start = Date.parse(props.session.starts_at)
  const span = Date.parse(props.session.ends_at) - start
  if (span <= 0) return 1
  return Math.min(1, Math.max(0, (now.value - start) / span))
})

/** En direct, le rouge passe par le style comme la couleur de journée : une classe ne l'emporterait pas sur lui. */
const flagStyle = computed(() => ({
  ...(live.value
    ? { '--paper': 'var(--color-live)', '--paper-ink': 'var(--color-live-contrast)' }
    : trackPaperStyle(props.session.tracks.find((badge) => badge.kind === 'special_day')?.color ?? null)),
  '--tilt': TILTS[props.index % TILTS.length],
}))

const zone = computed(() => (props.showZone ? zoneLabel(props.session.timezone, props.zoneLabel) : ''))
</script>

<template>
  <article data-bird-perch class="aside-row relative grid grid-cols-[4.5rem_minmax(0,1fr)] gap-3.5 py-3">
    <!-- La marge négative vaut le retrait du panneau moins 12 px : le marque-page dépasse du bord. -->
    <p
      class="aside-flag -ml-8 self-start py-2 pr-2 pl-3 font-condensed leading-none uppercase sm:-ml-9"
      :class="[`aside-flag-${props.fastening}`, { 'aside-flag-live': live }]"
      :style="flagStyle"
    >
      <template v-if="live">
        <b class="flex items-center gap-1.5 text-[1.35rem] font-bold tracking-[0.02em]">
          <span class="live-pulse size-2.25 shrink-0 rounded-full bg-live-contrast" aria-hidden="true" />{{ t('home.aside.live.tag') }}
        </b>
        <small class="mt-0.5 block text-[0.85rem] font-semibold tracking-[0.02em] whitespace-nowrap normal-case">{{ since }}</small>
        <span class="mt-1.75 block h-1 overflow-hidden rounded-full bg-live-contrast/35" aria-hidden="true">
          <span class="block h-full bg-live-contrast" :style="{ width: `${elapsed * 100}%` }" />
        </span>
      </template>
      <template v-else>
        <b class="block text-[1.5rem] font-bold tabular-nums">{{ time(props.session.starts_at, props.session.timezone) }}</b>
        <small class="text-[0.72rem] font-semibold tracking-[0.05em]">{{ day }}</small>
      </template>
    </p>

    <div class="min-w-0">
      <h3 class="font-condensed text-[1.12rem] leading-[1.15] font-semibold text-text-on-inverse">
        <!-- Toute la ligne mène à l'activité, pas seulement son titre. -->
        <NuxtLink v-if="props.to" :to="props.to" class="text-text-on-inverse no-underline after:absolute after:inset-0 hover:underline">
          {{ title }}
        </NuxtLink>
        <template v-else>{{ title }}</template>
      </h3>
      <p v-if="props.editionLabel || organization || zone" class="mt-1 text-[0.8rem] text-text-on-inverse-muted">
        <span v-if="props.editionLabel || organization" class="block truncate">
          <span v-if="props.editionLabel" class="font-bold text-text-on-inverse">{{ props.editionLabel }}</span>
          <template v-if="props.editionLabel && organization">, </template>{{ organization }}
        </span>
        <span v-if="zone" class="block">{{ zone }}</span>
      </p>
      <!-- Le marque-page dit déjà « Direct » : une étiquette de plus ferait doublon. Toute la ligne est le lien. -->
      <p v-if="live" class="mt-1.5 text-[0.85rem] font-semibold text-text-on-inverse">
        <span class="flex items-center gap-1.5">{{ t('home.aside.live.join') }}<UiIcon name="arrow-right" size="0.95rem" /></span>
        <span class="block text-[0.75rem] font-normal text-text-on-inverse-muted">
          {{ t('home.aside.live.ends', { time: time(props.session.ends_at, props.session.timezone) }) }}
        </span>
      </p>
      <div v-else-if="props.session.temporal_state === 'ongoing' && props.session.status !== 'completed'" class="mt-1.5">
        <UiStatusBadge state="ongoing" size="sm" :label="t('home.aside.sessions.ongoing')" />
      </div>
    </div>
  </article>
</template>

<style scoped>
.aside-flag {
  position: relative;
  border-radius: 0 3px 3px 0;
  background: var(--paper);
  color: var(--color-text-on-inverse-selected);
  box-shadow:
    0 1px 1px color-mix(in srgb, var(--color-text-on-inverse-selected) 20%, transparent),
    0 6px 10px -6px color-mix(in srgb, var(--color-text-on-inverse-selected) 70%, transparent);
  transform: rotate(var(--tilt));
  transform-origin: 50% 0;
}
.aside-flag small {
  color: var(--paper-ink);
}
.aside-flag-live {
  color: var(--color-live-contrast);
}

/* Ruban adhésif : une bande claire à demi transparente, sans flou (ce n'est pas un panneau de verre). */
.aside-flag-tape::before {
  content: '';
  position: absolute;
  top: -8px;
  left: 50%;
  width: 46px;
  height: 16px;
  transform: translateX(-50%) rotate(-4deg);
  background: color-mix(in srgb, var(--color-surface-inverse-selected) 50%, transparent);
  border: 1px solid color-mix(in srgb, var(--color-surface-inverse-selected) 45%, transparent);
}

/* Épingle : tête bleu nuit éclaircie, reflet et ombre portée. Jamais rouge, couleur du direct. */
.aside-flag-pin {
  padding-top: 0.75rem;
}
.aside-flag-pin::before {
  content: '';
  position: absolute;
  top: -6px;
  left: 50%;
  width: 14px;
  height: 14px;
  margin-left: -7px;
  border-radius: 50%;
  background: color-mix(in srgb, var(--color-surface-inverse) 70%, var(--color-text-on-inverse));
  box-shadow:
    inset -2px -2px 0 color-mix(in srgb, var(--color-text-on-inverse-selected) 35%, transparent),
    inset 2px 2px 0 color-mix(in srgb, var(--color-text-on-inverse) 35%, transparent),
    2px 3px 3px color-mix(in srgb, var(--color-text-on-inverse-selected) 50%, transparent);
}

@media (prefers-reduced-motion: no-preference) {
  .aside-flag {
    transition: transform 160ms ease-out;
  }
  .aside-row:has(a:hover) .aside-flag,
  .aside-row:has(a:focus-visible) .aside-flag {
    transform: rotate(0);
  }
}
</style>
