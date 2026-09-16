<script setup lang="ts">
import type {
  CommitteeMemberProgress,
  MyReview,
  PeerReview,
  ReviewDeskPermissions,
  SaveReviewPayload,
} from '~/types/admin-review'
import type { ReviewCriterion } from '~/types/event/call'
import type { Numeric, TimeZoneName } from '~/types/shared'
import type { TabItem } from '~/types/ui'

/**
 * L'évaluation d'un dossier, dans une fenêtre flottante ouverte à la demande
 * (reprise de la v1, arbitré le 16/09) : noter est facultatif, le dossier garde
 * toute la largeur.
 */

interface Props {
  referenceCode: string
  criteria: ReviewCriterion[]
  maxWeightedScore: Numeric
  myReview: MyReview
  permissions: ReviewDeskPermissions
  peerReviews: PeerReview[]
  committee: CommitteeMemberProgress[]
  blindVeiled: boolean
  veiledCount: number
  requiredReviews: number | null
  timezone: TimeZoneName
  busy?: boolean
  error?: string | null
  savedAt?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  save: [payload: Omit<SaveReviewPayload, 'proposal_id'>]
  recuse: []
}>()

const { t } = useI18n()

const open = ref(false)
const tab = ref<'mine' | 'all'>('mine')

const myScore = computed(() => {
  const review = props.myReview.review
  return review?.submitted_at && review.score_out_of_20 !== null
    ? String(Math.round(Number(review.score_out_of_20) * 10) / 10)
    : null
})

const tabs = computed<TabItem[]>(() => [
  { value: 'mine', label: t('admin.proposal.review.floating.tabs.mine') },
  {
    value: 'all',
    label: t('admin.proposal.review.floating.tabs.all'),
    count: props.blindVeiled ? props.veiledCount : props.peerReviews.length,
  },
])

function onSave(payload: Omit<SaveReviewPayload, 'proposal_id'>): void {
  emit('save', payload)
}
</script>

<template>
  <UiFloatingPanel
    v-model:open="open"
    :title="t('admin.proposal.review.floating.title', { reference: props.referenceCode })"
    :launcher-label="
      myScore
        ? t('admin.proposal.review.floating.launcherScored', { score: myScore })
        : t('admin.proposal.review.floating.launcher')
    "
    launcher-icon="star"
    :launcher-value="myScore"
  >
    <UiTabs
      class="px-5 pt-2"
      :items="tabs"
      :model-value="tab"
      :label="t('admin.proposal.review.floating.title', { reference: props.referenceCode })"
      @update:model-value="(value: string) => (tab = value === 'all' ? 'all' : 'mine')"
    />

    <!-- v-show : une saisie en cours survit au passage d'un onglet à l'autre. -->
    <AdminReviewScorePanel
      v-show="tab === 'mine'"
      :criteria="props.criteria"
      :max-weighted-score="props.maxWeightedScore"
      :my-review="props.myReview"
      :permissions="props.permissions"
      :timezone="props.timezone"
      :busy="props.busy"
      :error="props.error"
      :saved-at="props.savedAt"
      @save="onSave"
      @recuse="emit('recuse')"
    />

    <AdminReviewPeerReviews
      v-show="tab === 'all'"
      :peer-reviews="props.peerReviews"
      :committee="props.committee"
      :criteria="props.criteria"
      :max-weighted-score="props.maxWeightedScore"
      :blind-veiled="props.blindVeiled"
      :veiled-count="props.veiledCount"
      :required-reviews="props.requiredReviews"
      :timezone="props.timezone"
    />
  </UiFloatingPanel>
</template>
