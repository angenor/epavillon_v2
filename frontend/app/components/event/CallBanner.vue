<script setup lang="ts">
import type { CallForProposals } from '~/types/event/call'
import type { EventEdition } from '~/types/event/edition'

/**
 * ENCART D'APPEL À PROPOSITIONS — la deuxième question de la page : QUELLES SONT
 * LES ÉCHÉANCES, et la troisième : QUE PUIS-JE FAIRE ?
 *
 * C'est le bloc le plus important de l'écran tant que l'appel est ouvert. Une
 * organisation qui ne le voit pas ne dépose pas, et un dossier non déposé est
 * une activité qui n'aura pas lieu. Il est donc rendu HAUT, large, et il porte
 * une action unique et nommée.
 *
 * TROIS ÉTATS, et ils ne se ressemblent pas :
 *
 *   · À VENIR  — la date d'ouverture, et rien à faire aujourd'hui. Ton
 *                d'information ; annoncer une action impossible use la confiance.
 *   · OUVERT   — l'échéance, le temps restant, le bouton de dépôt, le lien vers
 *                les critères. Ton d'accent, puis d'avertissement dans les
 *                dernières 48 heures : le jaune signale ce qui demande
 *                attention, et une échéance qui approche en demande.
 *   · CLOS     — ce qui a été fait, et quand les résultats sont attendus. Gris :
 *                clos n'est ni un succès ni un échec.
 *
 * L'ÉTAT VIENT DU MODÈLE, pas d'une lecture de statut : `utils/call.ts`
 * reproduit `event.is_call_open()` et `event.effective_deadline()`, prolongation
 * comprise. L'échéance affichée est donc celle qui fait foi — et quand l'appel a
 * été prolongé, la date annoncée à l'origine reste lisible à côté, ce que la v1
 * ne savait pas faire.
 */

interface Props {
  /** `null` quand l'édition n'ouvre aucun appel — une COP sans pavillon, un
   *  cycle de webinaires. L'encart n'est alors pas rendu du tout. */
  call: CallForProposals | null
  edition: EventEdition
  /** Ancre de la section des critères, sur cette même page. */
  criteriaHref: string
  /** Destination du dépôt (écran A4). */
  submitTo: string
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date, dateTime } = useDateTime()

const phase = computed<CallPhase | null>(() => (props.call ? callPhase(props.call) : null))

const deadline = computed(() => (props.call ? effectiveDeadline(props.call) : null))
const countdown = useCountdown(deadline)

/** Fuseau d'affichage des échéances : celui de l'édition, comme tout le reste. */
const zone = props.edition.timezone
const zoneCity = computed(() => props.edition.city ?? undefined)

const deadlineLabel = computed(() => (deadline.value ? dateTime(deadline.value, zone) : ''))
const opensLabel = computed(() => (props.call ? dateTime(props.call.opens_at, zone) : ''))
const originalDeadlineLabel = computed(() =>
  props.call && wasExtended(props.call) ? dateTime(props.call.closes_at, zone) : '',
)
/**
 * `results_expected_at` est une DATE : on n'en affiche que le jour. Passer par
 * `dateTime()` produirait « 15 novembre 2026 à 09:00 » — une heure que personne
 * n'a saisie, née de la conversion vers le fuseau de l'édition. Midi UTC sert
 * seulement à ce que la conversion ne fasse pas changer de jour.
 */
const resultsLabel = computed(() =>
  props.call?.results_expected_at ? date(`${props.call.results_expected_at}T12:00:00Z`, zone) : '',
)

/**
 * L'urgence n'est PAS l'ouverture : un appel ouvert depuis trois mois n'appelle
 * pas la même couleur qu'un appel qui ferme demain. Le jaune est réservé aux
 * dernières 48 heures — c'est ce que le guide de style appelle « ce qui demande
 * attention ».
 */
const isUrgent = computed(() => phase.value === 'open' && Boolean(countdown.value?.imminent))

const tone = computed(() => {
  if (phase.value === 'open') {
    return isUrgent.value
      ? 'border-warning-border bg-warning-surface'
      : 'border-accent bg-info-surface'
  }
  if (phase.value === 'upcoming') return 'border-info-border bg-surface-raised'
  return 'border-border bg-surface-sunken'
})
</script>

<template>
  <section
    v-if="props.call && phase"
    id="appel-a-propositions"
    class="scroll-mt-24 rounded-lg border-(length:--border-medium) px-5 py-6 sm:px-7"
    :class="tone"
    :aria-labelledby="'appel-titre'"
  >
    <div class="flex flex-wrap items-start justify-between gap-4">
      <div class="min-w-0">
        <UiBadge
          :intent="phase === 'open' ? (isUrgent ? 'warning' : 'info') : 'neutral'"
          size="sm"
          :solid="phase === 'open'"
        >
          {{ t(`event.public.call.phase.${phase}`) }}
        </UiBadge>
        <h2 id="appel-titre" class="mt-2 font-display text-2xl leading-snug">
          {{ tr(props.call.title) }}
        </h2>
      </div>

      <!-- LE REBOURS. Rendu seulement quand l'appel est ouvert : « il reste 0 j »
           sur un appel clos serait un contresens. Il est absent du rendu serveur
           au premier affichage et se remplit à l'hydratation — voir
           `useCountdown()`, qui explique pourquoi. -->
      <div
        v-if="phase === 'open' && countdown && !countdown.expired"
        class="rounded-md border border-border bg-surface-raised px-4 py-3 text-center"
      >
        <p class="text-xs uppercase text-text-subtle" :style="{ letterSpacing: 'var(--tracking-caps)' }">
          {{ t('event.public.call.remaining') }}
        </p>
        <p class="mt-1 font-display text-2xl tabular-nums" :class="isUrgent ? 'text-warning' : 'text-accent'">
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
    </div>

    <p v-if="props.call.description" class="mt-4 text-text-secondary" :style="{ maxWidth: 'var(--measure)' }">
      {{ tr(props.call.description) }}
    </p>

    <!-- Les dates, toutes dans le fuseau de l'édition. -->
    <dl class="mt-5 grid gap-4 sm:grid-cols-3">
      <div v-if="phase === 'upcoming'">
        <dt class="text-xs uppercase text-text-subtle" :style="{ letterSpacing: 'var(--tracking-caps)' }">
          {{ t('event.public.call.opensAt') }}
        </dt>
        <dd class="mt-1 text-text">{{ opensLabel }}</dd>
      </div>
      <div v-else>
        <dt class="text-xs uppercase text-text-subtle" :style="{ letterSpacing: 'var(--tracking-caps)' }">
          {{ t(phase === 'open' ? 'event.public.call.deadline' : 'event.public.call.closedAt') }}
        </dt>
        <dd class="mt-1 text-text">
          {{ deadlineLabel }}
          <span class="block text-sm text-text-muted">
            {{ t('common.datetime.zoneOf', { zone: props.edition.city ?? props.edition.timezone }) }}
          </span>
          <!-- La prolongation ne remplace pas l'échéance annoncée : elle s'ajoute. -->
          <span v-if="originalDeadlineLabel" class="mt-1 block text-sm text-text-muted">
            {{ t('event.public.call.extendedFrom', { date: originalDeadlineLabel }) }}
          </span>
        </dd>
      </div>

      <div v-if="resultsLabel">
        <dt class="text-xs uppercase text-text-subtle" :style="{ letterSpacing: 'var(--tracking-caps)' }">
          {{ t('event.public.call.results') }}
        </dt>
        <dd class="mt-1 text-text">{{ resultsLabel }}</dd>
      </div>

      <div>
        <dt class="text-xs uppercase text-text-subtle" :style="{ letterSpacing: 'var(--tracking-caps)' }">
          {{ t('event.public.call.rules') }}
        </dt>
        <dd class="mt-1 text-sm text-text-secondary">
          {{
            t('event.public.call.rulesValue', {
              max: props.call.max_proposals_per_organization ?? t('common.labels.none'),
              speakers: `${props.call.min_speakers}–${props.call.max_speakers}`,
              duration: props.call.default_duration_minutes,
            })
          }}
        </dd>
      </div>
    </dl>

    <!-- L'ACTION. Une seule est principale : déposer. Le reste renseigne. -->
    <div class="mt-6 flex flex-wrap items-center gap-3">
      <UiButton
        v-if="phase === 'open'"
        :to="props.submitTo"
        :label="t('event.public.call.submit')"
        icon-trailing="arrow-right"
      />
      <UiButton
        variant="secondary"
        :to="props.criteriaHref"
        :label="t('event.public.call.seeCriteria')"
      />
      <a
        v-if="props.call.guidelines_url"
        :href="props.call.guidelines_url"
        target="_blank"
        rel="noopener noreferrer"
        class="inline-flex items-center gap-1.5 text-sm text-accent"
      >
        {{ t('event.public.call.guidelines') }}
        <UiIcon name="external-link" size="0.9rem" />
        <span class="sr-only">{{ t('common.a11y.externalLink') }}</span>
      </a>
    </div>

    <p v-if="phase === 'closed'" class="mt-4 text-sm text-text-muted">
      {{ t('event.public.call.closedNote') }}
    </p>
  </section>
</template>
