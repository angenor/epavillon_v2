<script setup lang="ts">
import type { CallForProposals } from '~/types/event/call'
import type { EventEdition } from '~/types/event/edition'

/**
 * LE BILLET DE L'APPEL À PROPOSITIONS, en tête de la colonne de droite : la
 * même carte que la journée et l'activité (`UiTicket`), bande bleu nuit au nom
 * de l'appel. C'est la raison de la visite : il remonte sur le bandeau et reste
 * collé sur grand écran.
 *
 * Trois phases, trois contenus, l'état venant du modèle (`utils/call.ts` rejoue
 * `event.is_call_open()` et `event.effective_deadline()`, prolongation comprise) :
 *   · à venir : la date d'ouverture, rien à faire aujourd'hui ;
 *   · ouvert  : le rebours, l'échéance, le dépôt (seul bouton principal) ;
 *   · clos    : la date de clôture, l'annonce des résultats, la programmation.
 */

interface Props {
  /** `null` quand l'édition n'ouvre aucun appel : le billet n'est pas rendu. */
  call: CallForProposals | null
  edition: EventEdition
  /** Destination du dépôt (écran A4). */
  submitTo: string
  /** La programmation de l'édition, ou la liste des programmations tant qu'elle n'est pas publiée. */
  programmeTo: string
  programmePublished: boolean
  /** Ancre de la grille d'évaluation ; nulle quand l'appel n'en publie pas. */
  criteriaHref?: string | null
}

const props = withDefaults(defineProps<Props>(), { criteriaHref: null })

const { t } = useI18n()
const { tr } = useI18nText()
const { date, dateTime, timeWithZone } = useDateTime()

const phase = computed<CallPhase | null>(() => (props.call ? callPhase(props.call) : null))
const deadline = computed(() => (props.call ? effectiveDeadline(props.call) : null))
const countdown = useCountdown(deadline)

const zone = computed(() => props.edition.timezone)
const city = computed(() => props.edition.city ?? undefined)

/** L'instant qui fait foi : l'ouverture tant qu'elle n'a pas eu lieu, l'échéance ensuite. */
const keyInstant = computed(() => (phase.value === 'upcoming' ? (props.call?.opens_at ?? null) : deadline.value))
const keyDate = computed(() => (keyInstant.value ? date(keyInstant.value, zone.value) : ''))
const keyTime = computed(() =>
  keyInstant.value ? t('event.public.call.atTime', { time: timeWithZone(keyInstant.value, zone.value, city.value) }) : '',
)

const originalDeadlineLabel = computed(() =>
  props.call && wasExtended(props.call) ? dateTime(props.call.closes_at, zone.value) : '',
)
/** `results_expected_at` est une DATE : midi UTC pour qu'aucun fuseau ne la décale. */
const resultsLabel = computed(() =>
  props.call?.results_expected_at ? date(`${props.call.results_expected_at}T12:00:00Z`, zone.value) : '',
)

const isUrgent = computed(() => phase.value === 'open' && Boolean(countdown.value?.imminent))

const keyLabel = computed(() =>
  t(
    phase.value === 'upcoming'
      ? 'event.public.call.opensAt'
      : phase.value === 'open'
        ? 'event.public.call.deadline'
        : 'event.public.call.closedAt',
  ),
)

const capsClass = 'text-[0.72rem] font-bold tracking-[0.07em] text-text-muted uppercase'
</script>

<template>
  <UiTicket v-if="props.call && phase" :label="t('event.public.call.band')">
    <template #band>
      <div class="flex items-center gap-3.5 bg-surface-inverse px-5.5 py-4 text-text-on-inverse">
        <div class="min-w-0">
          <small class="block text-[0.72rem] font-bold tracking-[0.07em] text-text-on-inverse-muted uppercase">
            {{ t('event.public.call.band') }}
          </small>
          <b class="block font-condensed text-2xl leading-[1.05] font-bold tracking-[0.02em] uppercase">
            {{ tr(props.call.title) }}
          </b>
        </div>
        <UiIcon name="document" size="1.8rem" :stroke-width="1.6" class="ml-auto shrink-0 text-accent-on-inverse" />
      </div>
    </template>

    <div>
      <UiBadge
        :intent="phase === 'open' ? (isUrgent ? 'warning' : 'info') : 'neutral'"
        size="sm"
        :solid="phase === 'open'"
      >
        {{ t(`event.public.call.phase.${phase}`) }}
      </UiBadge>
    </div>

    <!-- Le rebours se remplit à l'hydratation : `useCountdown()` dit pourquoi. -->
    <div v-if="phase === 'open' && countdown && !countdown.expired" class="mt-4">
      <p :class="capsClass">{{ t('event.public.call.remaining') }}</p>
      <p
        class="mt-1 font-condensed text-[2.6rem] leading-none font-bold tabular-nums uppercase"
        :class="isUrgent ? 'text-warning' : 'text-accent'"
      >
        <template v-if="countdown.days > 0">
          {{ t('event.public.call.countdown.days', { count: countdown.days }, countdown.days) }}
        </template>
        <template v-else-if="countdown.hours > 0">
          {{ t('event.public.call.countdown.hours', { count: countdown.hours }, countdown.hours) }}
        </template>
        <template v-else>
          {{ t('event.public.call.countdown.minutes', { count: countdown.minutes }, countdown.minutes) }}
        </template>
      </p>
    </div>

    <!-- La date qui fait foi : le rebours dit l'urgence, elle dit le fait. -->
    <dl class="mt-4">
      <dt :class="capsClass">{{ keyLabel }}</dt>
      <dd class="mt-1">
        <span class="block font-condensed text-[2.1rem] leading-none font-bold text-heading uppercase">{{ keyDate }}</span>
        <span class="mt-1 block text-sm text-text-secondary">{{ keyTime }}</span>
        <!-- La prolongation ne remplace pas l'échéance annoncée : elle s'ajoute. -->
        <span v-if="originalDeadlineLabel" class="mt-1 block text-xs text-text-muted">
          {{ t('event.public.call.extendedFrom', { date: originalDeadlineLabel }) }}
        </span>
      </dd>

      <template v-if="phase !== 'open' && resultsLabel">
        <dt class="mt-4" :class="capsClass">{{ t('event.public.call.results') }}</dt>
        <dd class="mt-1 text-sm font-semibold text-text">{{ resultsLabel }}</dd>
      </template>
    </dl>

    <p v-if="phase === 'closed'" class="mt-4 rounded-lg bg-info-surface px-3 py-2.5 text-sm font-semibold text-info">
      {{ t('event.public.call.closedNote') }}
    </p>

    <div class="mt-5 flex flex-col gap-2.5">
      <UiButton
        v-if="phase === 'open'"
        :to="props.submitTo"
        size="lg"
        block
        icon-trailing="arrow-right"
        :label="t('event.public.call.submit')"
      />
      <UiButton
        v-else
        :to="props.programmeTo"
        size="lg"
        block
        :label="props.programmePublished ? t('event.public.programmeLink.see') : t('event.public.programmeLink.seePast')"
      />
      <UiButton
        v-if="props.criteriaHref"
        variant="secondary"
        :to="props.criteriaHref"
        block
        :label="t('event.public.call.seeCriteria')"
      />
    </div>
  </UiTicket>
</template>
