<script setup lang="ts">
import type { PublicTrackEntry, PublicTrackPage } from '~/types/programme/track-page'
import type { SessionDisplayState } from '~/components/ui/StatusBadge.vue'
import type { TrackDayPhase } from '~/composables/useTrackDay'

/**
 * Le billet d'une journée spéciale : les heures de la journée dans le fuseau de
 * l'édition, puis chez le lecteur, et LE geste du moment. Avant : l'ajouter à
 * son agenda. Le jour J : ce qui se passe maintenant. Après : les
 * enregistrements, ou l'encart (le Guide) pour l'atelier. L'atelier, hors du
 * pavillon, y porte son plan d'accès.
 */

interface Props {
  page: PublicTrackPage
  phase: TrackDayPhase
  countdown: string
  current: { entry: PublicTrackEntry; state: SessionDisplayState } | null
  /** Monté dans le navigateur : l'heure chez le lecteur ne se connaît qu'ici. */
  mounted: boolean
  dayTitle: string
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { intlLocale, time, zoneLabel, zoneOffsetShort } = useDateTime()
const { link } = useProgrammeSession()
const localePath = useLocalePath()
const { copied, share, url } = useShareLink(() => props.dayTitle)

const timezone = computed(() => props.page.edition.timezone)
const venue = computed(() => props.page.venue)
const venueName = computed(() => (venue.value ? tr(venue.value.name) : ''))

const eyebrow = computed(() =>
  [venueName.value, props.page.is_off_site ? props.page.edition.city : venue.value?.area ? tr(venue.value.area) : '']
    .filter(Boolean)
    .join(' · '),
)

const start = computed(() => props.page.starts_at)
const end = computed(() => props.page.ends_at)

const span = computed(() => {
  if (!start.value || !end.value) return ''
  const parts = durationParts(Math.round((Date.parse(end.value) - Date.parse(start.value)) / 60_000))
  const length = !parts
    ? ''
    : !parts.hours
      ? t('programme.list.minutes', { minutes: parts.minutes })
      : parts.minutes
        ? t('programme.list.hoursMinutes', { hours: parts.hours, minutes: String(parts.minutes).padStart(2, '0') })
        : t('programme.list.hours', { hours: parts.hours })
  return t('programme.list.until', { end: time(end.value, timezone.value), duration: length })
})

const zone = computed(() =>
  t('track.ticket.zone', {
    zone: zoneLabel(timezone.value, props.page.edition.city ?? undefined),
    offset: zoneOffsetShort(timezone.value, start.value ?? undefined),
  }),
)
const opens = computed(() => (props.page.opens_at ? t('track.ticket.opens', { time: time(props.page.opens_at, timezone.value) }) : ''))

/** « Chez vous (Paris, UTC+1) : 07:00 → 16:00 », seulement si l'heure diffère. */
const homeTime = computed(() => {
  if (!props.mounted || !start.value || !end.value) return ''
  const visitor = Intl.DateTimeFormat().resolvedOptions().timeZone
  if (!visitor || visitor === timezone.value) return ''
  const range = `${time(start.value, visitor)} → ${time(end.value, visitor)}`
  if (range === `${time(start.value, timezone.value)} → ${time(end.value, timezone.value)}`) return ''
  return t('track.ticket.home', { zone: `${timeZoneCityLabel(visitor)}, ${zoneOffsetShort(visitor, start.value)}`, range })
})

const firstDay = computed(() => {
  const first = props.page.opens_at ?? start.value
  if (first) return formatDate(first, { locale: intlLocale.value, timeZone: timezone.value, dateStyle: 'full' })
  return props.page.track.starts_on
    ? formatDate(`${props.page.track.starts_on}T12:00:00Z`, { locale: intlLocale.value, timeZone: 'UTC', dateStyle: 'full' })
    : ''
})

const now = computed(() => {
  const item = props.current
  if (!item || props.phase !== 'during') return null
  const entry = item.entry
  const since = time(entry.starts_at, timezone.value)
  if (entry.kind === 'moment') {
    return { live: false, label: t('track.ticket.now', { time: since }), title: tr(entry.moment.label), to: null, note: '' }
  }
  const session = entry.activity.session
  const live = item.state === 'live'
  return {
    live,
    label: live ? t('track.ticket.liveSince', { time: since }) : t('track.ticket.now', { time: since }),
    title: tr(session.title),
    to: live ? `${link(props.page.edition.slug, session)}#direct` : link(props.page.edition.slug, session),
    note: session.is_streamed ? '' : t('track.ticket.notStreamed'),
  }
})

const hasReplays = computed(() =>
  props.page.agenda.some((entry) => entry.kind === 'activity' && entry.activity.session.replay_url),
)

const mapUrl = computed(() => {
  const value = venue.value
  if (!value) return null
  if (value.map_url) return value.map_url
  if (value.latitude === null || value.longitude === null) return null
  return `https://www.openstreetmap.org/?mlat=${value.latitude}&mlon=${value.longitude}#map=17/${value.latitude}/${value.longitude}`
})

/** Le plan d'OpenStreetMap, quand le lieu a été relevé. */
const mapEmbed = computed(() => {
  const value = venue.value
  if (!value || value.latitude === null || value.longitude === null) return null
  const d = 0.006
  const box = [value.longitude - d, value.latitude - d / 2, value.longitude + d, value.latitude + d / 2].join(',')
  return `https://www.openstreetmap.org/export/embed.html?bbox=${box}&layer=mapnik&marker=${value.latitude},${value.longitude}`
})

const featureTo = computed(() => {
  const target = props.page.feature?.link_url
  if (!target) return null
  return target.startsWith('/') ? { to: localePath(target) } : { href: target }
})

const addressCopied = ref(false)
async function copyAddress(): Promise<void> {
  const text = [venueName.value, venue.value?.address].filter(Boolean).join(', ')
  await navigator.clipboard.writeText(text)
  addressCopied.value = true
  setTimeout(() => (addressCopied.value = false), 2500)
}

function addToCalendar(): void {
  const first = props.page.opens_at ?? start.value
  if (!first || !end.value) return
  downloadIcs(
    {
      uid: `${props.page.track.id}@epavillon`,
      title: props.dayTitle,
      startsAt: first,
      endsAt: end.value,
      location: venue.value?.address
        ? `${venueName.value}, ${venue.value.address}`
        : [venueName.value, venue.value?.area ? tr(venue.value.area) : '', props.page.edition.city].filter(Boolean).join(', '),
      url: url.href,
    },
    props.page.track.slug,
  )
}

const partnerLogo = (logo: PublicTrackPage['partners'][number]['logo']) => (logo ? (logo.sources.thumb?.url ?? logo.url) : null)
</script>

<template>
  <UiTicket :label="t('track.ticket.label')" bar-class="bg-(--color-day-line)">
    <p class="text-xs text-text-muted uppercase" :style="{ letterSpacing: 'var(--tracking-caps)' }">{{ eyebrow }}</p>

    <template v-if="start">
      <p class="mt-2.5 flex flex-wrap items-end gap-x-3 gap-y-1">
        <span class="text-[3.25rem] leading-[0.9] font-light tabular-nums">{{ time(start, timezone) }}</span>
        <span class="pb-0.5 text-sm text-text-muted tabular-nums">{{ span }}</span>
      </p>
      <p class="mt-2 text-sm text-text-muted first-letter:uppercase">
        {{ zone }}<template v-if="opens"> · {{ opens }}</template>
      </p>
      <p v-if="homeTime" class="mt-1 text-sm text-text-muted">{{ homeTime }}</p>
    </template>

    <p v-if="firstDay" class="mt-4 font-bold">
      <span class="first-letter:uppercase">{{ firstDay }}</span>
      <span v-if="props.countdown" class="text-sm font-normal text-text-muted"> · {{ props.countdown }}</span>
    </p>

    <div
      v-if="now"
      class="mt-4 rounded-md border p-4"
      :class="now.live ? 'border-live-border bg-live-surface' : 'border-warning-border bg-warning-surface'"
    >
      <p
        class="text-xs font-bold uppercase"
        :class="now.live ? 'text-live' : 'text-warning'"
        :style="{ letterSpacing: 'var(--tracking-caps)' }"
      >
        {{ now.label }}
      </p>
      <p class="mt-1 leading-snug font-bold">{{ now.title }}</p>
      <p v-if="now.note" class="mt-1 text-sm text-text-muted">{{ now.note }}</p>
      <UiButton v-if="now.live && now.to" class="mt-3" block icon="broadcast" :to="now.to">
        {{ t('track.ticket.watch') }}
      </UiButton>
      <UiButton v-else-if="now.to" class="mt-3" block variant="secondary" icon-trailing="arrow-right" :to="now.to">
        {{ t('track.highlight.open') }}
      </UiButton>
    </div>

    <div class="mt-4 flex flex-col gap-2">
      <UiButton v-if="props.phase === 'before' && start && end" block icon="calendar" @click="addToCalendar">
        {{ t('track.ticket.calendar') }}
      </UiButton>
      <UiButton v-else-if="props.phase === 'during' && props.page.is_off_site && mapUrl" block icon="map-pin" :href="mapUrl">
        {{ t('track.ticket.route') }}
      </UiButton>
      <UiButton v-else-if="props.phase === 'after' && hasReplays" block icon="video" href="#deroule">
        {{ t('track.ticket.replays') }}
      </UiButton>
      <UiButton
        v-else-if="props.phase === 'after' && props.page.feature?.link_label && featureTo"
        block
        icon="document"
        v-bind="featureTo"
      >
        {{ tr(props.page.feature.link_label) }}
      </UiButton>

      <div class="grid gap-2" :class="props.page.is_off_site || mapUrl ? 'grid-cols-2' : 'grid-cols-1'">
        <UiButton variant="secondary" :icon="copied ? 'check' : 'copy'" @click="share">
          {{ t(copied ? 'track.ticket.copied' : 'track.ticket.share') }}
        </UiButton>
        <UiButton v-if="props.page.is_off_site" variant="secondary" icon="map-pin" href="#acces">
          {{ t('track.ticket.access') }}
        </UiButton>
        <UiButton v-else-if="mapUrl" variant="secondary" icon="map-pin" :href="mapUrl">
          {{ t('track.ticket.accessShort') }}
        </UiButton>
      </div>
    </div>

    <div v-if="props.page.is_off_site && venue" id="acces" class="mt-5 scroll-mt-28 border-t border-border-subtle pt-5">
      <p class="text-xs font-bold text-text-muted uppercase" :style="{ letterSpacing: 'var(--tracking-caps)' }">
        {{ t('track.ticket.map') }}
      </p>
      <!-- 4/3 : l'attribution d'OpenStreetMap, obligatoire, couvrait un tiers du cadre en 16/10 à 375 px. -->
      <iframe
        v-if="mapEmbed"
        :src="mapEmbed"
        :title="t('track.ticket.mapOf', { place: venueName })"
        loading="lazy"
        referrerpolicy="no-referrer"
        class="mt-3 aspect-4/3 w-full rounded-lg border border-border-subtle bg-surface-sunken"
      />
      <address class="mt-3 leading-snug not-italic">
        <b class="block">{{ venueName }}</b>
        <span v-if="venue.address">{{ venue.address }}</span>
      </address>
      <p v-if="venue.access_note" class="mt-2 text-sm text-text-muted">{{ tr(venue.access_note) }}</p>
      <p class="mt-1 flex flex-wrap gap-x-5">
        <UiButton v-if="mapUrl" variant="link" icon="external-link" :href="mapUrl">{{ t('track.ticket.itinerary') }}</UiButton>
        <UiButton v-if="venue.address" variant="link" :icon="addressCopied ? 'check' : 'copy'" @click="copyAddress">
          {{ t(addressCopied ? 'track.ticket.addressCopied' : 'track.ticket.copyAddress') }}
        </UiButton>
      </p>
    </div>

    <div v-if="props.page.partners.length || props.page.support_note" class="mt-5 border-t border-border-subtle pt-5">
      <p class="text-xs font-bold text-text-muted uppercase" :style="{ letterSpacing: 'var(--tracking-caps)' }">
        {{ t('track.ticket.with') }}
      </p>
      <ul v-if="props.page.partners.length" class="mt-3 grid grid-cols-2 gap-2">
        <li v-for="partner in props.page.partners" :key="partner.organization_id">
          <OrganizationLogoTile :name="partner.name" :acronym="partner.acronym" :logo="partnerLogo(partner.logo)" :role="tr(partner.role_label)" compact />
        </li>
      </ul>
      <p v-if="props.page.support_note" class="mt-3 text-sm text-text-muted">{{ tr(props.page.support_note) }}</p>
    </div>
  </UiTicket>
</template>
