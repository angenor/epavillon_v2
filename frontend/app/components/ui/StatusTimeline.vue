<script setup lang="ts">
import type { StepState, TimelineStep } from '~/types/ui'
import type { TimeZoneName } from '~/types/shared'

/**
 * FRISE D'AVANCEMENT D'UN DOSSIER — brouillon → déposé → en évaluation →
 * décision, chaque étape portant sa date.
 *
 * C'EST L'ÉCRAN QUE REGARDE UNE ORGANISATION QUI ATTEND UNE RÉPONSE. Il doit
 * répondre à trois questions sans qu'on ait à écrire à l'IFDD : où en est mon
 * dossier, depuis quand, et qu'attend-on de moi.
 *
 * ELLE NE DÉDUIT RIEN. Les étapes lui sont données, dans l'ordre, avec leur
 * état et leur date. La machine à états vit dans la BASE
 * (`programme.proposal_transitions_allowed`) et le journal des franchissements
 * dans `programme.proposal_transitions` : réimplémenter le graphe ici, c'est se
 * garantir de diverger de la base au premier changement de règle. L'appelant
 * compose donc ses étapes à partir de ces deux tables.
 *
 * LE CHEMIN N'EST PAS TOUJOURS LINÉAIRE, et la frise doit le montrer :
 * · `error`   — correction demandée : le dossier est REVENU en arrière ;
 * · `skipped` — étape non concernée (un dossier retiré ne passe pas en décision).
 * Une frise qui n'afficherait que des étapes franchies mentirait sur la moitié
 * des dossiers.
 *
 * TOUTE DATE PORTE SON FUSEAU, ici comme ailleurs : `timezone` est obligatoire
 * dès qu'une étape a une date.
 *
 * VERTICALE PAR DÉFAUT : quatre étapes portant chacune une date, un auteur et un
 * motif ne tiennent pas côte à côte. L'orientation horizontale est réservée aux
 * frises courtes et sans détail.
 */

interface Props {
  steps: TimelineStep[]
  /** Nom de la frise, annoncé par les lecteurs d'écran. */
  label?: string
  /** Fuseau d'affichage des dates. Obligatoire dès qu'une étape en porte une. */
  timezone?: TimeZoneName
  zoneLabel?: string
  /** Étape courante — sa `value`. Sert quand les `state` ne sont pas tous fournis. */
  current?: string
  orientation?: 'vertical' | 'horizontal'
}

const props = withDefaults(defineProps<Props>(), { orientation: 'vertical' })

const { t } = useI18n()
const { dateTime } = useDateTime()

const currentIndex = computed(() => {
  if (props.current) return props.steps.findIndex((step) => step.value === props.current)
  // À défaut, la dernière étape datée fait foi — c'est ce que dit le journal.
  const lastDated = props.steps.map((step) => Boolean(step.at)).lastIndexOf(true)
  return lastDated
})

function stateOf(step: TimelineStep, index: number): StepState {
  if (step.state) return step.state
  if (index < currentIndex.value) return 'done'
  if (index === currentIndex.value) return 'current'
  return 'upcoming'
}

const MARKERS: Record<StepState, string> = {
  done: 'border-success bg-success-surface text-success',
  current: 'border-accent bg-accent-solid text-accent-contrast',
  upcoming: 'border-border bg-surface-raised text-text-subtle',
  error: 'border-danger bg-danger-surface text-danger',
  skipped: 'border-border border-dashed bg-surface-sunken text-text-subtle',
}

const MARKER_ICONS: Record<StepState, string | null> = {
  done: 'check',
  current: null,
  upcoming: null,
  error: 'warning',
  skipped: 'minus',
}

const STATE_LABELS: Record<StepState, string> = {
  done: 'status-timeline.state.done',
  current: 'status-timeline.state.current',
  upcoming: 'status-timeline.state.upcoming',
  error: 'status-timeline.state.error',
  skipped: 'status-timeline.state.skipped',
}

function dateOf(step: TimelineStep): string {
  if (!step.at || !props.timezone) return ''
  return dateTime(step.at, props.timezone)
}
</script>

<template>
  <div :aria-label="props.label ?? t('status-timeline.label')" role="group">
    <ol
      :class="
        props.orientation === 'horizontal'
          ? 'flex flex-col sm:flex-row sm:items-start'
          : 'flex flex-col'
      "
    >
      <li
        v-for="(step, index) in props.steps"
        :key="step.value"
        class="relative flex gap-3"
        :class="
          props.orientation === 'horizontal'
            ? 'pb-5 last:pb-0 sm:flex-1 sm:flex-col sm:pb-0'
            : 'pb-5 last:pb-0'
        "
      >
        <!-- Trait de liaison, arrêté avant la dernière étape. En pointillés
             lorsqu'il mène à une étape non franchie : le chemin reste à faire. -->
        <span
          v-if="index < props.steps.length - 1"
          aria-hidden="true"
          class="absolute"
          :class="[
            props.orientation === 'horizontal'
              ? 'top-9 bottom-0 left-[0.875rem] w-px sm:top-[0.875rem] sm:right-0 sm:bottom-auto sm:left-[calc(50%+1.5rem)] sm:h-px sm:w-auto'
              : 'top-9 bottom-0 left-[0.875rem] w-px',
            stateOf(props.steps[index + 1] as TimelineStep, index + 1) === 'upcoming'
              ? 'border-l border-dashed border-border sm:border-t sm:border-l-0'
              : 'bg-border',
          ]"
        />

        <span
          class="relative z-10 flex size-7 shrink-0 items-center justify-center rounded-full border-2 transition-colors"
          :class="MARKERS[stateOf(step, index)]"
        >
          <UiIcon
            v-if="MARKER_ICONS[stateOf(step, index)]"
            :name="MARKER_ICONS[stateOf(step, index)] as string"
            size="0.9rem"
            :stroke-width="2.4"
          />
          <span v-else class="size-2 rounded-full bg-current" aria-hidden="true" />
        </span>

        <div class="min-w-0 flex-1 pb-1">
          <p
            class="text-sm leading-snug"
            :class="[
              stateOf(step, index) === 'current' ? 'font-semibold text-text' : 'text-text',
              stateOf(step, index) === 'upcoming' ? 'text-text-subtle' : '',
              stateOf(step, index) === 'skipped' ? 'text-text-subtle line-through' : '',
              stateOf(step, index) === 'error' ? 'font-semibold text-danger' : '',
            ]"
          >
            {{ step.label }}
            <span class="sr-only"> — {{ t(STATE_LABELS[stateOf(step, index)]) }}</span>
          </p>

          <!-- La date, dans le fuseau de l'édition. Sans date, l'étape est en
               attente et le dit : un blanc laisserait croire à un oubli. -->
          <p class="mt-0.5 text-xs text-text-subtle">
            <time v-if="step.at" :datetime="step.at" class="tabular-nums">{{ dateOf(step) }}</time>
            <span v-else>{{ t('status-timeline.pending') }}</span>
            <span v-if="step.actor"> · {{ t('status-timeline.by', { actor: step.actor }) }}</span>
          </p>

          <p v-if="step.detail" class="mt-1.5 rounded-md border border-border-subtle bg-surface-sunken px-2.5 py-1.5 text-sm text-text-muted">
            {{ step.detail }}
          </p>
        </div>
      </li>
    </ol>
  </div>
</template>
