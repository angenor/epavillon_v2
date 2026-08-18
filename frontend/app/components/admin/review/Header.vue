<script setup lang="ts">
import type { CommitteeMemberProgress } from '~/types/admin-review'
import type { Proposal } from '~/types/programme/proposal'
import type { DecisionOption } from '~/utils/review-scoring'
import type { Intent } from '~/types/ui'
import type { ProposalStatus } from '~/types/programme/proposal'
import type { TimeZoneName } from '~/types/shared'

/**
 * L'EN-TÊTE DE LA FICHE — de quel dossier parle-t-on, où en est le comité, et
 * que peut-on en décider.
 *
 * LA DÉCISION EST SÉPARÉE DE LA NOTATION, VISUELLEMENT ET PAS SEULEMENT PAR LE
 * DROIT. Ce sont deux gestes différents faits par deux personnes différentes :
 * un membre du comité NOTE dans le panneau de droite, l'équipe de l'IFDD DÉCIDE
 * ici. Les mêler — un bouton « Retenir » à côté d'une grille de notes — ferait
 * croire qu'une bonne note vaut acceptation, ce qui n'est vrai dans aucun
 * comité. D'où le bloc distinct, en haut, sur fond creusé.
 *
 * LES ACTIONS OFFERTES SONT CELLES QUE LA BASE DÉCLARE, filtrées par ce que la
 * personne a le droit de faire SUR CETTE ÉDITION (`decisionOptions()`, lue dans
 * `proposal_transitions_allowed`). Sans droit, le bloc dit pourquoi il est vide
 * plutôt que de disparaître : un écran amputé sans explication se lit comme une
 * panne.
 *
 * LE NUMÉRO DE DOSSIER EST LE TITRE. C'est par lui qu'on désigne l'affaire au
 * téléphone et dans un courriel — « regarde COP31-00020 » —, et il est
 * sélectionnable au clavier comme du texte, pas dessiné dans une pastille.
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

const done = computed(() => submittedCount(props.committee))

/**
 * L'avancement se dit « 2 sur 3 » quand l'appel fixe un nombre de revues, et
 * « 2 revues rendues » sinon. Un dénominateur inventé — le nombre de personnes
 * affectées — ferait passer pour incomplet un dossier qui ne l'est pas.
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
 * L'action de décision porte l'intention de ce qu'elle FAIT, pas du statut de
 * départ : retenir est un succès, rejeter une suppression, demander des
 * corrections un avertissement. C'est la règle de couleur de la charte.
 */
function decisionVariant(status: ProposalStatus): 'primary' | 'secondary' | 'danger' {
  if (status === 'accepted') return 'primary'
  if (status === 'rejected' || status === 'cancelled') return 'danger'
  return 'secondary'
}
</script>

<template>
  <header class="rounded-lg border border-border bg-surface-raised">
    <div class="flex flex-wrap items-start justify-between gap-x-8 gap-y-4 p-5 sm:p-6">
      <div class="min-w-0">
        <p class="font-mono text-sm tracking-wide text-text-subtle">
          {{ props.proposal.reference_code }}
        </p>
        <h1 class="mt-1 text-2xl leading-tight font-semibold text-balance sm:text-3xl">
          {{ tr(props.proposal.title) }}
        </h1>

        <div class="mt-3 flex flex-wrap items-center gap-2">
          <UiBadge
            :intent="STATUS_INTENT[props.proposal.status]"
            :label="t(`admin.proposal.review.status.${props.proposal.status}`)"
          />
          <UiBadge
            :label="t(`admin.proposal.review.format.${props.proposal.format}`)"
            icon="monitor"
          />
          <!-- L'ÉLIMINATION EST UNE INFORMATION DE PREMIER RANG : sans elle,
               une note moyenne correcte à côté d'un rejet est incompréhensible. -->
          <UiBadge
            v-if="props.proposal.is_knocked_out"
            intent="danger"
            icon="ban"
            :label="t('admin.proposal.review.header.knockedOut')"
          />
          <span class="text-sm text-text-subtle">
            {{
              props.proposal.submitted_at
                ? t('admin.proposal.review.header.submittedAt', {
                    date: date(props.proposal.submitted_at, props.timezone),
                  })
                : t('admin.proposal.review.header.notSubmitted')
            }}
          </span>
        </div>
      </div>

      <!-- LES TROIS CHIFFRES QUE LE COMITÉ REGARDE EN PREMIER. Alignés à droite
           sur écran large, ils passent en ligne sous le titre sur mobile — un
           tableau de bord à trois colonnes de 120 px ne tient pas à 375 px. -->
      <dl class="flex flex-wrap gap-x-8 gap-y-3">
        <div>
          <dt class="text-xs tracking-wide text-text-subtle uppercase">
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
        <div>
          <dt class="text-xs tracking-wide text-text-subtle uppercase">
            {{ t('admin.proposal.review.header.rank') }}
          </dt>
          <dd class="mt-0.5 text-xl font-semibold tabular-nums">{{ rankLabel }}</dd>
        </div>
        <div>
          <dt class="text-xs tracking-wide text-text-subtle uppercase">
            {{ t('admin.proposal.review.header.committee') }}
          </dt>
          <dd class="mt-0.5 text-xl font-semibold tabular-nums">{{ progress }}</dd>
        </div>
      </dl>
    </div>

    <p class="flex flex-wrap items-center gap-x-4 gap-y-1 border-t border-border-subtle px-5 py-3 text-sm text-text-subtle sm:px-6">
      <span>{{ t('admin.proposal.review.header.readBy', props.readCount) }}</span>
      <span v-if="props.firstVisit" class="text-info">
        {{ t('admin.proposal.review.header.firstVisit') }}
      </span>
    </p>

    <!-- LA DÉCISION, À PART. Fond creusé, bordure haute, libellé propre : on ne
         clique pas ici par inadvertance en sortant de la grille de notes. -->
    <div class="rounded-b-lg border-t border-border bg-surface-sunken px-5 py-4 sm:px-6">
      <div class="flex flex-wrap items-center gap-x-4 gap-y-3">
        <div class="min-w-0">
          <h2 class="text-sm font-semibold tracking-wide uppercase">
            {{ t('admin.proposal.review.decision.title') }}
          </h2>
          <p class="mt-0.5 max-w-(--measure) text-sm text-text-muted">
            {{ t('admin.proposal.review.decision.hint') }}
          </p>
        </div>

        <div v-if="props.canDecide && props.decisions.length > 0" class="ml-auto flex flex-wrap gap-2">
          <UiButton
            v-for="option in props.decisions"
            :key="option.to_status"
            :variant="decisionVariant(option.to_status)"
            size="sm"
            :disabled="props.busy"
            @click="emit('decide', option)"
          >
            {{ t(`admin.proposal.review.decision.action.${option.to_status}`) }}
          </UiButton>
        </div>

        <!-- Deux vides différents : « vous n'y avez pas droit » et « rien n'est
             possible depuis cet état ». Les confondre laisserait chercher un
             bouton qui n'existe pour personne. -->
        <p v-else class="ml-auto text-sm text-text-muted">
          {{
            props.canDecide
              ? t('admin.proposal.review.decision.none')
              : t('admin.proposal.review.decision.forbidden')
          }}
        </p>
      </div>
    </div>
  </header>
</template>
