<script setup lang="ts">
import type { CommitteeMemberProgress, PeerReview } from '~/types/admin-review'
import type { ReviewCriterion } from '~/types/event/call'
import type { Numeric, TimeZoneName, Uuid } from '~/types/shared'

/**
 * LES REVUES DES AUTRES — et le voile de l'évaluation en aveugle.
 *
 * CE COMPOSANT NE MASQUE RIEN, ET C'EST LE POINT. Quand l'appel est en aveugle
 * (`calls_for_proposals.blind_review`) et que la personne n'a pas déposé sa
 * revue, les revues des pairs NE SONT PAS DANS LA RÉPONSE : `peer_reviews` est
 * vide et `veiled_count` dit combien il en existe. Un masquage côté écran
 * laisserait les notes dans le corps de la réponse, donc lisibles de qui ouvre
 * l'onglet réseau — un voile qui ne cache rien vaut moins que pas de voile, il
 * donne l'illusion d'une garantie.
 *
 * CE QUI RESTE VISIBLE SOUS LE VOILE : COMBIEN, et de QUI on attend une revue.
 * Savoir que deux revues existent n'ancre personne ; lire leurs notes, si. Et
 * l'avancement nominatif reste indispensable pour relancer un collègue en
 * retard, ce qui n'a rien à voir avec la note qu'il a mise.
 *
 * L'AVANCEMENT DU COMITÉ EST DONC AFFICHÉ DANS LES DEUX CAS, voile levé ou non.
 * Il vient de `review_assignments` croisées aux revues, avec les déports :
 * quelqu'un qui s'est retiré n'est ni en retard ni attendu.
 */

interface Props {
  peerReviews: PeerReview[]
  committee: CommitteeMemberProgress[]
  criteria: ReviewCriterion[]
  maxWeightedScore: Numeric
  blindVeiled: boolean
  veiledCount: number
  requiredReviews: number | null
  timezone: TimeZoneName
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()

const criterionById = computed(() => new Map(props.criteria.map((criterion) => [criterion.id, criterion])))

/** Un seul détail ouvert à la fois : six critères par revue, trois revues. */
const openDetail = ref<Uuid | null>(null)

const missing = computed(() => reviewsMissing(props.committee, props.requiredReviews))
</script>

<template>
  <section class="rounded-lg border border-border bg-surface-raised" aria-labelledby="peer-reviews-title">
    <header class="border-b border-border-subtle px-5 py-4">
      <h2 id="peer-reviews-title" class="text-lg font-semibold">
        {{ t('admin.proposal.review.peers.title') }}
      </h2>
    </header>

    <div class="flex flex-col gap-4 p-5">
      <!-- LE VOILE. Il s'explique : sans raison donnée, un panneau vide passe
           pour une panne, et l'on cherche ailleurs ce qu'on croit avoir perdu. -->
      <UiAlert
        v-if="props.blindVeiled"
        intent="info"
        icon="eye-off"
        :title="t('admin.proposal.review.peers.blind.title')"
        :message="t('admin.proposal.review.peers.blind.description')"
      >
        <p class="mt-1 text-sm font-medium">
          {{ t('admin.proposal.review.peers.blind.count', props.veiledCount) }}
        </p>
      </UiAlert>

      <template v-else>
        <p v-if="props.peerReviews.length === 0" class="text-sm text-text-muted">
          {{ t('admin.proposal.review.peers.empty') }}
        </p>

        <article
          v-for="entry in props.peerReviews"
          :key="entry.review.id"
          class="rounded-md border border-border bg-surface px-4 py-3"
        >
          <header class="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
            <span class="font-semibold text-text">{{ entry.reviewer?.display_name ?? '' }}</span>
            <span class="text-lg font-semibold tabular-nums">
              {{ t('admin.proposal.review.peers.score', { score: entry.review.score_out_of_20 ?? '—' }) }}
            </span>
          </header>

          <div class="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1">
            <UiBadge
              size="sm"
              :intent="recommendationIntent(entry.review.recommendation)"
              :label="t(`admin.proposal.review.panel.recommendationValue.${entry.review.recommendation}`)"
            />
            <span class="text-sm text-text-subtle">
              {{
                t('admin.proposal.review.peers.weighted', {
                  score: entry.review.weighted_score ?? '—',
                  max: props.maxWeightedScore,
                })
              }}
            </span>
            <time v-if="entry.review.submitted_at" :datetime="entry.review.submitted_at" class="text-sm text-text-subtle">
              {{
                t('admin.proposal.review.peers.submittedAt', {
                  date: date(entry.review.submitted_at, props.timezone),
                })
              }}
            </time>
          </div>

          <dl v-if="entry.review.strengths || entry.review.weaknesses" class="mt-3 flex flex-col gap-2 text-sm">
            <div v-if="entry.review.strengths">
              <dt class="text-text-subtle">{{ t('admin.proposal.review.peers.strengths') }}</dt>
              <dd class="text-text-secondary">{{ entry.review.strengths }}</dd>
            </div>
            <div v-if="entry.review.weaknesses">
              <dt class="text-text-subtle">{{ t('admin.proposal.review.peers.weaknesses') }}</dt>
              <dd class="text-text-secondary">{{ entry.review.weaknesses }}</dd>
            </div>
          </dl>

          <!-- LE DÉTAIL PAR CRITÈRE EST REPLIÉ. C'est ce qu'on ouvre quand une
               note surprend — rarement —, et le déplier d'office ferait de ce
               panneau six blocs de six lignes. -->
          <UiButton
            variant="ghost"
            size="sm"
            class="mt-2"
            :icon="openDetail === entry.review.id ? 'chevron-up' : 'chevron-down'"
            :aria-expanded="openDetail === entry.review.id"
            @click="openDetail = openDetail === entry.review.id ? null : entry.review.id"
          >
            {{
              openDetail === entry.review.id
                ? t('admin.proposal.review.peers.hideDetail')
                : t('admin.proposal.review.peers.detail')
            }}
          </UiButton>

          <ul v-if="openDetail === entry.review.id" class="mt-2 flex flex-col divide-y divide-border-subtle text-sm">
            <li v-for="score in entry.scores" :key="score.criterion_id" class="py-2">
              <p class="flex items-baseline justify-between gap-3">
                <span class="text-text-secondary">
                  {{ tr(criterionById.get(score.criterion_id)?.label) }}
                </span>
                <span class="font-semibold tabular-nums">
                  {{ score.score }} / {{ criterionById.get(score.criterion_id)?.max_score ?? '' }}
                </span>
              </p>
              <p v-if="score.comment" class="mt-1 text-text-muted">{{ score.comment }}</p>
            </li>
          </ul>
        </article>
      </template>

      <!-- L'AVANCEMENT NOMINATIF, VOILE OU PAS. On relance quelqu'un parce que
           sa revue manque, jamais parce qu'on sait ce qu'il a mis. -->
      <div class="border-t border-border-subtle pt-4">
        <h3 class="text-sm font-semibold tracking-wide uppercase">
          {{ t('admin.proposal.review.committee.title') }}
        </h3>

        <p v-if="props.committee.length === 0" class="mt-2 text-sm text-text-muted">
          {{ t('admin.proposal.review.committee.empty') }}
        </p>

        <ul v-else class="mt-2 flex flex-col gap-2">
          <li
            v-for="entry in props.committee"
            :key="entry.assignment.id"
            class="flex flex-wrap items-baseline justify-between gap-x-3 gap-y-0.5"
          >
            <span
              class="text-sm"
              :class="entry.state === 'recused' ? 'text-text-subtle line-through' : 'text-text-secondary'"
            >
              {{ entry.person?.display_name ?? '' }}
            </span>
            <span class="flex flex-wrap items-center gap-2">
              <UiBadge
                size="sm"
                :intent="progressIntent(entry.state)"
                :label="t(`admin.proposal.review.committee.state.${entry.state}`)"
              />
              <span class="text-xs text-text-subtle">
                <template v-if="entry.state === 'recused' && entry.assignment.recused_at">
                  {{
                    t('admin.proposal.review.committee.recusedOn', {
                      date: date(entry.assignment.recused_at, props.timezone),
                    })
                  }}
                </template>
                <template v-else-if="entry.assignment.due_at">
                  {{
                    t('admin.proposal.review.committee.dueAt', {
                      date: date(entry.assignment.due_at, props.timezone),
                    })
                  }}
                </template>
                <template v-else>{{ t('admin.proposal.review.committee.noDue') }}</template>
              </span>
            </span>
          </li>
        </ul>

        <p class="mt-3 text-sm" :class="missing > 0 ? 'text-warning' : 'text-success'">
          {{
            missing > 0
              ? t('admin.proposal.review.committee.missing', missing)
              : t('admin.proposal.review.committee.complete')
          }}
        </p>
      </div>
    </div>
  </section>
</template>
