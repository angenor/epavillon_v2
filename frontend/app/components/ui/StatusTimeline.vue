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
 * · `skipped` — étape non concernée (un dossier retiré ne passe pas en décision) ;
 * · `refused` — refus : la demande n'est pas retenue.
 * Une frise qui n'afficherait que des étapes franchies mentirait sur la moitié
 * des dossiers.
 *
 * RÈGLE DU GUIDE, ET ELLE EST FERME : « un refus ne clôt jamais la frise sans
 * motif écrit ni proposition de suite. » Un `refused` sans `detail` renvoie une
 * organisation à un mur, sans savoir quoi corriger ni s'il existe une session
 * ultérieure — c'est le message qui produit les courriels de réclamation que la
 * frise devait éviter. Le composant ne peut pas inventer ce motif : il le SIGNALE
 * en développement (`console.warn`) pour que l'écran fautif soit corrigé, et il
 * réserve l'espace du détail dans tous les cas.
 *
 * LE CHEMIN PARCOURU EST VERT. Le trait qui DESCEND d'une étape franchie prend
 * `--color-success-solid` ; celui qui mène à une étape à venir reste en
 * pointillés gris. On lit d'un coup jusqu'où le dossier est allé, sans compter
 * les pastilles.
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
  /**
   * Masque les `detail` sans les retirer des étapes.
   *
   * À RÉSERVER À L'ORIENTATION HORIZONTALE, et pour une raison mesurée : un
   * motif de décision fait deux lignes en vertical et six dans une colonne de
   * frise horizontale, où il chevauche alors ses voisines. La frise résumée
   * d'une liste dit OÙ en est le dossier ; le motif se lit sur sa fiche, où la
   * place existe. Les étapes restent inchangées — la vérification du refus muet
   * continue donc de s'appliquer.
   */
  hideDetails?: boolean
}

const props = withDefaults(defineProps<Props>(), { orientation: 'vertical' })

const { t } = useI18n()
const { date, dateTime } = useDateTime()

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
  // Aplat vert plein, pas un contour : l'étape franchie est un fait acquis, elle
  // doit se voir depuis l'autre bout de la frise.
  done: 'border-success-solid bg-success-solid text-success-contrast',
  current: 'border-accent bg-accent-solid text-accent-contrast',
  upcoming: 'border-border bg-surface-raised text-text-subtle',
  error: 'border-danger bg-danger-surface text-danger',
  refused: 'border-danger-solid bg-danger-solid text-danger-contrast',
  skipped: 'border-border border-dashed bg-surface-sunken text-text-subtle',
}

const MARKER_ICONS: Record<StepState, string | null> = {
  done: 'check',
  current: null,
  upcoming: null,
  error: 'warning',
  refused: 'close',
  skipped: 'minus',
}

const STATE_LABELS: Record<StepState, string> = {
  done: 'status-timeline.state.done',
  current: 'status-timeline.state.current',
  upcoming: 'status-timeline.state.upcoming',
  error: 'status-timeline.state.error',
  refused: 'status-timeline.state.refused',
  skipped: 'status-timeline.state.skipped',
}

function stateLabel(state: StepState): string {
  return t(STATE_LABELS[state])
}

function dateOf(step: TimelineStep): string {
  if (!step.at || !props.timezone) return ''
  // Une étape datée au jour n'affiche pas d'heure : celle qu'on lirait serait
  // le produit d'une conversion de fuseau, pas une saisie.
  return step.dateOnly ? date(step.at, props.timezone) : dateTime(step.at, props.timezone)
}

/**
 * Un refus muet est une faute d'écran, pas un cas de figure : on l'annonce au
 * développement plutôt que de le laisser passer en production.
 */
if (import.meta.dev) {
  watchEffect(() => {
    props.steps.forEach((step, index) => {
      if (stateOf(step, index) === 'refused' && !step.detail) {
        console.warn(
          `[UiStatusTimeline] L'étape « ${step.value} » est un refus sans « detail » : ` +
            'un refus ne clôt jamais la frise sans motif écrit ni proposition de suite.',
        )
      }
    })
  })
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
        <!-- Trait de liaison, arrêté avant la dernière étape. Il appartient à
             l'étape dont il DESCEND : vert plein quand celle-ci est franchie
             (le chemin parcouru se colore), pointillés gris quand il mène à une
             étape encore à venir, plein gris entre deux étapes hors parcours.

             LA GÉOMÉTRIE SE DÉDUIT DE LA PASTILLE, ET D'ELLE SEULE. Elle fait
             `size-7` (28 px) : son axe tombe à 14 px, moins la moitié des 2 px
             du trait, d'où le 0,8125 rem transversal ; et elle occupe les 28
             premiers pixels, d'où le 1,75 rem de départ. Le trait part donc au
             ras de sa pastille et court jusqu'au bord de l'étape, où commence
             celle qui suit : LES DEUX BOUTS TOUCHENT. Toute valeur choisie « à
             l'œil » rouvre le défaut qu'on corrige ici — des traits flottant
             entre deux pastilles, qui ne se rejoignaient ni en haut ni en bas.

             En horizontal, cette géométrie ne vaut qu'à partir de `sm` : sous
             cette largeur la frise retombe en colonne, donc sur le cas vertical. -->
        <span
          v-if="index < props.steps.length - 1"
          aria-hidden="true"
          class="absolute"
          :class="[
            props.orientation === 'horizontal'
              ? 'top-7 bottom-0 left-3.25 w-(--border-medium) sm:top-3.25 sm:right-0 sm:bottom-auto sm:left-7 sm:h-(--border-medium) sm:w-auto'
              : 'top-7 bottom-0 left-3.25 w-(--border-medium)',
            stateOf(step, index) === 'done'
              ? 'bg-success-solid'
              : stateOf(props.steps[index + 1] as TimelineStep, index + 1) === 'upcoming'
                ? 'border-l-(length:--border-medium) border-dashed border-border sm:border-t-(length:--border-medium) sm:border-l-0'
                : 'bg-border',
          ]"
        />

        <span
          class="relative z-10 flex size-7 shrink-0 items-center justify-center rounded-full border-(length:--border-medium) transition-colors duration-(--duration-fast)"
          :class="MARKERS[stateOf(step, index)]"
        >
          <UiIcon
            v-if="MARKER_ICONS[stateOf(step, index)]"
            :name="MARKER_ICONS[stateOf(step, index)] as string"
            size="0.875rem"
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
              stateOf(step, index) === 'refused' ? 'font-semibold text-danger' : '',
            ]"
          >
            {{ step.label }}
            <span class="sr-only"> — {{ stateLabel(stateOf(step, index)) }}</span>
          </p>

          <!-- La date, dans le fuseau de l'édition. Sans date, l'étape est en
               attente et le dit : un blanc laisserait croire à un oubli. -->
          <!-- « En attente » qualifie ce qui n'est PAS ENCORE ARRIVÉ, et rien
               d'autre. Une étape franchie dont la date n'est pas connue — le
               journal ne remonte pas toujours à la première COP — n'est pas en
               attente : elle n'affiche simplement pas d'heure. Écrire « en
               attente » sous une étape verte est un contresens, et c'est
               exactement ce qu'on lisait sur un dossier retenu. -->
          <p class="mt-0.5 text-xs text-text-subtle">
            <time v-if="step.at" :datetime="step.at" class="tabular-nums">{{ dateOf(step) }}</time>
            <span v-else-if="stateOf(step, index) === 'upcoming'">{{ t('status-timeline.pending') }}</span>
            <span v-if="step.actor"> · {{ t('status-timeline.by', { actor: step.actor }) }}</span>
          </p>

          <!-- Le motif d'un refus est OBLIGATOIRE (voir l'en-tête) : le bloc est
               donc bordé de rouge sur cet état, pour que son absence saute aux
               yeux à la relecture d'un écran. -->
          <p
            v-if="step.detail && !props.hideDetails"
            class="mt-1.5 max-w-(--measure) rounded-md border px-2.5 py-1.5 text-sm"
            :class="
              stateOf(step, index) === 'refused'
                ? 'border-danger-border bg-danger-surface text-text-secondary'
                : 'border-border-subtle bg-surface-sunken text-text-muted'
            "
          >
            {{ step.detail }}
          </p>
        </div>
      </li>
    </ol>
  </div>
</template>
