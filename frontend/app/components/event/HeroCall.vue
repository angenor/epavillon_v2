<script setup lang="ts">
import type { CallForProposals } from '~/types/event/call'
import type { EventEdition } from '~/types/event/edition'

/**
 * L'ACTION DE DÉPÔT, DANS LE BANDEAU — le panneau flottant de la page publique
 * d'une édition.
 *
 * ── POURQUOI ELLE A QUITTÉ L'ENCART D'APPEL (19/08) ─────────────────────────
 *
 * Le bouton « Déposer une proposition » vivait dans `EventCallDetails`, à mi-page.
 * C'est LA raison pour laquelle une organisation ouvre cet écran, et il fallait
 * défiler pour le trouver : sur un portable, le premier écran ne montrait que le
 * titre de la conférence. Le panneau remonte donc dans le bandeau, à côté du
 * titre, avec les deux seules informations qui décident du clic — le temps qui
 * reste, et la date qui fait foi.
 *
 * L'encart garde tout le reste : la description de l'appel, les conditions, les
 * consignes, la grille. Rien n'est perdu, et la page ne porte toujours qu'UN
 * bouton principal.
 *
 * ── TROIS PHASES, TROIS PANNEAUX ────────────────────────────────────────────
 *
 *   · À VENIR  la date d'ouverture, et rien à faire aujourd'hui. Annoncer une
 *              action impossible use la confiance.
 *   · OUVERT   le rebours, l'échéance, le dépôt, les critères.
 *   · CLOS     la date de clôture et l'annonce des résultats. Aucun bouton.
 *
 * L'état vient du MODÈLE : `utils/call.ts` rejoue `event.is_call_open()` et
 * `event.effective_deadline()`, prolongation comprise.
 *
 * ── SUR PHOTOGRAPHIE, L'URGENCE PASSE PAR LA PASTILLE, PAS PAR LE CHIFFRE ───
 *
 * Les jetons d'état sont calibrés sur les surfaces de page : `--color-warning`
 * est un jaune sombre en thème clair, illisible sur un bandeau voilé. Le rebours
 * garde donc la couleur du texte inversé, et c'est la pastille — un APLAT opaque,
 * lisible sur n'importe quelle image — qui porte le jaune des dernières
 * quarante-huit heures. Sur la surface de page, le rendu sobre reprend les jetons
 * habituels.
 */

interface Props {
  /** `null` quand l'édition n'ouvre aucun appel : le panneau n'est pas rendu. */
  call: CallForProposals | null
  edition: EventEdition
  /** Destination du dépôt (écran A4). */
  submitTo: string
  /** Ancre de la grille d'évaluation, sur cette même page. */
  criteriaHref: string
  /** `glass` sur le bandeau photographique, `surface` sur l'en-tête sobre. */
  tone?: 'glass' | 'surface'
}

const props = withDefaults(defineProps<Props>(), { tone: 'surface' })

const { t } = useI18n()
const { date, dateTime } = useDateTime()

const phase = computed<CallPhase | null>(() => (props.call ? callPhase(props.call) : null))
const deadline = computed(() => (props.call ? effectiveDeadline(props.call) : null))
const countdown = useCountdown(deadline)

const zone = computed(() => props.edition.timezone)
const zoneName = computed(() => props.edition.city ?? props.edition.timezone)

const deadlineLabel = computed(() => (deadline.value ? dateTime(deadline.value, zone.value) : ''))
const opensLabel = computed(() =>
  props.call ? dateTime(props.call.opens_at, zone.value) : '',
)
/** `results_expected_at` est une DATE : midi UTC pour qu'aucun fuseau ne la décale. */
const resultsLabel = computed(() =>
  props.call?.results_expected_at
    ? date(`${props.call.results_expected_at}T12:00:00Z`, zone.value)
    : '',
)

const isUrgent = computed(() => phase.value === 'open' && Boolean(countdown.value?.imminent))

const glass = computed(() => props.tone === 'glass')

const panelClass = computed(() => {
  if (glass.value) {
    return 'border-glass-border bg-glass shadow-glass backdrop-blur-glass'
  }
  if (phase.value === 'open') {
    return isUrgent.value
      ? 'border-(length:--border-medium) border-warning-border bg-warning-surface'
      : 'border-(length:--border-medium) border-accent bg-info-surface'
  }
  return 'border-border bg-surface-raised'
})

const labelClass = computed(() => (glass.value ? 'text-text-on-inverse-muted' : 'text-text-subtle'))
const valueClass = computed(() => (glass.value ? 'text-text-on-inverse' : 'text-text'))
const mutedClass = computed(() => (glass.value ? 'text-text-on-inverse-muted' : 'text-text-muted'))

/** Le chiffre du rebours : neutre sur photographie, coloré sur fond de page. */
const countdownClass = computed(() => {
  if (glass.value) return 'text-text-on-inverse'
  return isUrgent.value ? 'text-warning' : 'text-accent'
})
</script>

<template>
  <aside
    v-if="props.call && phase"
    class="rounded-xl border p-5 sm:p-6"
    :class="panelClass"
    :aria-label="t('event.public.call.phase.' + phase)"
  >
    <UiBadge
      :intent="phase === 'open' ? (isUrgent ? 'warning' : 'info') : 'neutral'"
      size="sm"
      :solid="phase !== 'closed'"
    >
      {{ t(`event.public.call.phase.${phase}`) }}
    </UiBadge>

    <!-- LE REBOURS. Absent du rendu serveur au premier affichage, il se remplit à
         l'hydratation — `useCountdown()` explique pourquoi. -->
    <template v-if="phase === 'open' && countdown && !countdown.expired">
      <p
        class="mt-4 text-xs uppercase"
        :class="labelClass"
        :style="{ letterSpacing: 'var(--tracking-caps)' }"
      >
        {{ t('event.public.call.remaining') }}
      </p>
      <p class="font-display text-4xl leading-none tabular-nums sm:text-5xl" :class="countdownClass">
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
    </template>

    <!-- La date qui fait foi. Le rebours dit l'urgence, elle dit le fait. -->
    <dl class="mt-4">
      <dt
        class="text-xs uppercase"
        :class="labelClass"
        :style="{ letterSpacing: 'var(--tracking-caps)' }"
      >
        {{
          t(
            phase === 'upcoming'
              ? 'event.public.call.opensAt'
              : phase === 'open'
                ? 'event.public.call.deadline'
                : 'event.public.call.closedAt',
          )
        }}
      </dt>
      <dd class="mt-1 text-sm" :class="valueClass">
        {{ phase === 'upcoming' ? opensLabel : deadlineLabel }}
        <span class="block text-xs" :class="mutedClass">
          {{ t('common.datetime.zoneOf', { zone: zoneName }) }}
        </span>
      </dd>

      <template v-if="phase !== 'open' && resultsLabel">
        <dt
          class="mt-3 text-xs uppercase"
          :class="labelClass"
          :style="{ letterSpacing: 'var(--tracking-caps)' }"
        >
          {{ t('event.public.call.results') }}
        </dt>
        <dd class="mt-1 text-sm" :class="valueClass">{{ resultsLabel }}</dd>
      </template>
    </dl>

    <!-- L'ACTION. Pleine largeur : c'est la seule de la page, et le panneau est
         étroit — un bouton flottant à gauche d'un vide ne se lit pas comme une
         invitation. -->
    <div class="mt-5 flex flex-col gap-2">
      <UiButton
        v-if="phase === 'open'"
        :to="props.submitTo"
        size="lg"
        block
        icon-trailing="arrow-right"
        :label="t('event.public.call.submit')"
      />
      <UiButton
        :variant="glass ? 'glass' : 'secondary'"
        :to="props.criteriaHref"
        block
        :label="t('event.public.call.seeCriteria')"
      />
    </div>
  </aside>
</template>
