<script setup lang="ts">
import type { CommitteeMemberProgress } from '~/types/admin-review'
import type { Proposal, ProposalStatus } from '~/types/programme/proposal'
import type { DecisionOption } from '~/utils/review-scoring'
import type { Intent } from '~/types/ui'
import type { TimeZoneName } from '~/types/shared'

/**
 * L'EN-TÊTE DE LA FICHE : de quel dossier parle-t-on, où en est le comité, et
 * que peut-on en décider.
 *
 * Composition (29/10) : numéro et date de dépôt en surtitre, titre en grand,
 * états, puis les quatre chiffres du comité sur une ligne de filets, la décision
 * dessous. Les actions de la fiche (modifier, dossier précédent ou suivant)
 * arrivent par la fente `actions`.
 *
 * LA DÉCISION EST SÉPARÉE DE LA NOTATION, VISUELLEMENT ET PAS SEULEMENT PAR LE
 * DROIT. Un membre du comité NOTE dans la fenêtre flottante, l'équipe de l'IFDD
 * DÉCIDE ici. Un bouton « Retenir » à côté d'une grille de notes ferait croire
 * qu'une bonne note vaut acceptation. D'où le bloc distinct, sur fond creusé.
 *
 * LES ACTIONS OFFERTES SONT CELLES QUE LA BASE DÉCLARE, filtrées par ce que la
 * personne a le droit de faire SUR CETTE ÉDITION (`decisionOptions()`). Sans
 * droit, le bloc dit pourquoi il est vide plutôt que de disparaître.
 *
 * LE NUMÉRO DE DOSSIER se lit en surtitre, sélectionnable comme du texte : c'est
 * par lui qu'on désigne l'affaire au téléphone (« regarde COP31-00020 »).
 */

interface Props {
  proposal: Proposal
  rank: number
  requiredReviews: number | null
  committee: CommitteeMemberProgress[]
  readCount: number
  firstVisit: boolean
  timezone: TimeZoneName
  decisions: DecisionOption[]
  canDecide: boolean
  busy?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ decide: [option: DecisionOption] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()

const STATUS_INTENT: Record<ProposalStatus, Intent> = {
  draft: 'neutral',
  submitted: 'info',
  under_review: 'info',
  changes_requested: 'warning',
  accepted: 'success',
  rejected: 'danger',
  withdrawn: 'neutral',
  cancelled: 'danger',
}

const FIGURE = 'min-w-0 py-3 sm:border-s sm:border-border-subtle sm:px-6 sm:first:border-s-0 sm:first:ps-0'

const done = computed(() => submittedCount(props.committee))

/**
 * L'avancement se dit « 2 sur 3 » quand l'appel fixe un nombre de revues, et
 * « 2 revues rendues » sinon. Un dénominateur inventé ferait passer pour
 * incomplet un dossier qui ne l'est pas.
 */
const progress = computed(() =>
  props.requiredReviews === null
    ? t('admin.proposal.review.header.progressNoTarget', done.value)
    : t('admin.proposal.review.header.progress', {
        done: done.value,
        expected: props.requiredReviews,
      }),
)

const rankLabel = computed(() => {
  if (props.proposal.average_score === null) return t('admin.proposal.review.header.noRank')
  return props.rank === 1
    ? t('admin.proposal.review.header.rankFirst')
    : t('admin.proposal.review.header.rankValue', { rank: props.rank })
})

/**
 * L'action de décision porte l'intention de ce qu'elle FAIT : retenir est un
 * succès, rejeter une suppression, demander des corrections un avertissement.
 */
function decisionVariant(status: ProposalStatus): 'primary' | 'secondary' | 'danger' {
  if (status === 'accepted') return 'primary'
  if (status === 'rejected' || status === 'cancelled') return 'danger'
  return 'secondary'
}
</script>

<template>
  <header>
    <div class="flex flex-wrap items-center justify-between gap-x-6 gap-y-2">
      <p class="text-sm text-text-subtle">
        <span class="font-mono tracking-wide">{{ props.proposal.reference_code }}</span>
        ·
        {{
          props.proposal.submitted_at
            ? t('admin.proposal.review.header.submittedAt', {
                date: date(props.proposal.submitted_at, props.timezone),
              })
            : t('admin.proposal.review.header.notSubmitted')
        }}
      </p>
      <div class="flex flex-wrap items-center gap-2">
        <slot name="actions" />
      </div>
    </div>

    <h1 class="mt-2 max-w-[48ch] text-2xl leading-tight font-semibold text-balance sm:text-3xl">
      {{ tr(props.proposal.title) }}
    </h1>

    <div class="mt-4 flex flex-wrap items-center gap-2">
      <UiBadge
        :intent="STATUS_INTENT[props.proposal.status]"
        :label="t(`admin.proposal.review.status.${props.proposal.status}`)"
      />
      <UiBadge
        :label="t(`admin.proposal.review.format.${props.proposal.format}`)"
        icon="monitor"
      />
      <!-- L'ÉLIMINATION EST UNE INFORMATION DE PREMIER RANG : sans elle, une note
           moyenne correcte à côté d'un rejet est incompréhensible. -->
      <UiBadge
        v-if="props.proposal.is_knocked_out"
        intent="danger"
        icon="ban"
        :label="t('admin.proposal.review.header.knockedOut')"
      />
      <span v-if="props.firstVisit" class="text-sm text-info">
        {{ t('admin.proposal.review.header.firstVisit') }}
      </span>
    </div>

    <!-- LES CHIFFRES QUE LE COMITÉ REGARDE EN PREMIER, sur une ligne de filets :
         deux colonnes sur téléphone, une ligne au-delà. -->
    <dl class="mt-6 grid grid-cols-2 gap-x-4 border-y border-border sm:flex sm:flex-wrap sm:gap-x-0">
      <div :class="FIGURE">
        <dt class="text-xs tracking-caps text-text-subtle uppercase">
          {{ t('admin.proposal.review.header.score') }}
        </dt>
        <dd class="mt-0.5 text-xl font-semibold tabular-nums">
          <template v-if="props.proposal.average_score !== null">
            {{ t('admin.proposal.review.header.outOf20', { score: props.proposal.average_score }) }}
          </template>
          <span v-else class="text-base font-normal text-text-muted">
            {{ t('admin.proposal.review.header.noScore') }}
          </span>
        </dd>
      </div>
      <div :class="FIGURE">
        <dt class="text-xs tracking-caps text-text-subtle uppercase">
          {{ t('admin.proposal.review.header.rank') }}
        </dt>
        <dd class="mt-0.5 text-xl font-semibold tabular-nums">{{ rankLabel }}</dd>
      </div>
      <div :class="FIGURE">
        <dt class="text-xs tracking-caps text-text-subtle uppercase">
          {{ t('admin.proposal.review.header.committee') }}
        </dt>
        <dd class="mt-0.5 text-base font-semibold tabular-nums sm:text-xl">{{ progress }}</dd>
      </div>
      <div :class="FIGURE">
        <dt class="text-xs tracking-caps text-text-subtle uppercase">
          {{ t('admin.proposal.review.header.reading') }}
        </dt>
        <dd class="mt-0.5 text-base text-text-secondary sm:mt-1.5">
          {{ t('admin.proposal.review.header.readBy', props.readCount) }}
        </dd>
      </div>
    </dl>

    <!-- LA DÉCISION, À PART. Fond creusé, libellé propre : on ne clique pas ici
         par inadvertance en sortant de la grille de notes. -->
    <div class="mt-4 flex flex-wrap items-center gap-x-6 gap-y-3 rounded-md border border-border bg-surface-sunken px-4 py-3">
      <div class="min-w-0 flex-1 basis-64">
        <h2 class="text-sm font-semibold tracking-caps uppercase">
          {{ t('admin.proposal.review.decision.title') }}
        </h2>
        <p class="mt-0.5 max-w-(--measure) text-sm text-text-muted">
          {{ t('admin.proposal.review.decision.hint') }}
        </p>
      </div>

      <div v-if="props.canDecide && props.decisions.length > 0" class="flex flex-wrap gap-2">
        <UiButton
          v-for="option in props.decisions"
          :key="option.to_status"
          :variant="decisionVariant(option.to_status)"
          :disabled="props.busy"
          @click="emit('decide', option)"
        >
          {{ t(`admin.proposal.review.decision.action.${option.to_status}`) }}
        </UiButton>
      </div>

      <!-- Deux vides différents : « vous n'y avez pas droit » et « rien n'est
           possible depuis cet état ». -->
      <p v-else class="text-sm text-text-muted">
        {{
          props.canDecide
            ? t('admin.proposal.review.decision.none')
            : t('admin.proposal.review.decision.forbidden')
        }}
      </p>
    </div>
  </header>
</template>
