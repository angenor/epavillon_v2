<script setup lang="ts">
import type { CommentVisibility, ProposalCommentView } from '~/types/programme/proposal'
import type { TimeZoneName } from '~/types/shared'

/**
 * UN MESSAGE DU FIL : son auteur, sa date, son état lu, et sous les siens, qui
 * l'a lu. Le point cyan dit « pas encore lu » ; il s'efface quand le message a
 * été affiché, ou revient par « Marquer comme non lu ».
 */

interface Props {
  comment: ProposalCommentView
  authorName: string
  isOwn: boolean
  /** La racine porte la visibilité, la demande de correction et les destinataires. */
  isRoot: boolean
  timezone: TimeZoneName
  zoneLabel: string
  disabled?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ unread: [] }>()

const { t } = useI18n()
const { dateTime } = useDateTime()

const BADGE_INTENT: Record<CommentVisibility, 'info' | 'neutral'> = {
  submitter: 'info',
  committee: 'neutral',
  private: 'neutral',
}

const recipientNames = computed(() =>
  props.comment.recipients.map((person) => person.display_name).join(', '),
)
</script>

<template>
  <article :data-comment-id="props.comment.id" :class="props.isRoot ? 'p-4' : 'px-4 py-3 sm:pl-9'">
    <header class="flex flex-wrap items-center gap-x-3 gap-y-1">
      <span
        class="size-2 shrink-0 rounded-full bg-info-solid transition-opacity duration-(--duration-base)"
        :class="props.comment.is_read ? 'opacity-0' : 'opacity-100'"
        aria-hidden="true"
      />
      <span v-if="!props.comment.is_read" class="sr-only">
        {{ t('admin.proposal.review.comments.read.unread') }}
      </span>
      <span class="font-semibold text-text" :class="props.comment.is_read ? '' : 'text-heading'">
        {{ props.authorName }}
      </span>
      <time :datetime="props.comment.created_at" class="text-sm text-text-subtle">
        {{ dateTime(props.comment.created_at, props.timezone) }}
      </time>
      <span v-if="props.comment.edited_at" class="text-xs text-text-subtle">
        · {{ t('admin.proposal.review.comments.edited') }}
      </span>

      <template v-if="props.isRoot">
        <!-- Le libellé de visibilité, en toutes lettres sur chaque fil : c'est
             ce qui empêche de lire un fil interne comme une discussion avec
             l'organisation. -->
        <UiBadge
          size="sm"
          :intent="BADGE_INTENT[props.comment.visibility]"
          :icon="props.comment.visibility === 'private' ? 'eye-off' : 'users'"
          :label="t(`admin.proposal.review.comments.visibility.${props.comment.visibility}.label`)"
        />
        <UiBadge
          v-if="props.comment.is_change_request"
          size="sm"
          intent="warning"
          icon="warning"
          :label="t('admin.proposal.review.comments.changeRequest')"
        />
      </template>
    </header>

    <p v-if="props.isRoot && props.comment.recipients.length > 0" class="mt-2 text-sm text-text-muted">
      <span class="font-semibold text-text">{{ t('admin.proposal.review.comments.recipients.to') }}</span>
      {{ recipientNames }}
    </p>

    <p
      class="max-w-(--measure) whitespace-pre-line text-text-secondary"
      :class="props.isRoot ? 'mt-3' : 'mt-2'"
    >
      {{ props.comment.body }}
    </p>

    <p v-if="props.isRoot && props.comment.is_change_request" class="mt-2 text-sm">
      <span v-if="props.comment.resolved_at" class="text-success">
        {{
          t('admin.proposal.review.comments.resolved', {
            date: dateTime(props.comment.resolved_at, props.timezone),
          })
        }}
      </span>
      <span v-else class="text-warning">
        {{ t('admin.proposal.review.comments.unresolved') }}
      </span>
    </p>

    <AdminReviewCommentReceipts
      v-if="props.isOwn"
      :receipts="props.comment.receipts"
      :visibility="props.comment.visibility"
      :timezone="props.timezone"
      :zone-label="props.zoneLabel"
    />

    <UiButton
      v-else-if="props.comment.is_read"
      class="mt-1"
      variant="ghost"
      icon="eye-off"
      :disabled="props.disabled"
      @click="emit('unread')"
    >
      {{ t('admin.proposal.review.comments.read.markUnread') }}
    </UiButton>
  </article>
</template>
