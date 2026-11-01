<script setup lang="ts">
import type { PublicTrackEntry, PublicTrackPage } from '~/types/programme/track-page'
import type { SessionDisplayState } from '~/components/ui/StatusBadge.vue'
import type { TrackDayPhase } from '~/composables/useTrackDay'

/**
 * Le billet d'une journée spéciale : la date en bande à la couleur de la
 * journée, les heures dans le fuseau de l'édition puis chez le lecteur, et LE
 * geste du moment. Avant : l'ajouter à son agenda. Le jour J : ce qui se passe
 * maintenant, avec « Rejoindre le direct ». Après : les enregistrements, ou
 * l'encart (le Guide) pour l'atelier. L'atelier, hors du pavillon, y porte son
 * plan d'accès.
 *
 * Collé à l'écran sur grand écran, sa hauteur est bornée : seuls le plan et
 * les partenaires défilent, la date, les heures et les boutons restent visibles.
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
const { time, zoneLabel, zoneOffsetShort } = useDateTime()
const { link } = useProgrammeSession()
const localePath = useLocalePath()
const { copied, share, url } = useShareLink(() => props.dayTitle)

const timezone = computed(() => props.page.edition.timezone)
const venue = computed(() => props.page.venue)
const venueName = computed(() => (venue.value ? tr(venue.value.name) : ''))
const icon = computed(() => trackIcon(props.page.track.icon_code))

const start = computed(() => props.page.starts_at)
const end = computed(() => props.page.ends_at)

/** Le jour de la bande : la première ligne, à défaut la date annoncée (lue en UTC, c'est une date). */
const bandDate = computed(() => {
  const first = props.page.opens_at ?? start.value
  if (first) return { date: first, timezone: timezone.value }
  return props.page.track.starts_on ? { date: `${props.page.track.starts_on}T12:00:00Z`, timezone: 'UTC' } : null
})

const hours = computed(() =>
  start.value && end.value ? t('track.ticket.hours', { start: time(start.value, timezone.value), end: time(end.value, timezone.value) }) : '',
)

const zone = computed(() => {
  const line = t('track.ticket.zone', {
    zone: zoneLabel(timezone.value, props.page.edition.city ?? undefined),
    offset: zoneOffsetShort(timezone.value, start.value ?? undefined),
  })
  return props.page.opens_at ? t('track.ticket.zoneOpens', { zone: line, time: time(props.page.opens_at, timezone.value) }) : line
})

/** « Chez vous, à Abidjan (UTC+0) : 05:00 à 15:00 », seulement si l'heure diffère. */
const homeTime = computed(() => {
  if (!props.mounted || !start.value || !end.value) return ''
  const visitor = Intl.DateTimeFormat().resolvedOptions().timeZone
  if (!visitor || visitor === timezone.value) return ''
  if (time(start.value, visitor) === time(start.value, timezone.value) && time(end.value, visitor) === time(end.value, timezone.value)) return ''
  return t('track.ticket.home', {
    city: timeZoneCityLabel(visitor),
    offset: zoneOffsetShort(visitor, start.value),
    range: t('track.ticket.hours', { start: time(start.value, visitor), end: time(end.value, visitor) }),
  })
})

const now = computed(() => {
  const item = props.current
  if (!item || props.phase !== 'during') return null
  const entry = item.entry
  const since = time(entry.starts_at, timezone.value)
  const until = entry.ends_at ? time(entry.ends_at, timezone.value) : ''
  if (entry.kind === 'moment') {
    return { live: false, label: t('track.ticket.now', { time: since }), title: tr(entry.moment.label), by: '', to: null, note: '' }
  }
  const session = entry.activity.session
  const lead = entry.activity.organizations[0]
  const organization = lead?.acronym ?? lead?.name ?? session.organization_acronym ?? session.organization_name ?? ''
  const live = item.state === 'live'
  return {
    live,
    label: live ? t('track.ticket.liveSince', { time: since }) : t('track.ticket.now', { time: since }),
    title: tr(session.title),
    by: until
      ? organization
        ? t('track.ticket.byUntil', { organization, time: until })
        : t('track.ticket.until', { time: until })
      : organization,
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

const hasMore = computed(() => (props.page.is_off_site && venue.value) || props.page.partners.length || props.page.support_note)
</script>

<template>
  <UiTicket :label="t('track.ticket.label')" bar-class="bg-(--color-day-line)" class="lg:max-h-[calc(100dvh-var(--nav-height)-2rem)]">
    <template v-if="bandDate" #band>
      <EventTicketDate :date="bandDate.date" :timezone="bandDate.timezone">
        <template v-if="icon" #icon>
          <UiIcon :name="icon" size="2.25rem" :stroke-width="1.6" />
        </template>
      </EventTicketDate>
    </template>

    <div class="shrink-0">
      <template v-if="start">
        <p class="font-condensed text-[2.1rem] leading-none font-bold text-heading tabular-nums">{{ hours }}</p>
        <p class="mt-1.5 text-sm text-text-secondary first-letter:uppercase">{{ zone }}</p>
        <p v-if="homeTime" class="mt-1 text-sm text-text-secondary">{{ homeTime }}</p>
      </template>
      <p v-if="props.countdown" class="text-sm text-text-muted first-letter:uppercase" :class="start ? 'mt-1' : ''">{{ props.countdown }}</p>

      <div
        v-if="now"
        class="mt-4 rounded-md border p-3"
        :class="now.live ? 'border-live-border bg-live-surface' : 'border-warning-border bg-warning-surface'"
      >
        <UiStatusBadge v-if="now.live" state="live" size="sm" :label="now.label" />
        <p v-else class="text-xs font-bold text-warning uppercase" :style="{ letterSpacing: 'var(--tracking-caps)' }">{{ now.label }}</p>
        <p class="mt-2 text-sm leading-snug">
          <b class="block">{{ now.title }}</b>
          <template v-if="now.by">{{ now.by }}</template>
        </p>
        <p v-if="now.note" class="mt-1 text-sm text-text-muted">{{ now.note }}</p>
        <TrackLiveLink v-if="now.live && now.to" class="mt-2.5" block :to="now.to" :label="t('track.ticket.watch')" />
        <UiButton v-else-if="now.to" class="mt-2.5" block variant="secondary" icon-trailing="arrow-right" :to="now.to">
          {{ t('track.highlight.open') }}
        </UiButton>
      </div>

      <div class="mt-4 grid grid-cols-2 gap-2.5">
        <UiButton v-if="props.phase === 'before' && start && end" class="col-span-2" block icon="calendar" @click="addToCalendar">
          {{ t('track.ticket.calendar') }}
        </UiButton>
        <UiButton v-else-if="props.phase === 'during' && props.page.is_off_site && mapUrl" class="col-span-2" block icon="map-pin" :href="mapUrl">
          {{ t('track.ticket.route') }}
        </UiButton>
        <UiButton v-else-if="props.phase === 'after' && hasReplays" class="col-span-2" block icon="video" href="#deroule">
          {{ t('track.ticket.replays') }}
        </UiButton>
        <UiButton
          v-else-if="props.phase === 'after' && props.page.feature?.link_label && featureTo"
          class="col-span-2"
          block
          icon="document"
          v-bind="featureTo"
        >
          {{ tr(props.page.feature.link_label) }}
        </UiButton>

        <UiButton v-if="props.page.is_off_site" variant="secondary" block icon="map-pin" href="#acces">
          {{ t('track.ticket.access') }}
        </UiButton>
        <UiButton v-else-if="mapUrl" variant="secondary" block icon="map-pin" :href="mapUrl">
          {{ t('track.ticket.itinerary') }}
        </UiButton>
        <UiButton
          variant="secondary"
          block
          :icon="copied ? 'check' : 'copy'"
          :class="props.page.is_off_site || mapUrl ? '' : 'col-span-2'"
          @click="share"
        >
          {{ t(copied ? 'track.ticket.copied' : 'track.ticket.share') }}
        </UiButton>
      </div>
    </div>

    <!-- Ce qui peut dépasser l'écran : défile seul dans le billet collé. -->
    <div v-if="hasMore" class="-mx-6 -mb-6 mt-5 min-h-0 flex-auto border-t border-border-subtle px-6 pt-4 pb-5 lg:overflow-y-auto lg:overscroll-contain">
      <div v-if="props.page.is_off_site && venue" id="acces" class="scroll-mt-28">
        <h3 class="heading-condensed text-[1.05rem] tracking-[0.05em] text-text-secondary">{{ t('track.ticket.map') }}</h3>
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

      <div v-if="props.page.partners.length || props.page.support_note" :class="props.page.is_off_site && venue ? 'mt-5 border-t border-border-subtle pt-4' : ''">
        <h3 class="heading-condensed text-[1.05rem] tracking-[0.05em] text-text-secondary">{{ t('track.ticket.with') }}</h3>
        <ul v-if="props.page.partners.length" class="mt-1.5">
          <li v-for="partner in props.page.partners" :key="partner.organization_id" class="flex items-center gap-3 py-2">
            <OrganizationLogoMark :name="partner.name" :acronym="partner.acronym" :logo="partner.logo" />
            <p class="min-w-0 leading-snug">
              <b class="block text-[0.9rem]">{{ partner.name }}</b>
              <small class="text-[0.78rem] text-text-muted">{{ tr(partner.role_label) }}</small>
            </p>
          </li>
        </ul>
        <p v-if="props.page.support_note" class="mt-2 text-sm text-text-muted">{{ tr(props.page.support_note) }}</p>
      </div>
    </div>
  </UiTicket>
</template>
