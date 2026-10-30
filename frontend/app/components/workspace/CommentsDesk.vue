<script setup lang="ts">
import type { DisplayedPerson } from '~/types/organization-workspace'
import type { ProposalCommentView } from '~/types/programme/proposal'
import type { PersonId, Uuid } from '~/types/shared'
import type { SelectOption } from '~/types/ui'

export interface CommentsDeskMessage {
  parentId: Uuid | null
  body: string
  topic: string | null
  topicOther: string | null
}

/**
 * LES ÉCHANGES AVEC L'ÉQUIPE, EN FENÊTRE FLOTTANTE (30/10), comme la fiche du
 * back-office mais lue comme une messagerie : un fil chronologique, ses
 * messages à droite, la saisie fixe au pied, Entrée pour envoyer.
 *
 * Ce qui arrive ici est filtré à la SOURCE (visibilité `submitter`) : ni note
 * du comité, ni destinataires, ni accusés de lecture.
 *
 * La demande de correction reste un message à part : étiquetée, bordée de
 * jaune, avec « Marquer comme résolu ». Répondre à un message range la réponse
 * sous son fil ; écrire sans y répondre ouvre un nouveau fil, dont on nomme la
 * nature (30/10) : « Problème technique », « Réclamation »… ou « Autre », précisé.
 */

interface Props {
  open: boolean
  referenceCode: string
  comments: ProposalCommentView[]
  participants: DisplayedPerson[]
  viewerId: PersonId
  loading: boolean
  failed: boolean
  busy?: boolean
  error?: string | null
  /** Rend vrai si le message est parti : le brouillon se vide alors. */
  submit: (message: CommentsDeskMessage) => Promise<boolean>
  markRead: (commentIds: Uuid[]) => Promise<void>
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [value: boolean]
  resolve: [commentId: Uuid, resolved: boolean]
  retry: []
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dateTime } = useDateTime()
const api = useApi()

const { data: topics } = useAsyncData('proposal-comment-topics', () => api.reference.terms('proposal_comment_topic'), {
  server: false,
  lazy: true,
  default: () => [],
})
const topicOptions = computed<SelectOption[]>(() => topics.value.map((term) => ({ value: term.code, label: tr(term.label) })))

/** La nature d'un fil, lue sur sa racine : « Autre » se dit par son intitulé. */
function topicOf(comment: ProposalCommentView): string | null {
  if (comment.parent_id || !comment.topic) return null
  if (comment.topic === 'other' && comment.topic_other) return comment.topic_other
  const term = topics.value.find((candidate) => candidate.code === comment.topic)
  return term ? tr(term.label) : null
}

const viewerTimezone = computed(() => Intl.DateTimeFormat().resolvedOptions().timeZone || 'Europe/Paris')

const isMine = (comment: ProposalCommentView): boolean => comment.author_id === props.viewerId

const unreadCount = computed(() => props.comments.filter((comment) => !comment.is_read && !isMine(comment)).length)

const launcherLabel = computed(() =>
  unreadCount.value > 0
    ? t('organization.workspace.proposal.exchanges.launcherUnread', unreadCount.value)
    : t('organization.workspace.proposal.exchanges.launcher'),
)

const openRequests = computed(
  () => props.comments.filter((comment) => comment.is_change_request && comment.resolved_at === null).length,
)

function authorOf(comment: ProposalCommentView): string {
  if (isMine(comment)) return t('organization.workspace.proposal.exchanges.you')
  const person = props.participants.find((p) => p.id === comment.author_id)
  return person ? `${person.first_name} ${person.last_name}` : t('organization.workspace.proposal.exchanges.team')
}

const byId = computed(() => new Map(props.comments.map((comment) => [comment.id, comment])))

function excerpt(comment: ProposalCommentView): string {
  const flat = comment.body.replace(/\s+/g, ' ').trim()
  return flat.length > 90 ? `${flat.slice(0, 90)}…` : flat
}

// ---------------------------------------------------------------------------
// Lecture : le fil défile vers le bas, et le premier non lu est signalé
// ---------------------------------------------------------------------------

const list = ref<HTMLElement | null>(null)

/** Premier non lu à l'ouverture : figé, sinon la ligne fuit dès que la lecture est posée. */
const firstUnreadId = ref<Uuid | null>(null)

function nearBottom(): boolean {
  const element = list.value
  return !element || element.scrollHeight - element.scrollTop - element.clientHeight < 120
}

async function scrollToEnd(): Promise<void> {
  await nextTick()
  const element = list.value
  if (!element) return
  const marker = firstUnreadId.value ? element.querySelector(`[data-comment-id="${firstUnreadId.value}"]`) : null
  if (marker instanceof HTMLElement && unreadCount.value > 0) element.scrollTop = marker.offsetTop - 16
  else element.scrollTop = element.scrollHeight
}

watch(
  () => props.open,
  (open) => {
    if (!open) return
    firstUnreadId.value = props.comments.find((comment) => !comment.is_read && !isMine(comment))?.id ?? null
    void scrollToEnd()
  },
)

// Un message arrive : on suit le bas du fil si on y était déjà, ou si c'est le sien.
watch(
  () => props.comments.length,
  async (count, before) => {
    if (!props.open || count <= (before ?? 0)) return
    const last = props.comments.at(-1)
    const follow = nearBottom() || (last !== undefined && isMine(last))
    await nextTick()
    if (follow && list.value) list.value.scrollTop = list.value.scrollHeight
  },
)

useReadOnSight({
  root: list,
  targets: () =>
    props.open ? props.comments.filter((comment) => !comment.is_read && !isMine(comment)).map((comment) => comment.id) : [],
  onRead: (ids) => props.markRead(ids),
})

// ---------------------------------------------------------------------------
// Rédaction
// ---------------------------------------------------------------------------

const draft = ref('')
const replyTo = ref<ProposalCommentView | null>(null)
const composer = ref<HTMLElement | null>(null)
const topic = ref<string | null>(null)
const topicOther = ref('')

/** Un nouveau fil part avec sa nature, et « Autre » avec son intitulé. */
const missingTopic = computed(
  () => replyTo.value === null && (topic.value === null || (topic.value === 'other' && topicOther.value.trim() === '')),
)

/** Une réponse se range sous la racine du fil, jamais sous une réponse. */
function answer(comment: ProposalCommentView): void {
  replyTo.value = comment.parent_id ? (byId.value.get(comment.parent_id) ?? comment) : comment
  composer.value?.querySelector('textarea')?.focus()
}

async function send(): Promise<void> {
  const body = draft.value.trim()
  if (body.length === 0 || props.busy) return
  if (missingTopic.value) {
    composer.value?.querySelector<HTMLElement>(topic.value === 'other' ? 'input' : 'select')?.focus()
    return
  }
  const opening = replyTo.value === null
  const sent = await props.submit({
    parentId: replyTo.value?.id ?? null,
    body,
    topic: opening ? topic.value : null,
    topicOther: opening && topic.value === 'other' ? topicOther.value.trim() : null,
  })
  if (sent) {
    draft.value = ''
    replyTo.value = null
    topic.value = null
    topicOther.value = ''
  }
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key !== 'Enter' || event.shiftKey || event.isComposing) return
  event.preventDefault()
  void send()
}
</script>

<template>
  <UiFloatingPanel
    :open="props.open"
    :title="t('organization.workspace.proposal.exchanges.windowTitle', { reference: props.referenceCode })"
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
    <p
      v-if="openRequests > 0"
      class="flex items-start gap-2 border-b border-warning-border bg-warning-surface px-5 py-2.5 text-sm font-semibold text-warning"
    >
      <UiIcon name="warning" size="1.1rem" class="mt-px shrink-0" />
      {{ t('organization.workspace.proposal.exchanges.openRequests', openRequests) }}
    </p>

    <div v-if="props.failed" class="p-5">
      <UiErrorState :retry-label="t('common.actions.retry')" @retry="emit('retry')" />
    </div>

    <div v-else-if="props.loading && props.comments.length === 0" class="flex flex-col gap-3 p-5">
      <UiSkeletonLoader height="4rem" />
      <UiSkeletonLoader height="4rem" />
    </div>

    <div v-else ref="list" class="min-h-0 flex-1 overflow-y-auto px-4 py-4">
      <UiEmptyState
        v-if="props.comments.length === 0"
        icon="chat"
        :title="t('organization.workspace.proposal.exchanges.empty.title')"
        :description="t('organization.workspace.proposal.exchanges.empty.description')"
      />

      <ol v-else class="flex flex-col gap-4">
        <li v-for="comment in props.comments" :key="comment.id" :data-comment-id="comment.id">
          <p
            v-if="comment.id === firstUnreadId"
            class="mb-4 flex items-center gap-3 text-xs font-bold text-accent uppercase"
            :style="{ letterSpacing: 'var(--tracking-caps)' }"
          >
            <span class="h-px flex-1 bg-accent-border" />
            {{ t('organization.workspace.proposal.exchanges.newMessages') }}
            <span class="h-px flex-1 bg-accent-border" />
          </p>

          <article class="flex flex-col gap-1" :class="isMine(comment) ? 'items-end' : 'items-start'">
            <p class="flex flex-wrap items-baseline gap-x-2 px-1 text-xs text-text-subtle">
              <span class="font-semibold text-text-secondary">{{ authorOf(comment) }}</span>
              <time :datetime="comment.created_at">{{ dateTime(comment.created_at, viewerTimezone) }}</time>
              <span v-if="comment.edited_at">· {{ t('organization.workspace.proposal.exchanges.edited') }}</span>
            </p>

            <div
              class="max-w-[85%] rounded-lg border px-3.5 py-2.5"
              :class="[
                comment.is_change_request
                  ? 'border-warning-border bg-surface-raised'
                  : isMine(comment)
                    ? 'border-accent-border bg-accent-surface'
                    : 'border-border bg-surface-raised',
              ]"
            >
              <p
                v-if="topicOf(comment)"
                class="mb-1.5 text-xs font-bold text-text-muted uppercase"
                :style="{ letterSpacing: 'var(--tracking-caps)' }"
              >
                {{ topicOf(comment) }}
              </p>
              <UiBadge
                v-if="comment.is_change_request"
                class="mb-2"
                intent="warning"
                icon="warning"
                size="sm"
                :label="t('organization.workspace.proposal.exchanges.changeRequest')"
              />
              <p
                v-if="comment.parent_id && byId.get(comment.parent_id)"
                class="mb-1.5 border-s-2 border-border-strong ps-2 text-xs text-text-muted"
              >
                {{ excerpt(byId.get(comment.parent_id)!) }}
              </p>
              <p class="text-sm text-text whitespace-pre-line">{{ comment.body }}</p>
            </div>

            <div class="flex flex-wrap items-center gap-x-1 gap-y-1">
              <UiButton v-if="!isMine(comment)" variant="ghost" size="sm" @click="answer(comment)">
                {{ t('organization.workspace.proposal.exchanges.reply') }}
              </UiButton>
              <template v-if="comment.is_change_request">
                <UiButton
                  v-if="comment.resolved_at === null"
                  variant="ghost"
                  size="sm"
                  icon="check"
                  :loading="props.busy"
                  @click="emit('resolve', comment.id, true)"
                >
                  {{ t('organization.workspace.proposal.exchanges.markResolved') }}
                </UiButton>
                <template v-else>
                  <UiBadge
                    intent="success"
                    icon="check"
                    size="sm"
                    :label="t('organization.workspace.proposal.exchanges.resolved', { date: dateTime(comment.resolved_at, viewerTimezone) })"
                  />
                  <UiButton variant="ghost" size="sm" :loading="props.busy" @click="emit('resolve', comment.id, false)">
                    {{ t('organization.workspace.proposal.exchanges.unmarkResolved') }}
                  </UiButton>
                </template>
              </template>
            </div>
          </article>
        </li>
      </ol>
    </div>

    <form
      v-if="!props.failed"
      ref="composer"
      class="flex flex-col gap-2 border-t border-border-subtle px-4 pt-3 pb-4"
      @submit.prevent="send()"
    >
      <p
        v-if="replyTo"
        class="flex items-center gap-2 rounded-md bg-surface-sunken py-1 ps-3 pe-1 text-xs text-text-muted"
      >
        <span class="min-w-0 flex-1 truncate">
          {{ t('organization.workspace.proposal.exchanges.replyingTo', { name: authorOf(replyTo) }) }}
          · {{ excerpt(replyTo) }}
        </span>
        <UiButton
          variant="ghost"
          size="sm"
          icon="close"
          icon-only
          :label="t('organization.workspace.proposal.exchanges.cancel')"
          @click="replyTo = null"
        />
      </p>

      <div v-if="!replyTo" class="flex flex-col gap-2 sm:flex-row">
        <UiSelect
          id="workspace-exchanges-topic"
          :model-value="topic"
          class="sm:flex-1"
          :label="t('organization.workspace.proposal.exchanges.topicLabel')"
          hide-label
          required
          :placeholder="t('organization.workspace.proposal.exchanges.topicPlaceholder')"
          :options="topicOptions"
          @update:model-value="(value: string) => (topic = value)"
        />
        <UiInput
          v-if="topic === 'other'"
          id="workspace-exchanges-topic-other"
          v-model="topicOther"
          class="sm:flex-1"
          :label="t('organization.workspace.proposal.exchanges.topicOtherLabel')"
          hide-label
          required
          :maxlength="80"
          :placeholder="t('organization.workspace.proposal.exchanges.topicOtherPlaceholder')"
        />
      </div>

      <UiAlert v-if="props.error" intent="danger" :message="props.error" live />

      <div class="flex items-end gap-2">
        <UiTextarea
          id="workspace-exchanges-draft"
          v-model="draft"
          class="min-w-0 flex-1"
          :label="t('organization.workspace.proposal.exchanges.messageLabel')"
          hide-label
          :rows="1"
          auto-grow
          :max-rows="6"
          :placeholder="t('organization.workspace.proposal.exchanges.messagePlaceholder')"
          @keydown="onKeydown"
        />
        <UiButton
          type="submit"
          icon="arrow-right"
          icon-only
          :label="t('organization.workspace.proposal.exchanges.send')"
          :loading="props.busy"
          :disabled="draft.trim().length === 0 || missingTopic"
        />
      </div>
      <p class="text-xs text-text-subtle max-sm:hidden">{{ t('organization.workspace.proposal.exchanges.sendHint') }}</p>
    </form>
  </UiFloatingPanel>
</template>
