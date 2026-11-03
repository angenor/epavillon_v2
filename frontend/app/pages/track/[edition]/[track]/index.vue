<script setup lang="ts">
import type { PublicSessionSpeaker } from '~/types/programme/session'
import type { PublicTrackActivity, PublicTrackEntry } from '~/types/programme/track-page'

/**
 * LA PAGE D'UNE JOURNÉE SPÉCIALE, ou de l'atelier préparatoire (un fil
 * `side_programme`) : la grande sœur de la page d'une activité (direction
 * « bandeau et billet », 30/10, passée en direction condensée le 01/11). Même
 * bandeau, même billet qui remonte de 72 px sur grand écran, collé et borné à
 * la hauteur de l'écran.
 *
 * L'adresse porte l'édition et le fil par leurs slugs. Un fil non publié et une
 * adresse inconnue donnent la même réponse, comme l'API.
 *
 * La couleur de la journée (`color_hex`) se décline en rôles sur la racine de
 * la page (`trackDayStyle`) ; l'atelier, sans couleur ni photo, garde le bleu
 * nuit institutionnel.
 *
 * Une journée se suit par un seul direct (01/11) : on s'inscrit à la journée,
 * pas à ses activités ; le billet porte l'inscription et, le jour même, le seul
 * « Rejoindre », qui mène à la salle de la journée (`room.vue`).
 */

definePageMeta({ layout: 'public' })
defineI18nRoute({ paths: { fr: '/programmations/[edition]/journees/[track]', en: '/programmes/[edition]/days/[track]' } })

const route = useRoute()
const api = useApi()
const { t } = useI18n()
const { tr } = useI18nText()
const localePath = useLocalePath()
const { state: sessionState } = useProgrammeSession()
const { zoneLabel, zoneOffsetShort } = useDateTime()

const editionSlug = computed(() => String(route.params.edition))
const trackSlug = computed(() => String(route.params.track))

const { data, status, error, refresh } = await useAsyncData(
  () => `journee-${editionSlug.value}-${trackSlug.value}`,
  () => api.sessions.trackPage(editionSlug.value, trackSlug.value),
  { lazy: true },
)

if (import.meta.server && status.value === 'success' && !data.value) setResponseStatus(useRequestEvent()!, 404)

const page = computed(() => data.value ?? null)

// La page du fil ne porte pas la photo de l'édition, repli des activités sans couverture : la liste des éditions, si.
const { data: editions } = useLazyAsyncData('journee-editions', () => api.events.publicList().catch(() => []), { default: () => [] })
const currentEdition = computed(() => editions.value.find((edition) => edition.slug === editionSlug.value))
const editionImage = computed(() => editionPhoto(currentEdition.value))

const { declareFrom } = useLiveSession()
watch(
  page,
  (value) => value && declareFrom(value.agenda.flatMap((entry) => (entry.kind === 'activity' ? [entry.activity.session] : []))),
  { immediate: true },
)
const { now, mounted, timezone, phase, countdown, entryState, current } = useTrackDay(page)

const { mine } = useDayRegistration(() => page.value?.track.id ?? null)
const kept = computed(() => (mine.value ? new Set(mine.value.session_ids) : null))
const dayLive = computed(() => (page.value ? isDayLive(page.value) : false))
const { meeting } = useTrackMeeting(() => (page.value ? { id: page.value.track.id, eventId: page.value.edition.id, expected: dayLive.value } : null))
// Direct démarré : « Rejoindre » mène à la salle, qui attend la réunion en création.
const meetingJoinable = computed(() => isMeetingJoinable(meeting.value) || dayLive.value)
const roomTo = computed(() =>
  localePath({ name: 'track-edition-track-room', params: { edition: editionSlug.value, track: trackSlug.value }, query: { entrer: '1' } }),
)

const { data: incidents, refresh: refreshIncidents } = useLazyAsyncData(
  () => `journee-incidents-${page.value?.track.id ?? 'aucune'}`,
  async () => (page.value ? api.incidents.forTrack(page.value.edition.id, page.value.track.id).catch(() => []) : []),
  { default: () => [], watch: [page] },
)

/** Au signal : la journée se relit sans squelette ni message ; une panne garde l'affichage. */
async function reloadQuietly(): Promise<void> {
  try {
    data.value = await api.sessions.trackPage(editionSlug.value, trackSlug.value)
  } catch {
    // Le prochain signal rattrape.
  }
}
const editionId = computed(() => page.value?.edition.id)
useRealtime(() => publicEditionTopics([editionId.value], PROGRAMME_CHANNELS), () => reloadQuietly())
useRealtime(() => publicEditionTopics([editionId.value], ['incidents']), () => refreshIncidents())

const editionLabel = computed(() => (page.value ? (page.value.edition.acronym ?? tr(page.value.edition.title)) : ''))
const title = computed(() => (page.value ? tr(page.value.track.title) : ''))
const dayStyle = computed(() => trackDayStyle(page.value?.track.kind === 'special_day' ? page.value.track.color_hex : null))

const programmeTo = computed(() =>
  localePath({
    name: 'programme',
    query: page.value
      ? { edition: page.value.edition.slug, jour: page.value.starts_at ? dayKeyInZone(page.value.starts_at, timezone.value) : undefined }
      : {},
  }),
)

/** « Veille de la CdP31 », « Avant la CdP31 » : le fil se tient avant l'ouverture de l'édition. */
const eve = computed(() => {
  const value = page.value
  const first = value?.opens_at ?? value?.starts_at
  if (!value || !first) return ''
  const day = dayKeyInZone(first, timezone.value)
  const opening = dayKeyInZone(value.edition.starts_at, timezone.value)
  if (day >= opening) return ''
  const gap = Math.round((Date.parse(`${opening}T00:00:00Z`) - Date.parse(`${day}T00:00:00Z`)) / 86_400_000)
  return t(gap === 1 ? 'track.eve' : 'track.before', { edition: editionLabel.value })
})

const live = computed(() => {
  const item = current.value
  return item?.state === 'live' && item.entry.kind === 'activity' ? item.entry.activity.session : null
})

const activities = computed(() =>
  (page.value?.agenda ?? []).flatMap((entry) => (entry.kind === 'activity' ? [entry.activity] : [])),
)
const highlight = computed(() => activities.value.find((activity) => activity.is_highlight && sessionState(activity.session) !== 'cancelled') ?? null)
/** Un moment temps fort (le hackathon, 02/11) ; une activité temps fort passe devant. */
const highlightMoment = computed(() =>
  highlight.value
    ? null
    : ((page.value?.agenda ?? []).find(
        (entry): entry is Extract<PublicTrackEntry, { kind: 'moment' }> => entry.kind === 'moment' && entry.moment.is_highlight,
      ) ?? null),
)

/** Dans l'ordre du déroulé, avec les activités qu'un moment regroupe : leurs intervenants et leurs organisations sont de la journée. */
type Speaking = Pick<PublicTrackActivity, 'speakers' | 'organizations'>
const speaking = computed(() =>
  (page.value?.agenda ?? []).flatMap<Speaking>((entry) =>
    entry.kind === 'activity' ? [entry.activity] : entry.kind === 'moment' ? entry.moment.activities : [],
  ),
)

const speakers = computed(() => {
  const seen = new Set<string>()
  const list: PublicSessionSpeaker[] = []
  for (const activity of speaking.value) {
    for (const speaker of activity.speakers) {
      if (seen.has(speaker.display_name)) continue
      seen.add(speaker.display_name)
      list.push(speaker)
    }
  }
  return list
})

const organizations = computed(() => {
  const seen = new Set<string>()
  return speaking.value
    .flatMap((activity) => activity.organizations)
    .filter((organization) => (seen.has(organization.organization_id) ? false : (seen.add(organization.organization_id), true)))
    .map((organization) => ({
      id: organization.organization_id,
      name: organization.name ?? organization.acronym ?? '',
      acronym: organization.acronym ?? null,
      logo: organization.logo ?? null,
    }))
})

const sectionTitle = 'heading-condensed text-[clamp(1.9rem,3.6vw,2.5rem)] text-heading'
const subject = computed(() => (page.value?.track.kind === 'special_day' ? 'day' : 'other'))
const hasTiles = computed(() => (page.value?.parts.filter((part) => part.starts_at).length ?? 0) > 1)
const paragraphs = computed(() =>
  (page.value?.track.description ? tr(page.value.track.description) : '')
    .split(/\n{2,}/)
    .map((text) => text.trim())
    .filter(Boolean),
)
const zoneNote = computed(() => {
  const value = page.value
  if (!value) return ''
  return t('track.agenda.zone', {
    zone: zoneLabel(timezone.value, value.edition.city ?? undefined),
    offset: zoneOffsetShort(timezone.value, value.starts_at ?? undefined),
  })
})

const summary = computed(() => {
  const value = page.value
  if (!value) return ''
  return value.track.subtitle ? tr(value.track.subtitle) : (paragraphs.value[0] ?? '')
})

useHead(() => ({
  title: title.value || t('track.notFound.title'),
  meta: summary.value ? [{ name: 'description', content: summary.value }] : [],
}))

// L'aperçu d'un lien partagé : la photo du bandeau, à défaut le visuel de l'encart.
const requestUrl = useRequestURL()
useSeoMeta({
  ogType: 'website',
  ogUrl: () => requestUrl.href,
  ogTitle: () => title.value || undefined,
  ogDescription: () => summary.value || undefined,
  ogImage: () => {
    const image = page.value?.cover ?? page.value?.feature?.visual
    if (!image) return undefined
    return new URL(image.sources.partage?.url ?? image.sources.md?.url ?? image.url, requestUrl.origin).href
  },
  ogImageAlt: () => {
    const image = page.value?.cover ?? page.value?.feature?.visual
    return image ? tr(image.alt_text) : undefined
  },
  twitterCard: 'summary_large_image',
})
</script>

<template>
  <div data-direction="condensee" class="full-bleed -mt-8 -mb-8 min-h-[75vh] bg-surface font-sans text-text sm:-mt-10 sm:-mb-10" :style="dayStyle">
    <UiLoadingState v-if="status === 'pending' && !data" :label="t('track.loading')">
      <ActivitySkeleton />
    </UiLoadingState>

    <div v-else-if="error" class="mx-auto max-w-[1280px] px-4 py-10 sm:px-6">
      <UiErrorState :title="t('track.error.title')" :description="t('track.error.description')" @retry="refresh()" />
    </div>

    <div v-else-if="!page" class="mx-auto max-w-[1280px] px-4 py-10 sm:px-6">
      <UiEmptyState
        icon="calendar"
        :title="t('track.notFound.title')"
        :description="t('track.notFound.description')"
        :action-label="t('track.notFound.back')"
        :action-to="localePath('programme')"
      />
    </div>

    <template v-else>
      <EventIncidentStrip :incidents="incidents" />
      <TrackHero
        :page="page"
        :phase="phase"
        :back-to="programmeTo"
        :eve="eve"
        :live="live !== null"
        :edition-logo="currentEdition?.logo_inverse ?? null"
      />

      <div class="mx-auto max-w-[1280px] px-4 pb-22 sm:px-6">
        <!-- Le billet et la carte des temps ne remontent sur le bandeau que sur grand écran : sur
             téléphone, une marge négative raccourcit la ligne de grille et la suite passe dessous. -->
        <div class="relative grid gap-8 pt-6 lg:-mt-18 lg:grid-cols-[minmax(0,1fr)_380px] lg:items-start lg:gap-12 lg:pt-0">
          <div class="lg:sticky lg:top-[calc(var(--nav-height)+1rem)] lg:col-start-2 lg:row-start-1">
            <TrackTicket
              :page="page"
              :phase="phase"
              :countdown="countdown"
              :current="current"
              :mounted="mounted"
              :day-title="title"
              :room-to="roomTo"
              :meeting-joinable="meetingJoinable"
            />
          </div>

          <div class="flex min-w-0 flex-col gap-14 lg:col-start-1 lg:row-start-1">
            <TrackOverview :page="page" :live="live" :edition-image="editionImage" />

            <TrackFeature v-if="page.feature" :feature="page.feature" :documents="page.documents" />

            <section v-if="hasTiles && paragraphs.length" aria-labelledby="journee-presentation-titre">
              <h2 id="journee-presentation-titre" :class="sectionTitle">{{ t(`track.about.${subject}`) }}</h2>
              <p
                v-for="(paragraph, index) in paragraphs"
                :key="index"
                class="max-w-[66ch] text-[1.05rem] leading-[1.75]"
                :class="index ? 'mt-3.5' : 'mt-4.5'"
              >
                {{ paragraph }}
              </p>
            </section>

            <TrackHighlight
              v-if="highlight || highlightMoment"
              :activity="highlight"
              :moment="highlightMoment"
              :edition-slug="page.edition.slug"
              :timezone="timezone"
            />

            <section v-if="page.agenda.length" id="deroule" class="scroll-mt-28" aria-labelledby="journee-deroule-titre">
              <h2 id="journee-deroule-titre" :class="sectionTitle">{{ t(`track.agenda.title.${subject}`) }}</h2>
              <p class="mt-2.5 max-w-[66ch] text-[0.95rem] text-text-secondary">{{ zoneNote }}</p>
              <TrackAgenda :page="page" :entry-state="entryState" :edition-image="editionImage" :kept="kept" :incidents="incidents" />
            </section>
            <UiEmptyState
              v-else
              icon="calendar"
              :title="t('track.agenda.empty.title')"
              :description="t('track.agenda.empty.description')"
            />

            <ActivityPeople v-if="speakers.length" :speakers="speakers" />

            <section v-if="organizations.length" aria-labelledby="journee-organisations-titre">
              <h2 id="journee-organisations-titre" :class="sectionTitle">{{ t('track.organizations.title') }}</h2>
              <p class="mt-2.5 max-w-[66ch] text-[0.95rem] text-text-secondary">{{ t(`track.organizations.lead.${subject}`) }}</p>
              <ul class="mt-5 grid grid-cols-2 gap-3 sm:grid-cols-4">
                <li
                  v-for="organization in organizations"
                  :key="organization.id"
                  class="flex h-full min-h-26 flex-col items-center justify-center gap-1.5 rounded-lg border border-border-subtle bg-surface-raised p-3.5 text-center"
                >
                  <OrganizationLogoMark v-if="organization.logo" :name="organization.name" :acronym="organization.acronym" :logo="organization.logo" />
                  <b v-else class="font-condensed text-2xl leading-none font-bold tracking-[0.03em] text-heading" aria-hidden="true">
                    {{ organization.acronym ?? initialsOf(organization.name) }}
                  </b>
                  <small class="line-clamp-3 text-[0.76rem] leading-snug text-text-secondary">{{ organization.name }}</small>
                </li>
              </ul>
            </section>

            <section v-if="page.target_audiences.length" id="public" aria-labelledby="journee-public-titre">
              <h2 id="journee-public-titre" :class="sectionTitle">{{ t('track.audiences') }}</h2>
              <ul class="mt-4 flex flex-wrap gap-2">
                <li
                  v-for="(audience, index) in page.target_audiences"
                  :key="index"
                  class="inline-flex min-h-8 items-center rounded-full border border-(--color-day-line) bg-(--color-day-surface) px-3 text-sm"
                >
                  {{ tr(audience) }}
                </li>
              </ul>
            </section>

            <TrackFeature v-if="!page.feature" :feature="null" :documents="page.documents" />

            <TrackOthers
              :tracks="page.other_tracks"
              :edition-slug="page.edition.slug"
              :edition-label="editionLabel"
              :timezone="timezone"
              :now="now"
            />
          </div>
        </div>
      </div>
    </template>
  </div>
</template>
