<script setup lang="ts">
import type { PublicSessionStream } from '~/types/live'

/**
 * LA PAGE D'UNE ACTIVITÉ, celle qu'ouvre un clic dans la semaine ou la liste
 * du jour. Bandeau de l'édition, titre en grand, image et billet en léger débord
 * (arbitré le 05/10), dans la direction condensée (01/11) ; pendant le direct,
 * l'image cède la place au lecteur.
 *
 * L'adresse porte l'édition et l'activité par leurs slugs : une séance n'a de
 * slug unique que dans son édition. Une activité inconnue et une activité non
 * publiée donnent la même réponse, comme l'API.
 *
 * Le direct, la réunion Zoom, les questions et les autres activités du jour se
 * chargent à part : leur absence ne coûte jamais la page.
 *
 * Une activité d'une journée spéciale ou de l'atelier se suit par le direct de
 * la journée (01/11) : on s'inscrit à la journée, et « Rejoindre » mène à sa
 * salle. La page de la journée se lit pour sa fenêtre d'inscription.
 */

definePageMeta({ layout: 'public' })
defineI18nRoute({ paths: { fr: '/programmations/[edition]/[session]', en: '/programmes/[edition]/[session]' } })

const route = useRoute()
const api = useApi()
const { t, locale } = useI18n()
const { tr } = useI18nText()
const { dayLong, time } = useDateTime()
const localePath = useLocalePath()
const { state: stateOf } = useProgrammeSession()
const visitedId = useState<string | null>('programme-visited', () => null)

const editionSlug = computed(() => String(route.params.edition))
const sessionSlug = computed(() => String(route.params.session))

const { data, status, error, refresh } = await useActivityDetail()

if (import.meta.server && status.value === 'success' && !data.value) setResponseStatus(useRequestEvent()!, 404)

const edition = computed(() => data.value?.edition ?? null)
const detail = computed(() => data.value?.detail ?? null)
const session = computed(() => detail.value?.session ?? null)
const { declareFrom } = useLiveSession()
watch(session, (current) => current && declareFrom([current]), { immediate: true })
const state = computed(() => (session.value ? stateOf(session.value) : 'upcoming'))
const timezone = computed(() => session.value?.timezone ?? 'UTC')
const { incidents } = useSessionIncidents(session)

// Séance commune : chaque dossier garde sa carte ; ses intervenants et organisations y vivent, pas dans les listes de la séance.
const { jointParts } = useJointSession()
const jointProposals = computed(() => detail.value?.proposals ?? [])
const parts = computed(() => jointParts(jointProposals.value.length ? jointProposals.value : session.value?.proposals))
/** La séance commune n'a en général pas d'image propre : le direct et la rediffusion se posent sur celle du premier dossier. */
const stageCover = computed(() => session.value?.cover ?? parts.value.find((part) => part.cover)?.cover ?? null)
/** Une activité ou un dossier sans couverture prend la photo de l'édition, voilée au sigle. */
const editionImage = computed(() => editionPhoto(edition.value))

const { data: schedule, refresh: refreshSchedule } = useLazyAsyncData(
  () => `activite-programme-${edition.value?.id ?? 'aucune'}`,
  async () => (edition.value ? api.sessions.schedule(edition.value.id).catch(() => []) : []),
  { default: () => [], watch: [edition] },
)

/**
 * Direct allumé ou éteint, activité déplacée, annulée, retitrée : la fiche et la
 * journée se relisent au signal du programme public. Sans squelette ni message :
 * une relecture manquée garde ce qui est affiché.
 */
async function reloadQuietly(): Promise<void> {
  const current = edition.value
  if (!current) return
  try {
    const next = await api.pavillon.activite(current.id, sessionSlug.value)
    data.value = next ? { edition: current, detail: next } : null
  } catch {
    // Le prochain signal rattrape.
  }
  void refreshSchedule()
}
useRealtime(() => publicEditionTopics([edition.value?.id], PROGRAMME_CHANNELS), () => reloadQuietly())

const sameDay = computed(() => {
  const current = session.value
  if (!current) return []
  const day = dayKeyInZone(current.starts_at, current.timezone)
  return (schedule.value ?? [])
    .filter((entry) => dayKeyInZone(entry.starts_at, current.timezone) === day)
    .sort((a, b) => a.starts_at.localeCompare(b.starts_at))
})

const streams = ref<PublicSessionStream[]>([])
const now = ref<number | null>(null)
let clock: ReturnType<typeof setInterval> | undefined

async function loadStreams(): Promise<void> {
  const current = session.value
  if (!current?.is_streamed || (state.value !== 'live' && state.value !== 'ongoing')) return
  streams.value = await api.live.sessionStreams(current.id).catch(() => [])
}

// Une règle pour la scène et le billet, à toute heure : un flux YouTube, ou la rediffusion d'une séance passée, l'emporte sur la salle Zoom.
const zoomOpen = computed(() => {
  const current = session.value
  if (!current?.is_streamed || state.value === 'cancelled' || streams.value.length > 0) return false
  if (state.value === 'past' && current.replay_url) return false
  return isMeetingJoinable(meeting.value)
})
const { meeting } = useSessionMeeting(session)
const zoomShown = computed(() => meeting.value !== null && zoomOpen.value)

const dayBadge = computed(() => (session.value ? dayTrackOf(session.value.tracks) : null))
const { data: dayPage } = useLazyAsyncData(
  () => `activite-journee-${editionSlug.value}-${dayBadge.value?.slug ?? 'aucune'}`,
  async () => (dayBadge.value ? api.sessions.trackPage(editionSlug.value, dayBadge.value.slug).catch(() => null) : null),
  { default: () => null, watch: [dayBadge] },
)
const day = computed(() => (dayPage.value ? { page: dayPage.value } : null))

// On n'entre jamais dans la page : « Rejoindre » ouvre la salle pleine page, qui entre sans second clic ; celle de la journée pour une activité de journée.
const roomTo = computed(() =>
  dayBadge.value
    ? localePath({ name: 'track-edition-track-room', params: { edition: editionSlug.value, track: dayBadge.value.slug }, query: { entrer: '1' } })
    : localePath({
        name: 'activity-edition-session-room',
        params: { edition: editionSlug.value, session: sessionSlug.value },
        query: { entrer: '1' },
      }),
)

onMounted(() => {
  now.value = Date.now()
  clock = setInterval(() => (now.value = Date.now()), 30_000)
  if (session.value) visitedId.value = session.value.id
  void loadStreams()
})
onBeforeUnmount(() => clearInterval(clock))
// `refresh()` au début de la séance rend un nouvel objet pour la même activité : on garde les flux.
watch(session, (current, previous) => {
  const changed = current?.id !== previous?.id
  if (changed) streams.value = []
  if (current) visitedId.value = current.id
  void loadStreams()
})

const homeTime = computed(() => {
  if (now.value === null || !session.value) return null
  const visitor = Intl.DateTimeFormat().resolvedOptions().timeZone
  if (!visitor || visitor === session.value.timezone) return null
  const range = t('activity.ticket.hours', { start: time(session.value.starts_at, visitor), end: time(session.value.ends_at, visitor) })
  const sameInstant = range === t('activity.ticket.hours', { start: time(session.value.starts_at, session.value.timezone), end: time(session.value.ends_at, session.value.timezone) })
  return sameInstant ? null : { range, zone: t('activity.hero.yourZone', { zone: timeZoneCityLabel(visitor) }) }
})

const languages = computed(() =>
  (session.value?.language_codes ?? []).map((code) => {
    try {
      return new Intl.DisplayNames([locale.value], { type: 'language' }).of(code) ?? code
    } catch {
      return code
    }
  }),
)

const description = computed(() => (detail.value?.description ? tr(detail.value.description) : ''))
const hasDescription = computed(() => richTextToPlain(description.value).length > 0)
const summary = computed(() => (session.value?.summary ? tr(session.value.summary) : ''))
const dayLabel = computed(() => (session.value ? dayLong(session.value.starts_at, timezone.value) : ''))
const programmeTo = computed(() =>
  localePath({ name: 'programme', query: edition.value ? { edition: edition.value.slug, jour: session.value ? dayKeyInZone(session.value.starts_at, timezone.value) : undefined } : {} }),
)
const trail = computed(() =>
  edition.value ? t('activity.trail', { edition: edition.value.acronym ?? edition.value.edition_label ?? tr(edition.value.title), day: dayLabel.value }) : '',
)
const programmeListTo = computed(() =>
  localePath({ name: 'programme', query: edition.value ? { edition: edition.value.slug, vue: 'liste', jour: session.value ? dayKeyInZone(session.value.starts_at, timezone.value) : undefined } : {} }),
)
// La première journée spéciale donne sa couleur à la page (bande de date, accents) ; sans elle, le bleu nuit.
const specialDay = computed(() => {
  const track = session.value?.tracks.find((entry) => entry.kind === 'special_day')
  if (!track || !edition.value) return null
  return {
    title: tr(track.title),
    color: track.color,
    to: localePath({ name: 'track-edition-track', params: { edition: edition.value.slug, track: track.slug } }),
  }
})
const dayStyle = computed(() => trackDayStyle(specialDay.value?.color ?? null))

const showQuestions = computed(
  () => Boolean(detail.value?.allows_questions) && state.value !== 'cancelled' && state.value !== 'past',
)

useHead(() => ({
  title: session.value ? tr(session.value.title) : t('activity.notFound.title'),
  meta: summary.value ? [{ name: 'description', content: summary.value }] : [],
}))

// L'aperçu d'un lien partagé (29/10) : l'image de couverture, celle que l'atelier
// de l'organisation fabrique, à défaut celle que l'IFDD a posée sur la séance.
const requestUrl = useRequestURL()
useSeoMeta({
  ogType: 'website',
  ogUrl: () => requestUrl.href,
  ogTitle: () => (session.value ? tr(session.value.title) : undefined),
  ogDescription: () => summary.value || undefined,
  // La déclinaison `partage` (JPEG sous 100 Ko) : WhatsApp ignore une image d'aperçu trop lourde.
  ogImage: () => {
    const cover = stageCover.value
    if (!cover) return undefined
    return new URL(cover.sources.partage?.url ?? cover.sources.md?.url ?? cover.url, requestUrl.origin).href
  },
  ogImageAlt: () => (session.value && stageCover.value ? tr(session.value.title) : undefined),
  twitterCard: 'summary_large_image',
})
</script>

<template>
  <div data-direction="condensee" :style="dayStyle" class="full-bleed -mt-8 -mb-8 min-h-[75vh] bg-surface font-sans text-text sm:-mt-10 sm:-mb-10">
    <UiLoadingState v-if="status === 'pending' && !data" :label="t('activity.loading')">
      <ActivitySkeleton />
    </UiLoadingState>

    <div v-else-if="error" class="mx-auto max-w-[1280px] px-4 py-10 sm:px-6">
      <UiErrorState :title="t('activity.error.title')" :description="t('activity.error.description')" @retry="refresh()" />
    </div>

    <div v-else-if="!session || !edition || !detail" class="mx-auto max-w-[1280px] px-4 py-10 sm:px-6">
      <UiEmptyState
        icon="calendar"
        :title="t('activity.notFound.title')"
        :description="t('activity.notFound.description')"
        :action-label="t('activity.back')"
        :action-to="localePath('programme')"
      />
    </div>

    <template v-else>
      <div v-if="incidents.length" class="mx-auto max-w-[1280px] px-4 pt-6 pb-4 sm:px-6">
        <EventIncidentBanners :incidents="incidents" :timezone="timezone" :zone-label="edition.city ?? undefined" />
      </div>
      <ActivityHero
        :session="session"
        :edition="edition"
        :state="state"
        :back-to="programmeTo"
        :trail="trail"
        :cancelled-reason="detail.cancelled_reason ? tr(detail.cancelled_reason) : null"
        :parts="parts"
      />

      <!--
        Billet d'abord sur téléphone, collé à droite sur grand écran. Les marges
        négatives n'existent qu'à partir de `lg` : sur une colonne, une marge
        négative raccourcit la ligne de grille et la suite remonte sous le billet.
      -->
      <div class="mx-auto max-w-7xl px-4 pb-16 sm:px-6">
        <div class="grid grid-cols-1 gap-7 pt-6 lg:grid-cols-[minmax(0,1fr)_380px] lg:items-start lg:gap-10 lg:pt-0">
          <div class="relative z-1 min-w-0 lg:sticky lg:top-[calc(var(--nav-height)+1rem)] lg:col-start-2 lg:row-start-1 lg:-mt-18">
            <ActivityTicket
              class="lg:max-h-[calc(100dvh-var(--nav-height)-2rem)]"
              :session="session"
              :edition="edition"
              :state="state"
              :languages="languages"
              :now="now"
              :home-time="homeTime"
              :meeting="meeting"
              :zoom-open="zoomShown"
              :room-to="roomTo"
              :special-day="specialDay"
              :day="day"
              @started="refresh()"
            />
          </div>

          <div class="relative z-1 flex min-w-0 flex-col gap-13 lg:col-start-1 lg:row-start-1 lg:-mt-18">
            <ActivityStage
              :session="session"
              :state="state"
              :streams="streams"
              :cover="stageCover"
              :acronym="session.organization_acronym"
              :parts="parts"
              :meeting="zoomShown ? meeting : null"
              :room-to="roomTo"
              :zone-label="edition.city ?? undefined"
              :is-recorded="detail.is_recorded"
              :edition-image="editionImage"
              :day-track-id="dayPage?.track.id ?? null"
            />

            <section v-if="summary || hasDescription" aria-labelledby="a-propos-titre">
              <h2 id="a-propos-titre" class="heading-condensed text-[clamp(1.8rem,3.4vw,2.3rem)] text-heading">
                {{ t('activity.about') }}
              </h2>
              <p v-if="summary" class="mt-4.5 max-w-[68ch] text-lg leading-relaxed text-text">{{ summary }}</p>
              <UiRichContent class="mt-3.5 max-w-[68ch]! text-[1.05rem] leading-[1.75]" :html="description" />
            </section>

            <ActivityJointParts v-if="jointProposals.length > 1" :proposals="jointProposals" :faded="state === 'past' || state === 'cancelled'" :edition-image="editionImage" />

            <template v-else>
              <ActivityPeople :speakers="detail.speakers" />

              <ActivityOrganizations :organizations="detail.organizations" />
            </template>

            <ClientOnly>
              <ActivityQuestions v-if="showQuestions" :session-id="session.id" :event-id="session.event_id" />
            </ClientOnly>

            <ActivityGallery v-if="detail.gallery?.length" :photos="detail.gallery" />

            <ActivitySameDay
              :sessions="sameDay"
              :current-id="session.id"
              :edition-slug="edition.slug"
              :timezone="timezone"
              :day-label="dayLabel"
              :day-to="specialDay?.to ?? programmeListTo"
              :special-day="specialDay"
              :edition-image="editionImage"
            />
          </div>
        </div>
      </div>
    </template>
  </div>
</template>
