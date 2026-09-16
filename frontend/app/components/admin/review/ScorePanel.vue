<script setup lang="ts">
import type { MyReview, ReviewDeskPermissions, SaveReviewPayload } from '~/types/admin-review'
import type { ReviewCriterion } from '~/types/event/call'
import type { ReviewMode, ReviewRecommendation } from '~/types/programme/review'
import type { CriterionId, Numeric, TimeZoneName } from '~/types/shared'

/**
 * MA NOTE — rendue dans la fenêtre flottante de la fiche.
 *
 * DEUX MANIÈRES DE NOTER (arbitré le 16/09) : une note directe sur 20, quand le
 * temps manque, ou la grille pondérée de l'appel. Le choix est retenu d'une
 * visite à l'autre. Noter ne conditionne jamais la décision, et toute personne
 * qui détient le droit de noter le peut, désignée ou non.
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

const MODE_STORAGE_KEY = 'epavillon.review.mode'

function rememberedMode(): ReviewMode {
  try {
    return localStorage.getItem(MODE_STORAGE_KEY) === 'detailed' ? 'detailed' : 'quick'
  } catch {
    return 'quick'
  }
}

const mode = ref<ReviewMode>(props.myReview.review?.mode ?? 'quick')
onMounted(() => {
  if (!props.myReview.review) mode.value = rememberedMode()
})

function setMode(value: string): void {
  mode.value = value === 'detailed' ? 'detailed' : 'quick'
  dirty.value = true
  try {
    localStorage.setItem(MODE_STORAGE_KEY, mode.value)
  } catch {
    // Un navigateur qui refuse le stockage garde simplement le choix par défaut.
  }
}

const quickScore = ref<number | null>(quickScoreOf(props.myReview))
const generalComment = ref(props.myReview.review?.comment ?? '')

function quickScoreOf(mine: MyReview): number | null {
  return mine.review?.mode === 'quick' && mine.review.score_out_of_20 !== null
    ? Number(mine.review.score_out_of_20)
    : null
}

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
    if (mine.review) mode.value = mine.review.mode
    quickScore.value = quickScoreOf(mine)
    generalComment.value = mine.review?.comment ?? ''
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

const modeOptions = computed(() =>
  (['quick', 'detailed'] as ReviewMode[]).map((value) => ({
    value,
    label: t(`admin.proposal.review.mode.${value}`),
  })),
)

/** Grille close : on ne note pas un dossier dont on s'est déporté. */
const readOnly = computed(() => !props.permissions.can_review || props.permissions.is_recused)

const submittedAt = computed(() => props.myReview.review?.submitted_at ?? null)

function saveQuick(): void {
  if (quickScore.value === null) return
  emit('save', {
    mode: 'quick',
    recommendation: quickRecommendation(quickScore.value),
    score_out_of_20: quickScore.value,
    comment: generalComment.value.trim() || null,
    scores: {},
    comments: {},
    strengths: null,
    weaknesses: null,
    private_note: privateNote.value.trim() || null,
    submit: true,
  })
  dirty.value = false
}

function save(submit: boolean): void {
  emit('save', {
    mode: 'detailed',
    score_out_of_20: null,
    comment: generalComment.value.trim() || null,
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
  <div>
    <!-- Deux raisons de ne pas pouvoir noter, deux messages : un panneau grisé
         sans explication laisse chercher la manipulation qui manque. -->
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

    <div v-else class="flex flex-col gap-5 p-5">
      <p class="text-sm text-text-muted">{{ t('admin.proposal.review.panel.optional') }}</p>

      <UiAlert v-if="props.error" intent="danger" live :message="props.error" />

      <UiRadio
        :model-value="mode"
        :label="t('admin.proposal.review.mode.label')"
        :options="modeOptions"
        :disabled="props.busy"
        inline
        @update:model-value="setMode"
      />

      <template v-if="mode === 'quick'">
        <AdminReviewQuickScore
          :model-value="quickScore"
          :disabled="props.busy"
          @update:model-value="(value: number) => { quickScore = value; dirty = true }"
        />

        <UiTextarea
          v-model="generalComment"
          :label="t('admin.proposal.review.quick.comment')"
          :placeholder="t('admin.proposal.review.quick.commentPlaceholder')"
          :rows="3"
          auto-grow
          block
          :disabled="props.busy"
          @update:model-value="dirty = true"
        />

        <div class="flex flex-col gap-2">
          <p v-if="submittedAt && !dirty" class="text-sm text-success">
            {{ t('admin.proposal.review.panel.submitted', { date: dateTime(submittedAt, props.timezone) }) }}
          </p>
          <p v-else-if="dirty" class="text-sm text-warning">{{ t('admin.proposal.review.panel.unsaved') }}</p>
          <UiButton
            variant="primary"
            block
            :loading="props.busy"
            :disabled="readOnly || quickScore === null"
            @click="saveQuick"
          >
            {{ submittedAt ? t('admin.proposal.review.quick.update') : t('admin.proposal.review.quick.save') }}
          </UiButton>
        </div>
      </template>

      <template v-else>

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
      </template>

      <!-- LE DÉPORT, EN BAS ET DISCRET. Il date une affectation : sans
           désignation, il n'y a rien à déclarer, on s'abstient de noter. -->
      <div v-if="props.permissions.is_assigned" class="border-t border-border-subtle pt-4">
        <UiButton variant="ghost" size="sm" icon="ban" :disabled="props.busy" @click="emit('recuse')">
          {{ t('admin.proposal.review.recusal.action') }}
        </UiButton>
      </div>
    </div>
  </div>
</template>
