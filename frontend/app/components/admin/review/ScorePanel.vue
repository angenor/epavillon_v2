<script setup lang="ts">
import type { MyReview, ReviewDeskPermissions, SaveReviewPayload } from '~/types/admin-review'
import type { ReviewCriterion } from '~/types/event/call'
import type { ReviewRecommendation } from '~/types/programme/review'
import type { CriterionId, Numeric, TimeZoneName } from '~/types/shared'

/**
 * LE PANNEAU D'ÉVALUATION — la grille pondérée, le total recalculé en direct, la
 * recommandation, les points forts et faibles, le déport.
 *
 * IL COLLE AU DÉFILEMENT, et ce n'est pas un effet : on note EN LISANT. Le
 * dossier fait plusieurs écrans de haut ; un panneau qui défile avec lui
 * obligerait à remonter après chaque paragraphe, ou à noter de mémoire. Sur
 * écran étroit, la colle est retirée — coller un panneau de 600 px sur un
 * téléphone masque le texte qu'il sert à juger.
 *
 * LE TOTAL SE RECALCULE À CHAQUE NOTE, et il le fait comme la base :
 * `somme(score × poids)`, ramenée sur 20 par `event.max_weighted_score()`. Les
 * deux calculs vivent dans `utils/review-scoring.ts`, pas ici : le mock rejoue
 * `refresh_proposal_score()` avec les mêmes règles, et deux implémentations
 * afficheraient deux totaux pour les mêmes notes.
 *
 * UN CRITÈRE NON NOTÉ N'EST PAS UN ZÉRO. Zéro sur le critère éliminatoire
 * DISQUALIFIE le dossier ; une case vide ne dit rien. Le total ne compte donc
 * que ce qui est posé, et l'écran annonce combien de critères restent à noter
 * plutôt que d'afficher un total qui aurait l'air complet.
 *
 * L'AVERTISSEMENT ÉLIMINATOIRE EST NET ET NOMMÉ. « Un critère éliminatoire est à
 * zéro » ne suffit pas : l'écran dit LEQUEL, parce que c'est ce qu'on vérifie
 * avant de déposer une revue qui écarte un dossier.
 *
 * LE BROUILLON EST UN ÉTAT DU MODÈLE, pas un confort d'interface : une revue
 * dont `submitted_at` est nul ne compte dans aucun agrégat et reste invisible du
 * comité. Enregistrer et déposer sont donc deux boutons, et le second dit ce
 * qu'il déclenche.
 */

interface Props {
  criteria: ReviewCriterion[]
  maxWeightedScore: Numeric
  myReview: MyReview
  permissions: ReviewDeskPermissions
  timezone: TimeZoneName
  busy?: boolean
  error?: string | null
  /** Heure du dernier enregistrement réussi, pour l'accusé discret. */
  savedAt?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  save: [payload: Omit<SaveReviewPayload, 'proposal_id'>]
  recuse: []
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date, dateTime } = useDateTime()

// ---------------------------------------------------------------------------
// L'état du formulaire
// ---------------------------------------------------------------------------

const scores = ref<Record<CriterionId, Numeric>>({ ...props.myReview.scores })
const comments = ref<Record<CriterionId, string>>({ ...props.myReview.comments })
const recommendation = ref<ReviewRecommendation>(props.myReview.review?.recommendation ?? 'neutral')
const strengths = ref(props.myReview.review?.strengths ?? '')
const weaknesses = ref(props.myReview.review?.weaknesses ?? '')
const privateNote = ref(props.myReview.review?.private_note ?? '')
const openComments = ref<Set<CriterionId>>(new Set(Object.keys(props.myReview.comments)))
const dirty = ref(false)

/** Une revue rechargée depuis le serveur écrase la saisie : c'est elle qui fait foi. */
watch(
  () => props.myReview,
  (mine) => {
    scores.value = { ...mine.scores }
    comments.value = { ...mine.comments }
    recommendation.value = mine.review?.recommendation ?? 'neutral'
    strengths.value = mine.review?.strengths ?? ''
    weaknesses.value = mine.review?.weaknesses ?? ''
    privateNote.value = mine.review?.private_note ?? ''
    openComments.value = new Set(Object.keys(mine.comments))
    dirty.value = false
  },
)

function setScore(criterion: ReviewCriterion, value: number): void {
  scores.value = { ...scores.value, [criterion.id]: value }
  dirty.value = true
}

function toggleComment(criterionId: CriterionId): void {
  const next = new Set(openComments.value)
  if (next.has(criterionId)) next.delete(criterionId)
  else next.add(criterionId)
  openComments.value = next
}

// ---------------------------------------------------------------------------
// Le calcul, en direct
// ---------------------------------------------------------------------------

const total = computed(() => weightedTotal(scores.value, props.criteria))
const outOf20 = computed(() => scoreOutOfTwenty(total.value, props.maxWeightedScore))
const breaches = computed(() => knockoutBreaches(scores.value, props.criteria))
const missing = computed(() => missingScores(scores.value, props.criteria))

const recommendationOptions = computed(() =>
  (['accept', 'accept_with_changes', 'neutral', 'reject'] as ReviewRecommendation[]).map(
    (value) => ({
      value,
      label: t(`admin.proposal.review.panel.recommendationValue.${value}`),
    }),
  ),
)

/** Grille close : on ne note pas un dossier dont on s'est déporté. */
const readOnly = computed(
  () => !props.permissions.can_review || !props.permissions.is_assigned || props.permissions.is_recused,
)

const submittedAt = computed(() => props.myReview.review?.submitted_at ?? null)

function save(submit: boolean): void {
  emit('save', {
    recommendation: recommendation.value,
    scores: scores.value,
    comments: comments.value,
    strengths: strengths.value.trim() || null,
    weaknesses: weaknesses.value.trim() || null,
    private_note: privateNote.value.trim() || null,
    submit,
  })
  dirty.value = false
}
</script>

<template>
  <section
    class="rounded-lg border border-border bg-surface-raised"
    aria-labelledby="review-panel-title"
  >
    <header class="border-b border-border-subtle px-5 py-4">
      <h2 id="review-panel-title" class="text-lg font-semibold">
        {{ t('admin.proposal.review.panel.title') }}
      </h2>
      <p class="mt-1 text-sm text-text-muted">
        {{ t('admin.proposal.review.panel.description') }}
      </p>
    </header>

    <!-- TROIS RAISONS DE NE PAS POUVOIR NOTER, ET TROIS MESSAGES DIFFÉRENTS :
         le déport déclaré, le dossier non confié, le droit absent. Un panneau
         grisé sans explication laisse chercher la manipulation qui manque. -->
    <div v-if="props.permissions.is_recused" class="p-5">
      <UiAlert
        intent="neutral"
        icon="ban"
        :title="
          t('admin.proposal.review.recusal.done', {
            date: props.myReview.assignment?.recused_at
              ? date(props.myReview.assignment.recused_at, props.timezone)
              : '',
          })
        "
        :message="
          props.myReview.assignment?.recusal_reason
            ? t('admin.proposal.review.recusal.doneReason', {
                reason: props.myReview.assignment.recusal_reason,
              })
            : undefined
        "
      />
    </div>

    <div v-else-if="!props.permissions.can_review" class="p-5">
      <UiAlert
        intent="info"
        :title="t('admin.proposal.review.panel.cannotReview.title')"
        :message="t('admin.proposal.review.panel.cannotReview.description')"
      />
    </div>

    <div v-else-if="!props.permissions.is_assigned" class="p-5">
      <UiAlert
        intent="info"
        :title="t('admin.proposal.review.panel.notAssigned.title')"
        :message="t('admin.proposal.review.panel.notAssigned.description')"
      />
    </div>

    <div v-else class="flex flex-col gap-5 p-5">
      <UiAlert v-if="props.error" intent="danger" live :message="props.error" />

      <!-- L'AVERTISSEMENT ÉLIMINATOIRE, NOMMÉ. Il apparaît à la note, pas au
           dépôt : c'est avant de valider qu'il faut savoir ce qu'on fait. -->
      <UiAlert
        v-if="breaches.length > 0"
        intent="danger"
        live
        icon="ban"
        :message="
          t('admin.proposal.review.panel.knockoutWarning', {
            criteria: breaches.map((criterion) => tr(criterion.label)).join(', '),
          })
        "
      />

      <!-- LA GRILLE ------------------------------------------------------- -->
      <fieldset class="flex flex-col gap-5">
        <legend class="sr-only">{{ t('admin.proposal.review.panel.grid') }}</legend>

        <div v-for="criterion in props.criteria" :key="criterion.id" class="flex flex-col gap-2">
          <div class="flex flex-wrap items-baseline gap-x-2 gap-y-1">
            <span class="font-medium text-text">{{ tr(criterion.label) }}</span>
            <span class="text-xs text-text-subtle">
              {{ t('admin.proposal.review.panel.weight', { weight: criterion.weight }) }} ·
              {{ t('admin.proposal.review.panel.max', { max: criterion.max_score }) }}
            </span>
            <UiBadge
              v-if="criterion.is_knockout"
              intent="warning"
              size="sm"
              :label="t('admin.proposal.review.panel.knockout')"
            />
          </div>

          <p v-if="criterion.description" class="max-w-(--measure) text-sm text-text-muted">
            {{ tr(criterion.description) }}
          </p>
          <p v-if="criterion.is_knockout" class="text-sm text-warning">
            {{ t('admin.proposal.review.panel.knockoutHint') }}
          </p>

          <!-- LES NOTES SONT DES BOUTONS, PAS UNE LISTE DÉROULANTE. Six critères
               notés de 0 à 5 font trente-six clics dans une liste ; ici, un seul
               par critère. Chaque cible fait 44 px — c'est la règle de la
               charte, et ce panneau se remplit aussi sur tablette en réunion. -->
          <div class="flex flex-wrap gap-1.5" role="radiogroup" :aria-label="tr(criterion.label)">
            <button
              v-for="choice in scoreChoices(criterion)"
              :key="choice"
              type="button"
              role="radio"
              :aria-checked="scores[criterion.id] === choice"
              :disabled="props.busy"
              class="min-h-(--target-min) min-w-(--target-min) rounded-md border px-3 text-base font-semibold tabular-nums transition-colors disabled:cursor-not-allowed disabled:opacity-60"
              :class="
                scores[criterion.id] === choice
                  ? choice === 0 && criterion.is_knockout
                    ? 'border-transparent bg-danger-solid text-danger-contrast'
                    : 'border-transparent bg-accent-solid text-accent-contrast'
                  : 'border-border bg-surface text-text-muted hover:bg-surface-hover'
              "
              @click="setScore(criterion, choice)"
            >
              {{ choice }}
            </button>

            <UiButton
              variant="ghost"
              size="sm"
              icon="mail"
              class="ml-auto"
              :aria-expanded="openComments.has(criterion.id)"
              @click="toggleComment(criterion.id)"
            >
              {{ t('admin.proposal.review.panel.addComment') }}
            </UiButton>
          </div>

          <UiTextarea
            v-if="openComments.has(criterion.id)"
            :id="`criterion-comment-${criterion.id}`"
            :model-value="comments[criterion.id] ?? ''"
            :label="t('admin.proposal.review.panel.criterionComment')"
            :placeholder="t('admin.proposal.review.panel.criterionCommentPlaceholder')"
            :rows="2"
            auto-grow
            hide-label
            block
            :disabled="props.busy"
            @update:model-value="
              (value: string) => {
                comments = { ...comments, [criterion.id]: value }
                dirty = true
              }
            "
          />
        </div>
      </fieldset>

      <!-- LE TOTAL, RECALCULÉ EN DIRECT ------------------------------------ -->
      <div class="rounded-md border border-border bg-surface-sunken px-4 py-3">
        <dl class="flex flex-wrap items-baseline justify-between gap-x-6 gap-y-2">
          <div>
            <dt class="text-xs tracking-wide text-text-subtle uppercase">
              {{ t('admin.proposal.review.panel.total') }}
            </dt>
            <dd class="mt-0.5 text-lg font-semibold tabular-nums">
              {{
                t('admin.proposal.review.panel.totalValue', {
                  total: Math.round(total * 100) / 100,
                  max: props.maxWeightedScore,
                })
              }}
            </dd>
          </div>
          <div class="text-right">
            <dt class="text-xs tracking-wide text-text-subtle uppercase">
              {{ t('admin.proposal.review.panel.outOf20') }}
            </dt>
            <dd class="mt-0.5 text-2xl font-semibold tabular-nums text-accent">
              {{ outOf20 ?? '—' }}
            </dd>
          </div>
        </dl>
        <p v-if="missing.length > 0" class="mt-2 text-sm text-text-muted">
          {{ t('admin.proposal.review.panel.missing', missing.length) }}
        </p>
      </div>

      <UiRadio
        v-model="recommendation"
        :label="t('admin.proposal.review.panel.recommendation')"
        :options="recommendationOptions"
        :disabled="props.busy"
        @update:model-value="dirty = true"
      />

      <UiTextarea
        v-model="strengths"
        :label="t('admin.proposal.review.panel.strengths')"
        :placeholder="t('admin.proposal.review.panel.strengthsPlaceholder')"
        :rows="3"
        auto-grow
        block
        :disabled="props.busy"
        @update:model-value="dirty = true"
      />

      <UiTextarea
        v-model="weaknesses"
        :label="t('admin.proposal.review.panel.weaknesses')"
        :placeholder="t('admin.proposal.review.panel.weaknessesPlaceholder')"
        :rows="3"
        auto-grow
        block
        :disabled="props.busy"
        @update:model-value="dirty = true"
      />

      <!-- LA NOTE PERSONNELLE porte son avertissement de visibilité, comme les
           messages du fil : `reviews.private_note` n'est lue de personne
           d'autre, et il faut le dire pour qu'on ose s'en servir. -->
      <UiTextarea
        v-model="privateNote"
        :label="t('admin.proposal.review.panel.privateNote')"
        :hint="t('admin.proposal.review.panel.privateNoteHint')"
        :rows="2"
        auto-grow
        block
        :disabled="props.busy"
        @update:model-value="dirty = true"
      />

      <div class="flex flex-col gap-2">
        <p v-if="submittedAt" class="text-sm text-success">
          {{
            t('admin.proposal.review.panel.submitted', {
              date: dateTime(submittedAt, props.timezone),
            })
          }}
        </p>
        <p v-else-if="props.savedAt" class="text-sm text-text-muted">
          {{ t('admin.proposal.review.panel.saved', { time: props.savedAt }) }}
        </p>
        <p v-if="dirty" class="text-sm text-warning">
          {{ t('admin.proposal.review.panel.unsaved') }}
        </p>

        <div class="flex flex-wrap gap-2">
          <UiButton
            variant="primary"
            :loading="props.busy"
            :disabled="readOnly || missing.length > 0"
            @click="save(true)"
          >
            {{
              submittedAt
                ? t('admin.proposal.review.panel.resubmit')
                : t('admin.proposal.review.panel.submit')
            }}
          </UiButton>
          <UiButton
            variant="secondary"
            :disabled="props.busy || readOnly"
            @click="save(false)"
          >
            {{ t('admin.proposal.review.panel.save') }}
          </UiButton>
        </div>

        <p class="text-sm text-text-subtle">
          {{
            missing.length > 0
              ? t('admin.proposal.review.panel.submitBlocked')
              : t('admin.proposal.review.panel.submitHint')
          }}
        </p>
      </div>

      <!-- LE DÉPORT, EN BAS ET DISCRET. C'est un geste rare et grave : il n'a
           pas sa place à côté des notes, mais il doit être atteignable sans
           écrire à l'IFDD. -->
      <div class="border-t border-border-subtle pt-4">
        <UiButton variant="ghost" size="sm" icon="ban" :disabled="props.busy" @click="emit('recuse')">
          {{ t('admin.proposal.review.recusal.action') }}
        </UiButton>
      </div>
    </div>
  </section>
</template>
