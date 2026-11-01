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
  /** Titre du sujet, quand le message en ouvre un. */
  subject: string | null
}

/**
 * LES ÉCHANGES AVEC L'ÉQUIPE, PAR SUJET (01/11), en fenêtre flottante : comme
 * des tickets d'assistance. La liste des sujets (ouverts, résolus), un nouveau
 * sujet avec son titre et sa nature, un sujet ouvert avec ses messages et sa
 * réponse. La nature se choisit une fois, à l'ouverture ; les réponses en
 * héritent. Écrire dans un sujet résolu le rouvre (la base s'en charge).
 *
 * Ce qui arrive ici est filtré à la SOURCE (visibilité `submitter`) : ni note
 * du comité, ni destinataires, ni accusés de lecture. La demande de correction
 * reste un message à part, étiquetée, avec « Marquer comme résolu ».
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
  /** Rend l'identifiant du message parti, nul sinon : le brouillon se vide alors. */
  submit: (message: CommentsDeskMessage) => Promise<Uuid | null>
  markRead: (commentIds: Uuid[]) => Promise<void>
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [value: boolean]
  resolve: [commentId: Uuid, resolved: boolean]
  closeSubject: [rootId: Uuid, closed: boolean]
  retry: []
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dateTime } = useDateTime()
const api = useApi()

const k = (key: string) => `organization.workspace.proposal.exchanges.${key}`

const { data: topics } = useAsyncData('proposal-comment-topics', () => api.reference.terms('proposal_comment_topic'), {
  server: false,
  lazy: true,
  default: () => [],
})
const topicOptions = computed<SelectOption[]>(() => topics.value.map((term) => ({ value: term.code, label: tr(term.label) })))

function topicOf(root: ProposalCommentView): string | null {
  if (!root.topic) return null
  if (root.topic === 'other' && root.topic_other) return root.topic_other
  const term = topics.value.find((candidate) => candidate.code === root.topic)
  return term ? tr(term.label) : null
}

const viewerTimezone = computed(() => Intl.DateTimeFormat().resolvedOptions().timeZone || 'Europe/Paris')
const isMine = (comment: ProposalCommentView): boolean => comment.author_id === props.viewerId
const isUnread = (comment: ProposalCommentView): boolean => !comment.is_read && !isMine(comment)

function authorOf(comment: ProposalCommentView): string {
  if (isMine(comment)) return t(k('you'))
  const person = props.participants.find((p) => p.id === comment.author_id)
  return person ? `${person.first_name} ${person.last_name}` : t(k('team'))
}

function excerpt(comment: ProposalCommentView, length = 90): string {
  const flat = comment.body.replace(/\s+/g, ' ').trim()
  return flat.length > length ? `${flat.slice(0, length)}…` : flat
}

// ---------------------------------------------------------------------------
// Les sujets
// ---------------------------------------------------------------------------

type SubjectState = 'waiting' | 'answered' | 'resolved'
const STATE_INTENT = { waiting: 'warning', answered: 'info', resolved: 'success' } as const

const subjects = computed(() =>
  groupThreads(props.comments)
    .map(({ root, replies }) => {
      const messages = [root, ...replies]
      const last = messages.at(-1) ?? root
      const state: SubjectState = root.closed_at ? 'resolved' : isMine(last) ? 'waiting' : 'answered'
      return {
        root,
        messages,
        last,
        state,
        // Un fil ouvert avant les sujets n'a pas de titre : sa nature, sinon son début.
        title: root.subject ?? topicOf(root) ?? excerpt(root, 60),
        nature: topicOf(root),
        unread: messages.filter(isUnread).length,
      }
    })
    .sort((a, b) => b.last.created_at.localeCompare(a.last.created_at)),
)

const unreadCount = computed(() => props.comments.filter(isUnread).length)
const launcherLabel = computed(() => (unreadCount.value > 0 ? t(k('launcherUnread'), unreadCount.value) : t(k('launcher'))))
const openRequests = computed(() => props.comments.filter((c) => c.is_change_request && c.resolved_at === null).length)

const filter = ref<'open' | 'resolved'>('open')
const shown = computed(() => subjects.value.filter((s) => (filter.value === 'resolved') === (s.state === 'resolved')))
const openCount = computed(() => subjects.value.filter((s) => s.state !== 'resolved').length)

type View = 'list' | 'new' | 'subject'
const view = ref<View>('list')
const currentId = ref<Uuid | null>(null)
const current = computed(() => subjects.value.find((s) => s.root.id === currentId.value) ?? null)

function openSubject(rootId: Uuid): void {
  currentId.value = rootId
  view.value = 'subject'
  draft.value = ''
  void scrollToEnd()
}

function backToList(): void {
  view.value = 'list'
  currentId.value = null
}

watch(
  () => props.open,
  (open) => {
    if (open && view.value === 'subject' && !current.value) backToList()
  },
)

// ---------------------------------------------------------------------------
// Lecture d'un sujet : il défile vers le bas, et ce qu'on y voit est lu
// ---------------------------------------------------------------------------

const list = ref<HTMLElement | null>(null)

async function scrollToEnd(): Promise<void> {
  await nextTick()
  if (list.value) list.value.scrollTop = list.value.scrollHeight
}

watch(
  () => current.value?.messages.length,
  (count, before) => {
    if (view.value === 'subject' && count && count > (before ?? 0)) void scrollToEnd()
  },
)

useReadOnSight({
  root: list,
  targets: () =>
    props.open && view.value === 'subject' && current.value
      ? current.value.messages.filter(isUnread).map((comment) => comment.id)
      : [],
  onRead: (ids) => props.markRead(ids),
})

// ---------------------------------------------------------------------------
// Rédaction
// ---------------------------------------------------------------------------

const draft = ref('')
const subjectTitle = ref('')
const topic = ref<string | null>(null)
const topicOther = ref('')

const newSubjectReady = computed(
  () =>
    subjectTitle.value.trim().length >= 2 &&
    topic.value !== null &&
    (topic.value !== 'other' || topicOther.value.trim() !== '') &&
    draft.value.trim().length > 0,
)

function startNew(): void {
  view.value = 'new'
  draft.value = ''
  subjectTitle.value = ''
  topic.value = null
  topicOther.value = ''
}

async function createSubject(): Promise<void> {
  if (!newSubjectReady.value || props.busy) return
  const created = await props.submit({
    parentId: null,
    body: draft.value.trim(),
    topic: topic.value,
    topicOther: topic.value === 'other' ? topicOther.value.trim() : null,
    subject: subjectTitle.value.trim(),
  })
  if (created) openSubject(created)
}

async function reply(): Promise<void> {
  const body = draft.value.trim()
  if (!current.value || body.length === 0 || props.busy) return
  const sent = await props.submit({ parentId: current.value.root.id, body, topic: null, topicOther: null, subject: null })
  if (sent) draft.value = ''
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key !== 'Enter' || event.shiftKey || event.isComposing) return
  event.preventDefault()
  void reply()
}
</script>

<template>
  <UiFloatingPanel
    :open="props.open"
    :title="t(k('windowTitle'), { reference: props.referenceCode })"
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
      {{ t(k('openRequests'), openRequests) }}
    </p>

    <div v-if="props.failed" class="p-5">
      <UiErrorState :retry-label="t('common.actions.retry')" @retry="emit('retry')" />
    </div>

    <div v-else-if="props.loading && props.comments.length === 0" class="flex flex-col gap-3 p-5">
      <UiSkeletonLoader height="4rem" />
      <UiSkeletonLoader height="4rem" />
    </div>

    <!-- LA LISTE DES SUJETS -->
    <div v-else-if="view === 'list'" class="flex min-h-0 flex-1 flex-col">
      <div class="flex flex-wrap items-center justify-between gap-3 px-4 pt-4 pb-3">
        <div role="tablist" :aria-label="t(k('filterLabel'))" class="flex gap-1 rounded-lg bg-surface-sunken p-1">
          <button
            v-for="key in ['open', 'resolved'] as const"
            :key="key"
            type="button"
            role="tab"
            :aria-selected="filter === key"
            class="min-h-9 cursor-pointer rounded-md px-3 text-sm font-semibold"
            :class="filter === key ? 'bg-surface-raised text-text shadow-sm' : 'text-text-secondary'"
            @click="filter = key"
          >
            {{ t(k(`filter.${key}`), { count: key === 'open' ? openCount : subjects.length - openCount }) }}
          </button>
        </div>
        <UiButton variant="primary" icon="plus" @click="startNew">{{ t(k('newSubject')) }}</UiButton>
      </div>

      <div class="min-h-0 flex-1 overflow-y-auto px-4 pb-4">
        <UiEmptyState
          v-if="shown.length === 0"
          icon="chat"
          compact
          :title="t(k(filter === 'open' ? 'emptyOpen.title' : 'emptyResolved.title'))"
          :description="filter === 'open' ? t(k('emptyOpen.description')) : undefined"
        />
        <ul v-else class="flex flex-col gap-2.5">
          <li v-for="subject in shown" :key="subject.root.id">
            <button
              type="button"
              class="flex w-full cursor-pointer flex-col gap-1.5 rounded-lg border border-border bg-surface-raised px-4 py-3 text-left hover:border-border-strong"
              @click="openSubject(subject.root.id)"
            >
              <span class="flex items-start gap-2">
                <span class="flex-1 font-semibold text-text">{{ subject.title }}</span>
                <span
                  v-if="subject.unread > 0"
                  class="grid h-5.5 min-w-5.5 place-items-center rounded-full bg-danger-solid px-1.5 text-xs font-bold text-text-on-inverse"
                  :aria-label="t(k('unreadInSubject'), subject.unread)"
                >
                  {{ subject.unread }}
                </span>
              </span>
              <span class="flex flex-wrap gap-1.5">
                <UiBadge v-if="subject.nature" size="sm" :label="subject.nature" />
                <UiBadge size="sm" :intent="STATE_INTENT[subject.state]" :label="t(k(`state.${subject.state}`))" />
              </span>
              <span class="truncate text-sm text-text-muted">{{ authorOf(subject.last) }} : {{ excerpt(subject.last) }}</span>
            </button>
          </li>
        </ul>
      </div>
    </div>

    <!-- NOUVEAU SUJET -->
    <form v-else-if="view === 'new'" class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-4 py-4" @submit.prevent="createSubject">
      <div class="flex items-center gap-2">
        <UiButton variant="ghost" size="sm" icon="arrow-left" icon-only :label="t(k('backToSubjects'))" @click="backToList" />
        <h3 class="text-base font-semibold">{{ t(k('newSubject')) }}</h3>
      </div>
      <p class="-mt-2 text-sm text-text-muted">{{ t(k('newSubjectHint')) }}</p>
      <UiInput
        id="workspace-exchanges-subject"
        v-model="subjectTitle"
        :label="t(k('subjectLabel'))"
        :placeholder="t(k('subjectPlaceholder'))"
        :maxlength="120"
        required
      />
      <UiSelect
        id="workspace-exchanges-topic"
        :model-value="topic"
        :label="t(k('topicLabel'))"
        required
        :placeholder="t(k('topicPlaceholder'))"
        :options="topicOptions"
        @update:model-value="(value: string) => (topic = value)"
      />
      <UiInput
        v-if="topic === 'other'"
        id="workspace-exchanges-topic-other"
        v-model="topicOther"
        :label="t(k('topicOtherLabel'))"
        required
        :maxlength="80"
        :placeholder="t(k('topicOtherPlaceholder'))"
      />
      <UiTextarea
        id="workspace-exchanges-first"
        v-model="draft"
        :label="t(k('messageLabel'))"
        :rows="5"
        required
        :placeholder="t(k('messagePlaceholder'))"
      />
      <UiAlert v-if="props.error" intent="danger" :message="props.error" live />
      <div class="flex justify-end gap-2">
        <UiButton variant="secondary" @click="backToList">{{ t(k('cancel')) }}</UiButton>
        <UiButton type="submit" variant="primary" :loading="props.busy" :disabled="!newSubjectReady">
          {{ t(k('openSubject')) }}
        </UiButton>
      </div>
    </form>

    <!-- UN SUJET -->
    <template v-else-if="current">
      <div class="flex flex-col gap-2 border-b border-border-subtle px-4 pt-3 pb-3">
        <div class="flex items-start gap-2">
          <UiButton variant="ghost" size="sm" icon="arrow-left" icon-only :label="t(k('backToSubjects'))" @click="backToList" />
          <h3 class="min-w-0 flex-1 pt-1 text-base font-semibold">{{ current.title }}</h3>
        </div>
        <div class="flex flex-wrap items-center gap-1.5 ps-11">
          <UiBadge v-if="current.nature" size="sm" :label="current.nature" />
          <UiBadge size="sm" :intent="STATE_INTENT[current.state]" :label="t(k(`state.${current.state}`))" />
          <span class="flex-1" />
          <UiButton
            variant="secondary"
            size="sm"
            :icon="current.state === 'resolved' ? undefined : 'check'"
            :loading="props.busy"
            @click="emit('closeSubject', current.root.id, current.state !== 'resolved')"
          >
            {{ t(k(current.state === 'resolved' ? 'reopenSubject' : 'markSubjectResolved')) }}
          </UiButton>
        </div>
      </div>

      <div ref="list" class="min-h-0 flex-1 overflow-y-auto px-4 py-4">
        <ol class="flex flex-col gap-4">
          <li v-for="comment in current.messages" :key="comment.id" :data-comment-id="comment.id">
            <article class="flex flex-col gap-1" :class="isMine(comment) ? 'items-end' : 'items-start'">
              <p class="flex flex-wrap items-baseline gap-x-2 px-1 text-xs text-text-subtle">
                <span class="font-semibold text-text-secondary">{{ authorOf(comment) }}</span>
                <time :datetime="comment.created_at">{{ dateTime(comment.created_at, viewerTimezone) }}</time>
                <span v-if="comment.edited_at">· {{ t(k('edited')) }}</span>
              </p>
              <div
                class="max-w-[85%] rounded-lg border px-3.5 py-2.5"
                :class="
                  comment.is_change_request
                    ? 'border-warning-border bg-surface-raised'
                    : isMine(comment)
                      ? 'border-accent-border bg-accent-surface'
                      : 'border-border bg-surface-raised'
                "
              >
                <UiBadge
                  v-if="comment.is_change_request"
                  class="mb-2"
                  intent="warning"
                  icon="warning"
                  size="sm"
                  :label="t(k('changeRequest'))"
                />
                <p class="text-sm whitespace-pre-line text-text">{{ comment.body }}</p>
              </div>
              <div v-if="comment.is_change_request" class="flex flex-wrap items-center gap-1">
                <UiButton
                  v-if="comment.resolved_at === null"
                  variant="ghost"
                  size="sm"
                  icon="check"
                  :loading="props.busy"
                  @click="emit('resolve', comment.id, true)"
                >
                  {{ t(k('markResolved')) }}
                </UiButton>
                <template v-else>
                  <UiBadge
                    intent="success"
                    icon="check"
                    size="sm"
                    :label="t(k('resolved'), { date: dateTime(comment.resolved_at, viewerTimezone) })"
                  />
                  <UiButton variant="ghost" size="sm" :loading="props.busy" @click="emit('resolve', comment.id, false)">
                    {{ t(k('unmarkResolved')) }}
                  </UiButton>
                </template>
              </div>
            </article>
          </li>
        </ol>
      </div>

      <p v-if="current.state === 'resolved'" class="mx-4 rounded-md bg-success-surface px-3 py-2 text-sm text-success">
        {{ t(k('resolvedNotice')) }}
      </p>
      <form class="flex flex-col gap-2 border-t border-border-subtle px-4 pt-3 pb-4" @submit.prevent="reply">
        <UiAlert v-if="props.error" intent="danger" :message="props.error" live />
        <div class="flex items-end gap-2">
          <UiTextarea
            id="workspace-exchanges-draft"
            v-model="draft"
            class="min-w-0 flex-1"
            :label="t(k('replyLabel'))"
            hide-label
            :rows="1"
            auto-grow
            :max-rows="6"
            :placeholder="t(k('replyPlaceholder'))"
            @keydown="onKeydown"
          />
          <UiButton
            type="submit"
            icon="arrow-right"
            icon-only
            :label="t(k('send'))"
            :loading="props.busy"
            :disabled="draft.trim().length === 0"
          />
        </div>
        <p class="text-xs text-text-subtle max-sm:hidden">{{ t(k('sendHint')) }}</p>
      </form>
    </template>
  </UiFloatingPanel>
</template>
