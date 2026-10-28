<script setup lang="ts">
import type { PostCommentPayload } from '~/types/admin-review'
import type { Person } from '~/types/identity'
import type {
  CommentPerson,
  CommentVisibility,
  ProposalCommentView,
} from '~/types/programme/proposal'
import type { PersonId, TimeZoneName, Uuid } from '~/types/shared'

/**
 * LES ÉCHANGES AUTOUR DU DOSSIER, et la distinction qui coûte le plus cher de
 * tout cet écran : « interne au comité » contre « partagé avec le
 * soumissionnaire ». Le fond de chaque fil dit son destinataire (bleuté et bordé
 * pour ce qui sort de l'IFDD, creusé pour le comité, neutre pour une note
 * personnelle), le libellé l'écrit en toutes lettres, et la rédaction
 * (`CommentComposer`) part du choix le plus sûr.
 *
 * Un fil du comité peut être RESTREINT à quelques personnes : il affiche « À :
 * … », ses réponses restent dans ce cercle, et « Ajouter des destinataires »
 * l'élargit (la personne ajoutée lit tout le fil).
 *
 * LU / NON LU : un message non lu porte un point ; il devient lu une fois
 * affiché (`useReadOnSight`), sans clic.
 */

interface Props {
  comments: ProposalCommentView[]
  participants: Person[]
  /** Membres du comité de ce dossier ; la personne connectée en est retirée ici. */
  audience: CommentPerson[]
  viewerId: PersonId | null
  /** L'organisation porteuse, nommée dans la confirmation d'envoi partagé. */
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
const emit = defineEmits<{ unread: [commentId: Uuid] }>()

const { t } = useI18n()

const root = ref<HTMLElement | null>(null)

const threads = computed(() => groupThreads(props.comments))
const others = computed(() => props.audience.filter((person) => person.person_id !== props.viewerId))

function nameOf(personId: PersonId): string {
  if (personId === props.viewerId) return t('admin.proposal.review.comments.you')
  return (
    props.participants.find((entry) => entry.id === personId)?.display_name ??
    props.audience.find((entry) => entry.person_id === personId)?.display_name ??
    ''
  )
}

const SURFACE: Record<CommentVisibility, string> = {
  submitter: 'border-info-border bg-info-surface',
  committee: 'border-border bg-surface-sunken',
  private: 'border-dashed border-border bg-surface',
}

// ---------------------------------------------------------------------------
// Lu / non lu
// ---------------------------------------------------------------------------

const tracker = useReadOnSight({
  root,
  targets: () => props.comments.filter((comment) => !comment.is_read).map((comment) => comment.id),
  onRead: (ids) => props.markRead(ids),
})

function markUnread(commentId: Uuid): void {
  tracker.suppress(commentId)
  emit('unread', commentId)
}

const unreadCount = computed(() => props.comments.filter((comment) => !comment.is_read).length)

// ---------------------------------------------------------------------------
// Répondre, élargir un cercle
// ---------------------------------------------------------------------------

const replyingTo = ref<ProposalCommentView | null>(null)

function circleOf(thread: ProposalCommentView): string | null {
  if (thread.recipients.length === 0) return null
  const ids = [...new Set([thread.author_id, ...thread.recipients.map((person) => person.person_id)])]
  return ids.map(nameOf).filter(Boolean).join(', ')
}

const widening = ref<ProposalCommentView | null>(null)
const added = ref<PersonId[]>([])

const wideningAudience = computed(() => {
  const thread = widening.value
  if (!thread) return []
  const circle = new Set([thread.author_id, ...thread.recipients.map((person) => person.person_id)])
  return others.value.filter((person) => !circle.has(person.person_id))
})

function openWidening(thread: ProposalCommentView): void {
  widening.value = thread
  added.value = []
}

async function confirmWidening(): Promise<void> {
  const thread = widening.value
  if (!thread || added.value.length === 0) return
  if (await props.addRecipients(thread.id, added.value)) widening.value = null
}
</script>

<template>
  <section
    ref="root"
    class="rounded-lg border border-border bg-surface-raised"
    aria-labelledby="review-comments-title"
  >
    <header class="border-b border-border-subtle px-5 py-4">
      <h2 id="review-comments-title" class="flex flex-wrap items-center gap-3 text-lg font-semibold">
        {{ t('admin.proposal.review.comments.title') }}
        <UiBadge
          v-if="unreadCount > 0"
          size="sm"
          intent="info"
          :label="t('admin.proposal.review.comments.read.newCount', unreadCount)"
        />
      </h2>
      <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
        {{ t('admin.proposal.review.comments.description') }}
      </p>
    </header>

    <div class="flex flex-col gap-4 p-5">
      <UiAlert v-if="props.error && !widening" intent="danger" live :message="props.error" />

      <p v-if="threads.length === 0" class="text-sm text-text-muted">
        {{ t('admin.proposal.review.comments.empty') }}
      </p>

      <ol v-else class="flex flex-col gap-4">
        <li
          v-for="thread in threads"
          :key="thread.root.id"
          class="rounded-md border"
          :class="SURFACE[thread.root.visibility]"
        >
          <AdminReviewCommentMessage
            :comment="thread.root"
            :author-name="nameOf(thread.root.author_id)"
            :is-own="thread.root.author_id === props.viewerId"
            is-root
            :timezone="props.timezone"
            :zone-label="props.zoneLabel"
            :disabled="props.busy"
            @unread="markUnread(thread.root.id)"
          />

          <ol v-if="thread.replies.length > 0" class="flex flex-col border-t border-border-subtle">
            <li
              v-for="reply in thread.replies"
              :key="reply.id"
              class="border-b border-border-subtle last:border-b-0"
            >
              <AdminReviewCommentMessage
                :comment="reply"
                :author-name="nameOf(reply.author_id)"
                :is-own="reply.author_id === props.viewerId"
                :is-root="false"
                :timezone="props.timezone"
                :zone-label="props.zoneLabel"
                :disabled="props.busy"
                @unread="markUnread(reply.id)"
              />
            </li>
          </ol>

          <footer
            v-if="props.canWrite"
            class="flex flex-wrap items-center gap-2 border-t border-border-subtle px-4 py-2"
          >
            <UiButton
              v-if="replyingTo?.id !== thread.root.id"
              variant="ghost"
              icon="mail"
              @click="replyingTo = thread.root"
            >
              {{ t('admin.proposal.review.comments.compose.reply') }}
            </UiButton>
            <p v-else class="py-1 text-sm text-text-muted">
              {{ t('admin.proposal.review.comments.compose.label') }} ↓
            </p>
            <UiButton
              v-if="thread.root.recipients.length > 0"
              variant="ghost"
              icon="users"
              :disabled="props.busy"
              @click="openWidening(thread.root)"
            >
              {{ t('admin.proposal.review.comments.recipients.add') }}
            </UiButton>
          </footer>
        </li>
      </ol>

      <AdminReviewCommentComposer
        v-if="props.canWrite"
        :reply-to="replyingTo"
        :reply-circle="replyingTo ? circleOf(replyingTo) : null"
        :audience="others"
        :lead-organization-name="props.leadOrganizationName"
        :submit="props.submit"
        :busy="props.busy"
        @cancel="replyingTo = null"
      />
    </div>

    <UiModal
      :open="widening !== null"
      :title="t('admin.proposal.review.comments.recipients.addTitle')"
      :description="t('admin.proposal.review.comments.recipients.addDescription')"
      size="md"
      @update:open="(value: boolean) => { if (!value) widening = null }"
    >
      <div class="space-y-4">
        <UiAlert v-if="props.error" intent="danger" live :message="props.error" />
        <p v-if="widening" class="text-sm text-text-muted">
          {{ t('admin.proposal.review.comments.recipients.current', { names: circleOf(widening) }) }}
        </p>
        <AdminReviewCommentRecipients
          v-model="added"
          :audience="wideningAudience"
          :label="t('admin.proposal.review.comments.recipients.toAdd')"
          :search-label="t('admin.proposal.review.comments.recipients.search')"
          hide-everyone
          :disabled="props.busy"
        />
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="widening = null">
          {{ t('admin.proposal.review.comments.compose.cancel') }}
        </UiButton>
        <UiButton
          variant="primary"
          :disabled="added.length === 0"
          :loading="props.busy"
          @click="confirmWidening()"
        >
          {{ t('admin.proposal.review.comments.recipients.addSubmit') }}
        </UiButton>
      </template>
    </UiModal>
  </section>
</template>
