<script setup lang="ts">
import type { StepItem, StepState } from '~/types/ui'

/**
 * Barre d'étapes d'un parcours en plusieurs écrans — le formulaire de soumission
 * en compte sept.
 *
 * CE QU'UNE BARRE DOIT DIRE, et que la v1 ne disait pas : où j'en suis, combien
 * il en reste, et LESQUELLES SONT EN DÉFAUT. Une étape franchie puis invalidée
 * par une correction demandée doit se voir depuis n'importe quelle autre étape,
 * sinon le dossier repart avec le même défaut.
 *
 * UNE BARRE BORDÉE À CELLULES SÉPARÉES, pas une suite de pastilles reliées par
 * un trait. Le guide tranche ainsi pour deux raisons : la cellule offre une
 * cible de clic franche là où la pastille en offre une de 32 px, et le fond
 * plein de la cellule courante répond à « où j'en suis ? » sans que l'œil ait à
 * comparer cinq pastilles entre elles. Le trait de liaison, lui, promettait une
 * continuité que le parcours n'a pas : on peut revenir en arrière à tout moment.
 *
 * DÉBORDEMENT DANS LA BARRE, jamais dans la page : sept cellules de 152 px ne
 * tiennent pas sur 375 px, et le corps de page ne défile jamais horizontalement.
 * Le repère textuel « Étape 3 sur 7 » double l'information sur écran étroit, où
 * la cellule courante peut se trouver hors du champ visible.
 *
 * NAVIGATION EN ARRIÈRE TOUJOURS POSSIBLE : une étape franchie est cliquable.
 * Une étape à venir ne l'est pas — non par principe, mais parce qu'elle dépend
 * de ce qui précède. C'est à l'appelant de le dire, par `disabled`.
 *
 * ÉTAT PAR LA FORME AUTANT QUE PAR LA COULEUR : coche pour l'étape franchie,
 * numéro sur aplat pour l'étape courante, numéro sur fond neutre pour celles à
 * venir, triangle d'alerte pour celles en défaut, contour tireté et libellé
 * barré pour celles écartées. Lisible en niveaux de gris.
 *
 * DEUX ORIENTATIONS : horizontale au-dessus d'un formulaire large, verticale en
 * colonne latérale. La verticale empile les mêmes cellules et sépare par le bas
 * — c'est la même barre tournée d'un quart de tour, pas un second composant.
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

/** Pastille de tête de cellule. Aplat plein pour ce qui est acquis ou en cours. */
const MARKERS: Record<StepState, string> = {
  done: 'bg-success-solid text-success-contrast',
  current: 'bg-accent-solid text-accent-contrast',
  upcoming: 'bg-neutral-surface text-text-muted',
  error: 'bg-danger-solid text-danger-contrast',
  // Le refus partage le rouge du défaut mais ne dit pas la même chose : l'un
  // attend une correction, l'autre clôt le parcours. C'est le libellé barré et
  // la croix qui les séparent — pas une sixième couleur.
  refused: 'bg-danger-solid text-danger-contrast',
  skipped: 'border border-dashed border-border bg-surface-sunken text-text-subtle',
}

/**
 * Fond de cellule. SEULE la cellule courante en porte un : c'est la réponse à
 * « où j'en suis ? », et deux cellules teintées la rendraient ambiguë. Le défaut
 * se signale par la pastille et le libellé rouges, pas par un second aplat.
 */
const CELLS: Record<StepState, string> = {
  done: '',
  current: 'bg-accent-surface',
  upcoming: '',
  error: '',
  refused: '',
  skipped: '',
}

const TITLES: Record<StepState, string> = {
  done: 'text-text',
  current: 'text-text',
  upcoming: 'text-text-muted',
  error: 'text-danger',
  refused: 'text-danger line-through',
  skipped: 'text-text-subtle line-through',
}

/**
 * DEUX JEUX DE LIBELLÉS, et ils ne sont pas interchangeables.
 *
 * `data.stepper.*` est écrit pour être ANNONCÉ à la suite du titre
 * (« Organisation — étape franchie ») : minuscule, tournure de complément.
 * `data.stepper.short.*` est écrit pour être VU dans la cellule (« Terminée ») :
 * court, capitale d'attaque, autonome. Poser une capitale par le style sur le
 * premier jeu donnait « Étape franchie » en tête de cellule — grammaticalement
 * bancal et deux fois plus long que la place disponible.
 */
const LABELS: Record<StepState, string> = {
  done: 'data.stepper.done',
  current: 'data.stepper.current',
  upcoming: 'data.stepper.upcoming',
  error: 'data.stepper.error',
  skipped: 'data.stepper.skipped',
  refused: 'data.stepper.refused',
}

const SHORT_LABELS: Record<StepState, string> = {
  done: 'data.stepper.short.done',
  current: 'data.stepper.short.current',
  upcoming: 'data.stepper.short.upcoming',
  error: 'data.stepper.short.error',
  skipped: 'data.stepper.short.skipped',
  refused: 'data.stepper.short.refused',
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
    <!-- Repère textuel : « Étape 3 sur 7 ». C'est ce qui reste quand la barre
         déborde de l'écran, et ce qu'annonce un lecteur d'écran. -->
    <p class="mb-3 text-sm text-text-subtle sm:sr-only">
      {{ t('data.stepper.position', { current: currentIndex + 1, total: props.steps.length }) }}
    </p>

    <ol
      class="flex rounded-lg border border-border bg-surface-raised"
      :class="props.orientation === 'vertical' ? 'flex-col' : 'overflow-x-auto'"
    >
      <li
        v-for="(step, index) in props.steps"
        :key="step.value"
        class="flex min-w-38 shrink-0 grow basis-auto border-separator"
        :class="
          props.orientation === 'vertical'
            ? 'border-b last:border-b-0'
            : 'border-e last:border-e-0'
        "
      >
        <!-- La cellule ENTIÈRE est la cible de clic, jamais la seule pastille :
             40 px de haut sur 152 px de large contre 32 px de côté. -->
        <component
          :is="isClickable(step, index) ? 'button' : 'div'"
          :type="isClickable(step, index) ? 'button' : undefined"
          class="flex w-full items-center gap-3 p-4 text-start transition-colors duration-(--duration-fast)"
          :class="[
            CELLS[stateOf(step, index)],
            isClickable(step, index) ? 'hover:bg-surface-hover' : '',
          ]"
          :aria-current="stateOf(step, index) === 'current' ? 'step' : undefined"
          @click="select(step, index)"
        >
          <span
            class="flex size-6.5 shrink-0 items-center justify-center rounded-full text-xs font-semibold tabular-nums"
            :class="MARKERS[stateOf(step, index)]"
          >
            <UiIcon v-if="stateOf(step, index) === 'done'" name="check" size="0.85rem" :stroke-width="2.6" />
            <UiIcon v-else-if="stateOf(step, index) === 'error'" name="warning" size="0.85rem" :stroke-width="2.2" />
            <template v-else>{{ index + 1 }}</template>
          </span>

          <span class="min-w-0">
            <span class="block text-sm leading-snug font-semibold" :class="TITLES[stateOf(step, index)]">
              {{ step.label }}
            </span>
            <!-- Sous-libellé d'avancement : la précision de l'appelant quand il
                 en donne une (« 2 sur 5 »), l'état de l'étape sinon. La cellule
                 ne reste jamais muette sous son titre. -->
            <span
              class="mt-0.5 block text-xs"
              :class="[
                stateOf(step, index) === 'error' ? 'text-danger' : 'text-text-muted',
              ]"
            >
              {{ step.description ?? t(SHORT_LABELS[stateOf(step, index)]) }}
            </span>
            <!-- L'état reste annoncé même quand le sous-libellé porte autre chose. -->
            <span v-if="step.description" class="sr-only"> — {{ t(LABELS[stateOf(step, index)]) }}</span>
          </span>
        </component>
      </li>
    </ol>
  </nav>
</template>
