<script setup lang="ts">
import type { PostCommentPayload } from '~/types/admin-review'
import type { Person } from '~/types/identity'
import type { CommentPerson, ProposalCommentView } from '~/types/programme/proposal'
import type { PersonId, TimeZoneName, Uuid } from '~/types/shared'

/**
 * Les échanges d'un dossier, dans une fenêtre flottante (comme en v1, demande du
 * 29/10) : le bouton porte le nombre de messages non lus, la fenêtre se lit à
 * côté du dossier. Plein écran sur téléphone.
 *
 * Les messages ne passent « lus » qu'une fois la fenêtre ouverte et le message
 * affiché : `Comments` n'est monté qu'à l'ouverture.
 */

interface Props {
  open: boolean
  stack?: number
  referenceCode: string
  comments: ProposalCommentView[]
  loading: boolean
  failed: boolean
  participants: Person[]
  audience: CommentPerson[]
  viewerId: PersonId | null
  leadOrganizationName: string
  timezone: TimeZoneName
  zoneLabel: string
  canWrite: boolean
  busy?: boolean
  error?: string | null
  submit: (payload: Omit<PostCommentPayload, 'proposal_id'>) => Promise<boolean>
  markRead: (commentIds: Uuid[]) => Promise<void>
  addRecipients: (rootId: Uuid, personIds: PersonId[]) => Promise<boolean>
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [value: boolean]
  unread: [commentId: Uuid]
  retry: []
}>()

const { t } = useI18n()

const unreadCount = computed(() => props.comments.filter((comment) => !comment.is_read).length)

const launcherLabel = computed(() =>
  unreadCount.value > 0
    ? t('admin.proposal.review.comments.launcherUnread', unreadCount.value)
    : t('admin.proposal.review.comments.launcher'),
)
</script>

<template>
  <UiFloatingPanel
    :open="props.open"
    :stack="props.stack"
    :title="t('admin.proposal.review.comments.windowTitle', { reference: props.referenceCode })"
    :launcher-label="launcherLabel"
    launcher-icon="chat"
    :launcher-badge="unreadCount"
    width="30rem"
    height="min(44rem, calc(100dvh - 3rem))"
    expandable
    fullscreen-on-mobile
    :scroll-body="false"
    @update:open="(value: boolean) => emit('update:open', value)"
  >
    <div v-if="props.failed" class="p-5">
      <UiErrorState :retry-label="t('common.actions.retry')" @retry="emit('retry')" />
    </div>
    <div v-else-if="props.loading && props.comments.length === 0" class="flex flex-col gap-3 p-5">
      <UiSkeletonLoader height="6rem" />
      <UiSkeletonLoader height="6rem" />
    </div>
    <AdminReviewComments
      v-else
      :comments="props.comments"
      :participants="props.participants"
      :audience="props.audience"
      :viewer-id="props.viewerId"
      :lead-organization-name="props.leadOrganizationName"
      :timezone="props.timezone"
      :zone-label="props.zoneLabel"
      :can-write="props.canWrite"
      :busy="props.busy"
      :error="props.error"
      :submit="props.submit"
      :mark-read="props.markRead"
      :add-recipients="props.addRecipients"
      @unread="(id: Uuid) => emit('unread', id)"
    />
  </UiFloatingPanel>
</template>
