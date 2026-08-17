<script setup lang="ts">
import type { CallForProposals } from '~/types/event/call'
import type { EventEdition } from '~/types/event/edition'

/**
 * L'ENCART PERMANENT — l'échéance de l'appel et le temps restant, visibles
 * depuis n'importe quelle étape.
 *
 * IL NE QUITTE JAMAIS L'ÉCRAN. Un formulaire de sept étapes se remplit sur
 * plusieurs jours ; l'échéance apprise à l'étape 1 est oubliée à l'étape 5.
 * L'encart est donc collé en tête de la colonne latérale sur écran large, et
 * reste en tête de page sur mobile — jamais replié, jamais fermable.
 *
 * L'ÉCHÉANCE EST CELLE QUI FAIT FOI : `event.effective_deadline()`, prolongation
 * comprise. Quand l'appel a été prolongé, la date annoncée à l'origine reste
 * lisible en dessous — c'est ce que la v1 ne savait pas faire, elle écrasait la
 * première.
 *
 * LE JAUNE EST RÉSERVÉ AUX DERNIÈRES QUARANTE-HUIT HEURES. Un appel ouvert
 * depuis trois mois n'appelle pas la même couleur qu'un appel qui ferme demain,
 * et une alerte permanente cesse d'être lue. Même règle que l'encart de la page
 * publique, et pour la même raison.
 */

interface Props {
  call: CallForProposals
  edition: EventEdition
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date, dateTime } = useDateTime()

const deadline = computed(() => effectiveDeadline(props.call))
const countdown = useCountdown(deadline)

const zone = computed(() => props.edition.timezone)
const zoneLabel = computed(() => props.edition.city ?? props.edition.timezone)

const deadlineLabel = computed(() => dateTime(deadline.value, zone.value))
const originalDeadlineLabel = computed(() =>
  wasExtended(props.call) ? dateTime(props.call.closes_at, zone.value) : '',
)
/** `results_expected_at` est une DATE : on n'en affiche jamais d'heure. */
const resultsLabel = computed(() =>
  props.call.results_expected_at ? date(`${props.call.results_expected_at}T12:00:00Z`, zone.value) : '',
)

const isUrgent = computed(() => Boolean(countdown.value?.imminent))
const isExpired = computed(() => Boolean(countdown.value?.expired))
</script>

<template>
  <aside
    class="rounded-lg border-(length:--border-medium) px-4 py-4"
    :class="
      isExpired
        ? 'border-border bg-surface-sunken'
        : isUrgent
          ? 'border-warning-border bg-warning-surface'
          : 'border-accent bg-info-surface'
    "
    :aria-label="t('proposal.form.deadline.label')"
  >
    <p class="text-xs uppercase text-text-subtle" :style="{ letterSpacing: 'var(--tracking-caps)' }">
      {{ t('proposal.form.deadline.title') }}
    </p>

    <p class="mt-1 font-bold text-text">{{ deadlineLabel }}</p>
    <p class="text-sm text-text-muted">
      {{ t('common.datetime.zoneOf', { zone: zoneLabel }) }}
    </p>

    <p v-if="originalDeadlineLabel" class="mt-1 text-sm text-text-muted">
      {{ t('proposal.form.deadline.extendedFrom', { date: originalDeadlineLabel }) }}
    </p>

    <!-- Le rebours n'est pas rendu au serveur : il se remplit à l'hydratation,
         voir `useCountdown()`. -->
    <p
      v-if="countdown && !countdown.expired"
      class="mt-3 font-display text-2xl tabular-nums"
      :class="isUrgent ? 'text-warning' : 'text-accent'"
    >
      <template v-if="countdown.days > 0">
        {{ t('proposal.form.deadline.countdown.days', { count: countdown.days }, countdown.days) }}
      </template>
      <template v-else-if="countdown.hours > 0">
        {{ t('proposal.form.deadline.countdown.hours', { count: countdown.hours }, countdown.hours) }}
      </template>
      <template v-else>
        {{ t('proposal.form.deadline.countdown.minutes', { count: countdown.minutes }, countdown.minutes) }}
      </template>
    </p>

    <p v-else-if="countdown?.expired" class="mt-3 font-bold text-text-secondary">
      {{ t('proposal.form.deadline.expired') }}
    </p>

    <dl class="mt-4 grid gap-3 border-t border-border pt-4 text-sm">
      <div v-if="resultsLabel">
        <dt class="text-text-subtle">{{ t('proposal.form.deadline.results') }}</dt>
        <dd class="text-text-secondary">{{ resultsLabel }}</dd>
      </div>
      <div>
        <dt class="text-text-subtle">{{ t('proposal.form.deadline.reviews') }}</dt>
        <dd class="text-text-secondary">
          {{ t('proposal.form.deadline.reviewsValue', { count: props.call.required_reviews }, props.call.required_reviews) }}
        </dd>
      </div>
      <div v-if="props.call.guidelines_url">
        <dt class="text-text-subtle">{{ t('proposal.form.deadline.guidelines') }}</dt>
        <dd>
          <a
            :href="props.call.guidelines_url"
            target="_blank"
            rel="noopener noreferrer"
            class="inline-flex items-center gap-1.5 text-accent"
          >
            {{ tr(props.call.title) }}
            <UiIcon name="external-link" size="0.85rem" />
            <span class="sr-only">{{ t('common.a11y.externalLink') }}</span>
          </a>
        </dd>
      </div>
    </dl>
  </aside>
</template>
