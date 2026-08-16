<script setup lang="ts">
import type { PublicScheduleRow, TemporalState } from '~/types/views'
import type { ThemeBadge, TimelineStep } from '~/types/ui'
import type { OrganizationId } from '~/types/shared'

/**
 * Section « Composants métier » — les deux composants les plus vus de la
 * plateforme, sur de vraies données.
 *
 * SIX CARTES, SIX ÉTATS. Les données simulées se tiennent toutes en novembre
 * 2027 : `temporal_state` y vaut donc « à venir » partout, ce qui est exact mais
 * ne démontre rien. Les six cartes ci-dessous reprennent SIX VRAIES SÉANCES et
 * n'en forcent que l'état temporel — le titre, l'organisation, la salle, les
 * thématiques, la jauge et le fuseau restent ceux de la base. C'est la seule
 * altération de cette page, et elle est ici pour que les six rendus existent
 * ensemble sous les yeux.
 *
 * LE REPÈRE « EN DIRECT » n'est pas un état temporel : il vient de la diffusion,
 * et une seule carte de toute la plateforme peut le porter. La démonstration
 * déclare donc une séance en direct par `useLiveSession()` — la même mécanique
 * qu'un écran réel — plutôt que de forcer l'affichage.
 */

interface Props {
  /** Séances réelles de `v_public_schedule`, chargées par la page. */
  sessions: PublicScheduleRow[]
  /** Thématiques indexées par code (`reference.taxonomy_terms`). */
  themesByCode: Record<string, ThemeBadge>
  /** Pays de chaque organisation — absent de la vue, voir l'écart consigné. */
  countryByOrganization: Record<OrganizationId, string>
  /** Nom du lieu de l'édition, pour le libellé de fuseau. */
  zoneLabel: string
  loading?: boolean
}

const props = defineProps<Props>()

const { t } = useI18n()
const { setLive } = useLiveSession()

/** Les six états à démontrer, dans l'ordre où ils se rencontrent. */
const DEMO_STATES: (TemporalState | 'live')[] = [
  'upcoming',
  'live',
  'ongoing',
  'past',
  'postponed',
  'cancelled',
]

/**
 * Six vraies séances, dont seul l'état temporel est forcé. La séance « en
 * direct » est déclarée au registre : c'est lui, et non la carte, qui autorise
 * le repère.
 */
const cards = computed(() =>
  DEMO_STATES.map((state, index) => {
    const source = props.sessions[index % Math.max(1, props.sessions.length)]
    if (!source) return null
    return {
      state,
      session: {
        ...source,
        temporal_state: (state === 'live' ? 'ongoing' : state) as TemporalState,
      } satisfies PublicScheduleRow,
    }
  }).filter((entry) => entry !== null),
)

/** Déclare la séance « en direct » — une seule, comme l'exige la règle n° 4. */
watchEffect(() => {
  const liveCard = cards.value.find((card) => card.state === 'live')
  setLive(liveCard?.session.id ?? null)
})

/**
 * La version dense écarte la séance « en direct » : c'est la MÊME séance, et la
 * voir porter le repère à deux endroits de la page brouillerait la règle qu'on
 * cherche justement à démontrer.
 */
const compactCards = computed(() => cards.value.filter((card) => card.state !== 'live').slice(0, 3))

const themesOf = (session: PublicScheduleRow): ThemeBadge[] =>
  session.theme_codes.map((code) => props.themesByCode[code]).filter((theme) => theme !== undefined)

const countryOf = (session: PublicScheduleRow): string | null =>
  session.organization_id ? (props.countryByOrganization[session.organization_id] ?? null) : null

// ---------------------------------------------------------------------------
// Frise d'avancement — les quatre parcours qu'un dossier peut suivre
// ---------------------------------------------------------------------------

/** Parcours nominal : déposé, évalué, retenu. */
const acceptedSteps = computed<TimelineStep[]>(() => [
  {
    value: 'draft',
    label: t('style-guide.business.status.draft'),
    at: '2027-06-12T09:24:00-03:00',
    actor: 'Awa Diop',
  },
  {
    value: 'submitted',
    label: t('style-guide.business.status.submitted'),
    at: '2027-06-28T17:02:00-03:00',
    actor: 'Awa Diop',
  },
  {
    value: 'under_review',
    label: t('style-guide.business.status.under_review'),
    at: '2027-07-04T10:15:00-03:00',
    detail: t('style-guide.business.timeline.reviewDetail', { count: 4 }),
  },
  {
    value: 'accepted',
    label: t('style-guide.business.status.accepted'),
    at: '2027-08-01T14:40:00-03:00',
    actor: 'Comité de sélection',
    detail: t('style-guide.business.timeline.acceptedDetail'),
  },
])

/** Parcours réel le plus fréquent : une correction demandée en cours de route. */
const changesRequestedSteps = computed<TimelineStep[]>(() => [
  {
    value: 'draft',
    label: t('style-guide.business.status.draft'),
    at: '2027-06-02T11:10:00-03:00',
    state: 'done',
  },
  {
    value: 'submitted',
    label: t('style-guide.business.status.submitted'),
    at: '2027-06-20T08:45:00-03:00',
    state: 'done',
  },
  {
    value: 'changes_requested',
    label: t('style-guide.business.status.changes_requested'),
    at: '2027-07-09T16:30:00-03:00',
    actor: 'Marc Boucher',
    detail: t('style-guide.business.timeline.changesDetail'),
    state: 'error',
  },
  { value: 'decision', label: t('style-guide.business.timeline.decision'), at: null },
])

/** Parcours interrompu : le dossier ne va pas jusqu'à la décision. */
const withdrawnSteps = computed<TimelineStep[]>(() => [
  { value: 'draft', label: t('style-guide.business.status.draft'), at: '2027-05-28T09:00:00-03:00', state: 'done' },
  { value: 'submitted', label: t('style-guide.business.status.submitted'), at: '2027-06-15T12:20:00-03:00', state: 'done' },
  {
    value: 'withdrawn',
    label: t('style-guide.business.status.withdrawn'),
    at: '2027-06-30T09:05:00-03:00',
    actor: 'Awa Diop',
    detail: t('style-guide.business.timeline.withdrawnDetail'),
    state: 'done',
  },
  { value: 'decision', label: t('style-guide.business.timeline.decision'), at: null, state: 'skipped' },
])
</script>

<template>
  <StyleGuideSection
    id="composants-metier"
    :title="t('style-guide.business.title')"
    :description="t('style-guide.business.description')"
  >
    <!-- SIX CARTES DE SÉANCE -->
    <StyleGuideDemo
      :title="t('style-guide.business.cards.title')"
      :note="t('style-guide.business.cards.note')"
      surface
    >
      <div v-if="props.loading" class="grid gap-4 lg:grid-cols-2">
        <UiSkeletonLoader v-for="index in 6" :key="index" height="12rem" />
      </div>

      <div v-else-if="cards.length === 0">
        <UiEmptyState compact :title="t('style-guide.business.cards.noData')" />
      </div>

      <div v-else class="grid gap-4 lg:grid-cols-2">
        <div v-for="card in cards" :key="card.state">
          <p class="mb-1.5 font-mono text-xs text-text-subtle">
            temporal_state = "{{ card.session.temporal_state }}"
            <span v-if="card.state === 'live'"> · status = "live"</span>
          </p>
          <UiSessionCard
            :session="card.session"
            :themes="themesOf(card.session)"
            :organization-country="countryOf(card.session)"
            :zone-label="props.zoneLabel"
            :cancelled-reason="
              card.state === 'cancelled' ? t('style-guide.business.cards.cancelledReason') : null
            "
            :waitlist-enabled="true"
            :waitlist-count="card.state === 'upcoming' ? 6 : undefined"
          />
        </div>
      </div>
    </StyleGuideDemo>

    <!-- VERSION DENSE -->
    <StyleGuideDemo
      :title="t('style-guide.business.compact.title')"
      :note="t('style-guide.business.compact.note')"
      surface
    >
      <div v-if="!props.loading && compactCards.length" class="space-y-3">
        <UiSessionCard
          v-for="card in compactCards"
          :key="`compact-${card.state}`"
          :session="card.session"
          :themes="themesOf(card.session)"
          :organization-country="countryOf(card.session)"
          :zone-label="props.zoneLabel"
          compact
        />
      </div>
      <UiSkeletonLoader v-else :lines="3" height="4rem" />
    </StyleGuideDemo>

    <!-- FRISE D'AVANCEMENT -->
    <StyleGuideDemo
      :title="t('style-guide.business.timeline.title')"
      :note="t('style-guide.business.timeline.note')"
    >
      <div class="grid gap-8 lg:grid-cols-3">
        <div>
          <p class="mb-3 text-sm font-semibold text-text">
            {{ t('style-guide.business.timeline.pathAccepted') }}
          </p>
          <UiStatusTimeline :steps="acceptedSteps" timezone="America/Belem" zone-label="Belém" />
        </div>
        <div>
          <p class="mb-3 text-sm font-semibold text-text">
            {{ t('style-guide.business.timeline.pathChanges') }}
          </p>
          <UiStatusTimeline
            :steps="changesRequestedSteps"
            timezone="America/Belem"
            zone-label="Belém"
            current="changes_requested"
          />
        </div>
        <div>
          <p class="mb-3 text-sm font-semibold text-text">
            {{ t('style-guide.business.timeline.pathWithdrawn') }}
          </p>
          <UiStatusTimeline :steps="withdrawnSteps" timezone="America/Belem" zone-label="Belém" />
        </div>
      </div>
    </StyleGuideDemo>
  </StyleGuideSection>
</template>
