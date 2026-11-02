<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'

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
 */

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
}

const props = withDefaults(defineProps<Props>(), { showZone: false })

const { t } = useI18n()
const { tr } = useI18nText()
const { intlLocale, time, zoneLabel } = useDateTime()
const { isLive } = useLiveSession()

const title = computed(() => tr(props.session.title))

const organization = computed(
  () => props.session.organization_acronym ?? props.session.organization_name ?? '',
)

/** « ven. 13 » */
const day = computed(() =>
  new Intl.DateTimeFormat(intlLocale.value, {
    weekday: 'short',
    day: 'numeric',
    timeZone: props.session.timezone,
  }).format(new Date(props.session.starts_at)),
)

const zone = computed(() => (props.showZone ? zoneLabel(props.session.timezone, props.zoneLabel) : ''))
</script>

<template>
  <article data-bird-perch class="relative grid grid-cols-[4rem_minmax(0,1fr)] gap-3 py-3">
    <p class="font-condensed leading-none uppercase">
      <b class="block text-[1.6rem] font-bold tabular-nums">{{ time(props.session.starts_at, props.session.timezone) }}</b>
      <small class="text-[0.78rem] font-semibold tracking-[0.05em] text-text-on-inverse-muted">{{ day }}</small>
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
        <span v-if="props.editionLabel" class="font-bold text-text-on-inverse">{{ props.editionLabel }}</span>
        <template v-if="props.editionLabel && organization">, </template>{{ organization }}
        <span v-if="zone" class="block">{{ zone }}</span>
      </p>
      <div v-if="isLive(props.session.id) || props.session.temporal_state === 'ongoing'" class="mt-1.5">
        <UiLiveBadge :session-id="props.session.id" size="sm" />
        <UiStatusBadge
          v-if="!isLive(props.session.id)"
          state="ongoing"
          size="sm"
          :label="t('home.aside.sessions.ongoing')"
        />
      </div>
    </div>
  </article>
</template>
