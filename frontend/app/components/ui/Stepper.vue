<script setup lang="ts">
import type { StepItem, StepState } from '~/types/ui'

/**
 * Frise d'étapes d'un parcours en plusieurs écrans — le formulaire de soumission
 * en compte sept.
 *
 * CE QU'UNE FRISE DOIT DIRE, et que la v1 ne disait pas : où j'en suis, combien
 * il en reste, et LESQUELLES SONT EN DÉFAUT. Une étape franchie puis invalidée
 * par une correction demandée doit se voir depuis n'importe quelle autre étape,
 * sinon le dossier repart avec le même défaut.
 *
 * NAVIGATION EN ARRIÈRE TOUJOURS POSSIBLE : une étape franchie est cliquable.
 * Une étape à venir ne l'est pas — non par principe, mais parce qu'elle dépend
 * de ce qui précède. C'est à l'appelant de le dire, par `disabled`.
 *
 * ÉTAT PAR LA FORME AUTANT QUE PAR LA COULEUR : coche pour l'étape franchie,
 * point plein pour l'étape courante, numéro pour celles à venir, triangle
 * d'alerte pour celles en défaut. Lisible en niveaux de gris.
 *
 * DEUX ORIENTATIONS : horizontale au-dessus d'un formulaire large, verticale en
 * colonne latérale — et automatiquement verticale sous 640 px, où sept étapes
 * côte à côte deviennent illisibles.
 */

interface Props {
  steps: StepItem[]
  /** Étape en cours — sa `value`. Prime sur les `state` individuels. */
  modelValue: string
  /** Nom de la frise, annoncé par les lecteurs d'écran. */
  label: string
  orientation?: 'horizontal' | 'vertical'
  /** Les étapes franchies sont-elles cliquables ? */
  navigable?: boolean
}

const props = withDefaults(defineProps<Props>(), { orientation: 'horizontal', navigable: true })
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

const { t } = useI18n()

const currentIndex = computed(() => props.steps.findIndex((step) => step.value === props.modelValue))

/**
 * L'état explicite de l'étape l'emporte — c'est lui qui porte `error` et
 * `skipped`. Sinon, la position par rapport à l'étape courante le donne.
 */
function stateOf(step: StepItem, index: number): StepState {
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

const LABELS: Record<StepState, string> = {
  done: 'data.stepper.done',
  current: 'data.stepper.current',
  upcoming: 'data.stepper.upcoming',
  error: 'data.stepper.error',
  skipped: 'data.stepper.skipped',
}

function isClickable(step: StepItem, index: number): boolean {
  if (!props.navigable || step.disabled) return false
  return index <= currentIndex.value || stateOf(step, index) === 'error'
}

function select(step: StepItem, index: number): void {
  if (!isClickable(step, index)) return
  emit('update:modelValue', step.value)
}
</script>

<template>
  <nav :aria-label="props.label">
    <!-- Repère textuel : « Étape 3 sur 7 ». C'est ce qui reste quand la frise
         est trop large pour l'écran, et ce qu'annonce un lecteur d'écran. -->
    <p class="mb-3 text-sm text-text-subtle sm:sr-only">
      {{ t('data.stepper.position', { current: currentIndex + 1, total: props.steps.length }) }}
    </p>

    <ol
      class="flex gap-0"
      :class="
        props.orientation === 'vertical'
          ? 'flex-col'
          : 'flex-col sm:flex-row sm:items-start'
      "
    >
      <li
        v-for="(step, index) in props.steps"
        :key="step.value"
        class="relative flex min-w-0 gap-3"
        :class="
          props.orientation === 'vertical'
            ? 'pb-6 last:pb-0'
            : 'pb-4 last:pb-0 sm:flex-1 sm:flex-col sm:pb-0 sm:text-center'
        "
      >
        <!-- Trait de liaison. Il part du repère et s'arrête au suivant : dessiné
             en pseudo-élément pour ne pas dépasser sur la dernière étape. -->
        <span
          v-if="index < props.steps.length - 1"
          aria-hidden="true"
          class="absolute bg-border"
          :class="
            props.orientation === 'vertical'
              ? 'top-9 bottom-1 left-[0.9375rem] w-px'
              : 'top-9 bottom-1 left-[0.9375rem] w-px sm:top-[0.9375rem] sm:right-0 sm:bottom-auto sm:left-[calc(50%+1.5rem)] sm:h-px sm:w-auto'
          "
        />

        <component
          :is="isClickable(step, index) ? 'button' : 'div'"
          :type="isClickable(step, index) ? 'button' : undefined"
          class="flex min-w-0 gap-3 text-left"
          :class="
            props.orientation === 'horizontal'
              ? 'sm:flex-col sm:items-center sm:gap-2 sm:text-center'
              : ''
          "
          :aria-current="stateOf(step, index) === 'current' ? 'step' : undefined"
          @click="select(step, index)"
        >
          <span
            class="relative z-10 flex size-8 shrink-0 items-center justify-center rounded-full border-2 font-mono text-sm font-semibold tabular-nums transition-colors"
            :class="MARKERS[stateOf(step, index)]"
          >
            <UiIcon v-if="stateOf(step, index) === 'done'" name="check" size="1rem" :stroke-width="2.4" />
            <UiIcon v-else-if="stateOf(step, index) === 'error'" name="warning" size="1rem" />
            <template v-else>{{ index + 1 }}</template>
          </span>

          <span class="min-w-0">
            <span
              class="block text-sm leading-snug"
              :class="[
                stateOf(step, index) === 'current' ? 'font-semibold text-text' : 'text-text-muted',
                stateOf(step, index) === 'error' ? 'font-semibold text-danger' : '',
                stateOf(step, index) === 'skipped' ? 'text-text-subtle line-through' : '',
              ]"
            >
              {{ step.label }}
            </span>
            <span v-if="step.description" class="mt-0.5 block text-xs text-text-subtle">
              {{ step.description }}
            </span>
            <span class="sr-only"> — {{ t(LABELS[stateOf(step, index)]) }}</span>
          </span>
        </component>
      </li>
    </ol>
  </nav>
</template>
