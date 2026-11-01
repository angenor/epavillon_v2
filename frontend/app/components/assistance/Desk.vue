<script setup lang="ts">
import type { AssistanceFeed, AssistanceMessage, AssistanceSide } from '~/types/programme/assistance'
import type { ProposalId, Uuid } from '~/types/shared'
import type { SelectOption } from '~/types/ui'

/**
 * L'ASSISTANCE, SUR CHAQUE ÉCRAN (02/11) : posée par les gabarits public et
 * back-office, visible de l'équipe (`programme.support.answer`) et des
 * organisations qui suivent un dossier déposé. Les sujets sont ceux des
 * échanges d'un dossier, plus ceux qui ne concernent aucun dossier (une
 * question sur la plateforme, une suggestion).
 *
 * Côté organisation, le bouton dit « Aide » ; côté équipe, une icône et ses
 * non-lus. L'écran d'un dossier se signale par `useAssistance().attach()` : un
 * nouveau sujet le propose d'office.
 */

const { t } = useI18n()
const { tr } = useI18nText()
const { dateTime } = useDateTime()
const localePath = useLocalePath()
const api = useApi()
const auth = useAuthStore()
const toast = useToast()
const assistance = useAssistance()

const k = (key: string) => `assistance-desk.${key}`
const viewerTimezone = computed(() => Intl.DateTimeFormat().resolvedOptions().timeZone || 'Europe/Paris')
const personId = computed(() => auth.person?.id ?? null)

const { data: status, refresh: refreshStatus } = useAsyncData(
  'assistance-status',
  () => (personId.value ? api.assistance.status(personId.value) : Promise.resolve(null)),
  { server: false, lazy: true, watch: [personId], default: () => null },
)
const side = computed<AssistanceSide | null>(() => status.value?.side ?? null)

const {
  data: feed,
  status: feedStatus,
  error: feedError,
  refresh: refreshFeed,
} = useAsyncData<AssistanceFeed | null>(
  'assistance-feed',
  () => (personId.value && side.value ? api.assistance.feed(personId.value) : Promise.resolve(null)),
  { server: false, lazy: true, watch: [side, personId], default: () => null },
)
const messages = computed<AssistanceMessage[]>(() => feed.value?.messages ?? [])
const isTeam = computed(() => side.value === 'team')

/** Relit le fil ; fenêtre fermée, un message arrivé se signale en bref. */
async function reread(): Promise<void> {
  const known = new Set(messages.value.map((m) => m.id))
  await refreshFeed()
  if (assistance.open.value || document.visibilityState !== 'visible') return
  const arrived = messages.value.some((m) => !known.has(m.id) && !m.is_read && m.author_id !== personId.value)
  if (arrived && known.size > 0) toast.info(t(k('arrived.title')), { message: t(k('arrived.message')) })
}

useRealtime(
  () => {
    if (!side.value || !personId.value) return []
    if (isTeam.value) return ['assistance:team']
    return [`person:${personId.value}`, ...(feed.value?.organization_ids ?? []).map((id) => `organization:${id}`)]
  },
  () => reread(),
)

// ---------------------------------------------------------------------------
// Les natures, des dossiers et de la plateforme
// ---------------------------------------------------------------------------

const termsOptions = { server: false, lazy: true, default: () => [] }
const { data: fileTopics } = useAsyncData('proposal-comment-topics', () => api.reference.terms('proposal_comment_topic'), termsOptions)
const { data: platformTopics } = useAsyncData('platform-support-topics', () => api.reference.terms('platform_support_topic'), termsOptions)

const SUGGESTION = 'suggestion'

function natureOf(root: AssistanceMessage): string | null {
  if (!root.topic || (!root.proposal_id && root.topic === SUGGESTION)) return null
  if (root.topic === 'other' && root.topic_other) return root.topic_other
  const terms = root.proposal_id ? fileTopics.value : platformTopics.value
  const term = terms.find((candidate) => candidate.code === root.topic)
  return term ? tr(term.label) : null
}

// ---------------------------------------------------------------------------
// Les sujets
// ---------------------------------------------------------------------------

type About = 'file' | 'platform' | 'idea'
type SubjectState = 'waiting' | 'answered' | 'resolved'

const aboutOf = (root: AssistanceMessage): About =>
  root.proposal_id ? 'file' : root.topic === SUGGESTION ? 'idea' : 'platform'
const isMine = (m: AssistanceMessage) => m.author_id === personId.value
const isUnread = (m: AssistanceMessage) => !m.is_read && !isMine(m)
/** Côté équipe, « en attente » veut dire : l'organisation attend l'équipe. */
const fromAsker = (m: AssistanceMessage) => !m.from_team

function excerpt(text: string, length = 90): string {
  const flat = text.replace(/\s+/g, ' ').trim()
  return flat.length > length ? `${flat.slice(0, length)}…` : flat
}

function authorOf(m: AssistanceMessage): string {
  if (isMine(m)) return t(k('you'))
  return m.author_name ?? t(k('teamAuthor'))
}

const subjects = computed(() =>
  groupThreads(messages.value)
    .map(({ root, replies }) => {
      const all = [root, ...replies]
      const last = all.at(-1) ?? root
      const state: SubjectState = root.closed_at ? 'resolved' : fromAsker(last) ? 'waiting' : 'answered'
      return {
        root,
        messages: all,
        last,
        state,
        about: aboutOf(root),
        title: root.subject ?? natureOf(root) ?? excerpt(root.body, 60),
        nature: natureOf(root),
        unread: all.filter(isUnread).length,
      }
    })
    .sort((a, b) => b.last.created_at.localeCompare(a.last.created_at)),
)
type Subject = (typeof subjects.value)[number]

const unreadCount = computed(() => (feed.value ? messages.value.filter(isUnread).length : (status.value?.unread ?? 0)))

const STATE_INTENT = { waiting: 'warning', answered: 'info', resolved: 'success' } as const
const stateLabel = (s: Subject) => t(k(`state.${isTeam.value ? 'team' : 'organization'}.${s.state}`))

function aboutLabel(s: Subject): string {
  if (s.about === 'file') return t(k('about.fileBadge'), { reference: s.root.reference_code ?? '' })
  return t(k(`about.${s.about}Badge`))
}

const TABS = { organization: ['open', 'resolved'], team: ['waiting', 'answered', 'resolved'] } as const
type Tab = 'open' | 'resolved' | 'waiting' | 'answered'
const tab = ref<Tab>('open')
watch(isTeam, (team) => (tab.value = team ? 'waiting' : 'open'), { immediate: true })

const inTab = (s: Subject, key: Tab) =>
  key === 'open' ? s.state !== 'resolved' : key === 'resolved' ? s.state === 'resolved' : s.state === key

const KINDS = ['all', 'file', 'platform', 'idea'] as const
const kind = ref<(typeof KINDS)[number]>('all')
const byKind = computed(() => subjects.value.filter((s) => kind.value === 'all' || s.about === kind.value))
const tabs = computed(() =>
  TABS[isTeam.value ? 'team' : 'organization'].map((key) => ({
    key,
    count: byKind.value.filter((s) => inTab(s, key)).length,
  })),
)
const shown = computed(() => byKind.value.filter((s) => inTab(s, tab.value)))

type View = 'list' | 'new' | 'subject'
const view = ref<View>('list')
const currentId = ref<Uuid | null>(null)
const current = computed(() => subjects.value.find((s) => s.root.id === currentId.value) ?? null)

function openSubject(rootId: Uuid): void {
  currentId.value = rootId
  view.value = 'subject'
  draft.value = ''
  error.value = null
  void scrollToEnd()
}

function backToList(): void {
  view.value = 'list'
  currentId.value = null
  error.value = null
}

watch(assistance.open, (open) => {
  if (!open) return
  void refreshFeed()
  if (view.value === 'subject' && !current.value) backToList()
})

function fileLink(proposalId: ProposalId): string {
  return localePath(isTeam.value ? `/admin/propositions/${proposalId}` : `/mon-organisation/dossiers/${proposalId}`)
}

// ---------------------------------------------------------------------------
// Lecture : un sujet défile vers le bas, et ce qu'on y voit est lu
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
    assistance.open.value && view.value === 'subject' && current.value
      ? current.value.messages.filter(isUnread).map((m) => m.id)
      : [],
  onRead: async (ids) => {
    if (!personId.value) return
    await api.assistance.markRead(personId.value, ids)
    for (const m of messages.value) if (ids.includes(m.id)) m.is_read = true
  },
})

// ---------------------------------------------------------------------------
// Rédaction
// ---------------------------------------------------------------------------

const busy = ref(false)
const error = ref<string | null>(null)
const draft = ref('')
const about = ref<About>('file')
const proposalId = ref<ProposalId | null>(null)
const subjectTitle = ref('')
const topic = ref<string | null>(null)
const topicOther = ref('')

const proposalOptions = computed<SelectOption[]>(() =>
  (feed.value?.proposals ?? []).map((p) => ({ value: p.id, label: `${p.reference_code} · ${tr(p.title)}` })),
)
const topicOptions = computed<SelectOption[]>(() =>
  (about.value === 'file' ? fileTopics.value : platformTopics.value.filter((term) => term.code !== SUGGESTION)).map(
    (term) => ({ value: term.code, label: tr(term.label) }),
  ),
)
watch(about, () => {
  topic.value = null
  topicOther.value = ''
})

const newSubjectReady = computed(
  () =>
    subjectTitle.value.trim().length >= 2 &&
    draft.value.trim().length > 0 &&
    (about.value === 'idea' ||
      (topic.value !== null && (topic.value !== 'other' || topicOther.value.trim() !== ''))) &&
    (about.value !== 'file' || proposalId.value !== null),
)

function startNew(): void {
  const focus = assistance.focusProposalId.value
  const followed = feed.value?.proposals ?? []
  view.value = 'new'
  error.value = null
  draft.value = ''
  subjectTitle.value = ''
  about.value = followed.length > 0 ? 'file' : 'platform'
  proposalId.value = followed.find((p) => p.id === focus)?.id ?? (followed.length === 1 ? followed[0]!.id : null)
  topic.value = null
  topicOther.value = ''
}

async function run<T>(action: () => Promise<T>): Promise<T | null> {
  busy.value = true
  error.value = null
  try {
    const result = await action()
    await Promise.all([refreshFeed(), refreshStatus()])
    return result
  } catch (thrown) {
    error.value = apiErrorMessage(thrown, (key) => t(key))
    return null
  } finally {
    busy.value = false
  }
}

async function createSubject(): Promise<void> {
  const person = personId.value
  if (!person || !newSubjectReady.value || busy.value) return
  const subject = subjectTitle.value.trim()
  const body = draft.value.trim()
  const created = await run(() => {
    if (about.value === 'file' && proposalId.value) {
      return api.workspace.reply(person, {
        proposal_id: proposalId.value,
        parent_id: null,
        body,
        topic: topic.value,
        topic_other: topic.value === 'other' ? topicOther.value.trim() : null,
        subject,
      })
    }
    return api.assistance.open(person, { topic: about.value === 'idea' ? SUGGESTION : (topic.value ?? ''), subject, body })
  })
  if (created) openSubject(created.id)
}

async function reply(): Promise<void> {
  const person = personId.value
  const subject = current.value
  const body = draft.value.trim()
  if (!person || !subject || body.length === 0 || busy.value) return
  const root = subject.root
  const sent = await run(() => {
    if (!root.proposal_id) return api.assistance.reply(person, root.id, { body })
    if (isTeam.value) {
      return api.review.comment(person, {
        proposal_id: root.proposal_id,
        parent_id: root.id,
        visibility: 'submitter',
        body,
        is_change_request: false,
      })
    }
    return api.workspace.reply(person, { proposal_id: root.proposal_id, parent_id: root.id, body })
  })
  if (sent) draft.value = ''
}

async function closeSubject(subject: Subject): Promise<void> {
  const person = personId.value
  if (!person) return
  await run(() => api.proposals.closeSubject(person, subject.root.id, subject.state !== 'resolved'))
}

async function resolveRequest(commentId: Uuid, resolved: boolean): Promise<void> {
  const person = personId.value
  if (!person) return
  const done = await run(() => api.workspace.resolve(person, { comment_id: commentId, resolved }))
  if (done !== null) assistance.revision.value++
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key !== 'Enter' || event.shiftKey || event.isComposing) return
  event.preventDefault()
  void reply()
}

const title = computed(() => t(k(isTeam.value ? 'team.title' : 'organization.title')))
const intro = computed(() => t(k(isTeam.value ? 'team.intro' : 'organization.intro')))
const launcherLabel = computed(() =>
  unreadCount.value > 0 ? t(k('launcherUnread'), { title: title.value, count: unreadCount.value }, unreadCount.value) : title.value,
)
</script>

<template>
  <UiFloatingPanel
    v-if="side"
    v-model:open="assistance.open.value"
    :title="title"
    :launcher-label="launcherLabel"
    :launcher-icon="isTeam ? 'help' : 'chat'"
    :launcher-text="isTeam ? null : t(k('organization.launcher'))"
    :launcher-badge="unreadCount"
    :stack="assistance.stack.value"
    width="30rem"
    height="min(46rem, calc(100dvh - 7rem))"
    expandable
    fullscreen-on-mobile
    :scroll-body="false"
  >
    <p class="border-b border-border-subtle px-5 py-2 text-sm text-text-muted">{{ intro }}</p>

    <div v-if="feedError && !feed" class="p-5">
      <UiErrorState :retry-label="t('common.actions.retry')" @retry="refreshFeed()" />
    </div>

    <div v-else-if="feedStatus === 'pending' && !feed" class="flex flex-col gap-3 p-5">
      <UiSkeletonLoader height="4rem" />
      <UiSkeletonLoader height="4rem" />
    </div>

    <!-- LA LISTE DES SUJETS -->
    <div v-else-if="view === 'list'" class="flex min-h-0 flex-1 flex-col">
      <div class="flex flex-col gap-3 px-4 pt-4 pb-3">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div role="tablist" :aria-label="t(k('filterLabel'))" class="flex gap-1 rounded-lg bg-surface-sunken p-1">
            <button
              v-for="entry in tabs"
              :key="entry.key"
              type="button"
              role="tab"
              :aria-selected="tab === entry.key"
              class="min-h-9 cursor-pointer rounded-md px-3 text-sm font-semibold"
              :class="tab === entry.key ? 'bg-surface-raised text-text shadow-sm' : 'text-text-secondary'"
              @click="tab = entry.key"
            >
              {{ t(k(`tab.${entry.key}`), { count: entry.count }) }}
            </button>
          </div>
          <UiButton v-if="!isTeam" variant="primary" icon="plus" @click="startNew">{{ t(k('newSubject')) }}</UiButton>
        </div>
        <div v-if="isTeam" role="group" :aria-label="t(k('kindLabel'))" class="flex flex-wrap gap-1.5">
          <button
            v-for="entry in KINDS"
            :key="entry"
            type="button"
            :aria-pressed="kind === entry"
            class="min-h-8 cursor-pointer rounded-full border px-3 text-sm font-semibold"
            :class="kind === entry ? 'border-accent-solid bg-accent-solid text-accent-contrast' : 'border-border bg-surface-raised text-text'"
            @click="kind = entry"
          >
            {{ t(k(`kind.${entry}`)) }}
          </button>
        </div>
      </div>

      <div class="min-h-0 flex-1 overflow-y-auto px-4 pb-4">
        <UiEmptyState
          v-if="shown.length === 0"
          icon="chat"
          compact
          :title="t(k(tab === 'resolved' ? 'empty.resolved' : isTeam ? 'empty.team' : 'empty.organization'))"
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
              <span v-if="isTeam" class="text-sm text-text">
                {{ [subject.root.organization_name, subject.root.author_name].filter(Boolean).join(' · ') }}
              </span>
              <span class="flex flex-wrap gap-1.5">
                <UiBadge size="sm" :intent="subject.about === 'platform' ? 'info' : 'neutral'" :label="aboutLabel(subject)" />
                <UiBadge v-if="subject.nature" size="sm" :label="subject.nature" />
                <UiBadge size="sm" :intent="STATE_INTENT[subject.state]" :label="stateLabel(subject)" />
              </span>
              <span class="truncate text-sm text-text-muted">{{ authorOf(subject.last) }} : {{ excerpt(subject.last.body) }}</span>
            </button>
          </li>
        </ul>
      </div>
    </div>

    <!-- NOUVEAU SUJET (organisation) -->
    <form v-else-if="view === 'new'" class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-4 py-4" @submit.prevent="createSubject">
      <div class="flex items-center gap-2">
        <UiButton variant="ghost" size="sm" icon="arrow-left" icon-only :label="t(k('backToSubjects'))" @click="backToList" />
        <h3 class="text-base font-semibold">{{ t(k('newSubject')) }}</h3>
      </div>
      <fieldset class="flex flex-col gap-2">
        <legend class="mb-2 text-sm font-semibold">{{ t(k('about.label')) }}</legend>
        <label
          v-for="entry in (['file', 'platform', 'idea'] as const)"
          v-show="entry !== 'file' || proposalOptions.length > 0"
          :key="entry"
          class="flex min-h-11 cursor-pointer items-start gap-3 rounded-lg border px-3 py-2.5"
          :class="about === entry ? 'border-accent bg-accent-surface' : 'border-border bg-surface-raised'"
        >
          <input v-model="about" type="radio" name="assistance-about" :value="entry" class="mt-1 size-4 accent-(--color-accent)" />
          <span class="flex flex-col">
            <span class="font-semibold">{{ t(k(`about.${entry}`)) }}</span>
            <span class="text-sm text-text-muted">{{ t(k(`about.${entry}Hint`)) }}</span>
          </span>
        </label>
      </fieldset>
      <UiSelect
        v-if="about === 'file'"
        id="assistance-proposal"
        :model-value="proposalId"
        :label="t(k('proposalLabel'))"
        required
        :placeholder="t(k('proposalPlaceholder'))"
        :options="proposalOptions"
        @update:model-value="(value: string) => (proposalId = value)"
      />
      <UiSelect
        v-if="about !== 'idea'"
        id="assistance-topic"
        :model-value="topic"
        :label="t(k('topicLabel'))"
        required
        :placeholder="t(k('topicPlaceholder'))"
        :options="topicOptions"
        @update:model-value="(value: string) => (topic = value)"
      />
      <UiInput
        v-if="about === 'file' && topic === 'other'"
        id="assistance-topic-other"
        v-model="topicOther"
        :label="t(k('topicOtherLabel'))"
        required
        :maxlength="80"
      />
      <UiAlert v-if="about === 'idea'" intent="info" :message="t(k('about.ideaNotice'))" />
      <UiInput
        id="assistance-subject"
        v-model="subjectTitle"
        :label="t(k('subjectLabel'))"
        :placeholder="t(k(`subjectPlaceholder.${about}`))"
        :maxlength="120"
        required
      />
      <UiTextarea id="assistance-first" v-model="draft" :label="t(k('messageLabel'))" :rows="5" required />
      <UiAlert v-if="error" intent="danger" :message="error" live />
      <div class="flex justify-end gap-2">
        <UiButton variant="secondary" @click="backToList">{{ t('common.actions.cancel') }}</UiButton>
        <UiButton type="submit" variant="primary" :loading="busy" :disabled="!newSubjectReady">
          {{ t(k(about === 'idea' ? 'sendIdea' : 'openSubject')) }}
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
        <p v-if="isTeam" class="ps-11 text-sm">
          {{ [current.root.organization_name, current.root.author_name].filter(Boolean).join(' · ') }}
        </p>
        <div class="flex flex-wrap items-center gap-1.5 ps-11">
          <UiBadge size="sm" :intent="current.about === 'platform' ? 'info' : 'neutral'" :label="aboutLabel(current)" />
          <UiBadge v-if="current.nature" size="sm" :label="current.nature" />
          <UiBadge size="sm" :intent="STATE_INTENT[current.state]" :label="stateLabel(current)" />
        </div>
        <div class="flex flex-wrap gap-2 ps-11">
          <UiButton
            v-if="current.root.proposal_id"
            variant="secondary"
            size="sm"
            :to="fileLink(current.root.proposal_id)"
            @click="assistance.open.value = false"
          >
            {{ t(k('openFile'), { reference: current.root.reference_code ?? '' }) }}
          </UiButton>
          <UiButton
            variant="secondary"
            size="sm"
            :icon="current.state === 'resolved' ? undefined : 'check'"
            :loading="busy"
            @click="closeSubject(current)"
          >
            {{ t(k(current.state === 'resolved' ? 'reopenSubject' : 'markSubjectResolved')) }}
          </UiButton>
        </div>
      </div>

      <div ref="list" class="min-h-0 flex-1 overflow-y-auto px-4 py-4">
        <ol class="flex flex-col gap-4">
          <li v-for="message in current.messages" :key="message.id" :data-comment-id="message.id">
            <article class="flex flex-col gap-1" :class="isMine(message) ? 'items-end' : 'items-start'">
              <p class="flex flex-wrap items-baseline gap-x-2 px-1 text-xs text-text-subtle">
                <span class="font-semibold text-text-secondary">{{ authorOf(message) }}</span>
                <time :datetime="message.created_at">{{ dateTime(message.created_at, viewerTimezone) }}</time>
                <span v-if="message.edited_at">· {{ t(k('edited')) }}</span>
              </p>
              <div
                class="max-w-[85%] rounded-lg border px-3.5 py-2.5"
                :class="
                  message.is_change_request
                    ? 'border-warning-border bg-surface-raised'
                    : isMine(message)
                      ? 'border-accent-border bg-accent-surface'
                      : 'border-border bg-surface-raised'
                "
              >
                <UiBadge v-if="message.is_change_request" class="mb-2" intent="warning" icon="warning" size="sm" :label="t(k('changeRequest'))" />
                <p class="text-sm whitespace-pre-line text-text">{{ message.body }}</p>
              </div>
              <div v-if="message.is_change_request" class="flex flex-wrap items-center gap-1">
                <UiBadge
                  v-if="message.resolved_at"
                  intent="success"
                  icon="check"
                  size="sm"
                  :label="t(k('requestResolved'), { date: dateTime(message.resolved_at, viewerTimezone) })"
                />
                <UiButton
                  v-if="!isTeam"
                  variant="ghost"
                  size="sm"
                  :icon="message.resolved_at ? undefined : 'check'"
                  :loading="busy"
                  @click="resolveRequest(message.id, message.resolved_at === null)"
                >
                  {{ t(k(message.resolved_at ? 'unmarkRequest' : 'markRequest')) }}
                </UiButton>
              </div>
            </article>
          </li>
        </ol>
      </div>

      <p v-if="current.state === 'resolved'" class="mx-4 rounded-md bg-success-surface px-3 py-2 text-sm text-success">
        {{ t(k('resolvedNotice')) }}
      </p>
      <form class="flex flex-col gap-2 border-t border-border-subtle px-4 pt-3 pb-4" @submit.prevent="reply">
        <UiAlert v-if="error" intent="danger" :message="error" live />
        <div class="flex items-end gap-2">
          <UiTextarea
            id="assistance-draft"
            v-model="draft"
            class="min-w-0 flex-1"
            :label="t(k('replyLabel'))"
            hide-label
            :rows="1"
            auto-grow
            :max-rows="6"
            :placeholder="t(k(isTeam ? 'replyPlaceholderTeam' : 'replyPlaceholder'))"
            @keydown="onKeydown"
          />
          <UiButton type="submit" icon="arrow-right" icon-only :label="t(k('send'))" :loading="busy" :disabled="draft.trim().length === 0" />
        </div>
        <p class="text-xs text-text-subtle max-sm:hidden">{{ t(k('sendHint')) }}</p>
      </form>
    </template>
  </UiFloatingPanel>
</template>
