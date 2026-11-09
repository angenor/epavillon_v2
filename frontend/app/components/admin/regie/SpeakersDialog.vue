<script setup lang="ts">
import type { RegieSession } from '~/types/admin-regie'
import type { OnlineSpeakerInput, RegieSpeaker } from '~/types/programme/online-speaker'
import type { TimeZoneName } from '~/types/shared'
import type { OnlineSpeakerField } from '~/utils/speaker-links'

/**
 * « Inscrire un intervenant » : la liste des inscrits en ligne, puis le
 * formulaire. Inscrit ici, l'intervenant est affiché d'office et son lien Zoom
 * part aussitôt ; on l'affiche aussi, prêt à copier, quand Zoom le rend à temps.
 */

interface Props {
  open: boolean
  session: RegieSession | null
  timezone: TimeZoneName
  zoneLabel: string | null
  canManageMeeting: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:open': [value: boolean] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const dt = useDateTime()
const api = useApi()
const k = (key: string, params: Record<string, unknown> = {}) => t(`admin.regie.speakers.${key}`, params)

const sessionId = computed(() => props.session?.session_id ?? null)
const { detail, state, error: loadError, load, moment } = useRegieSpeakers(() => sessionId.value)

const form = ref<OnlineSpeakerInput>(blankOnlineSpeaker())
const errors = ref<Partial<Record<OnlineSpeakerField, string>>>({})
const submitError = ref<string | null>(null)
const phase = ref<'form' | 'sending' | 'waiting' | 'done'>('form')
const done = ref<{ name: string; email: string; joinUrl: string | null } | null>(null)
const listing = ref<string | null>(null)
const listError = ref<string | null>(null)
let waiter: AbortController | null = null
const FIELDS = new Set<string>(['first_name', 'last_name', 'job_title', 'organization', 'email'])

function reset(): void {
  form.value = blankOnlineSpeaker()
  errors.value = {}
  submitError.value = null
  done.value = null
  phase.value = 'form'
}

watch(
  () => props.open,
  (open) => {
    waiter?.abort()
    if (!open) return
    reset()
    listError.value = null
    void load()
  },
)
onBeforeUnmount(() => waiter?.abort())

const when = computed(() => {
  const s = props.session
  if (!s) return ''
  return k('register.when', {
    day: dayMonth(s.starts_at, props.timezone, dt.intlLocale.value),
    start: dt.time(s.starts_at, props.timezone),
    end: dt.time(s.ends_at, props.timezone),
    zone: dt.zoneLabel(props.timezone, props.zoneLabel ?? undefined),
  })
})

const cc = computed(() => k('cc', { first: SPEAKER_LINKS_CC[0], second: SPEAKER_LINKS_CC[1] }))

function detailLine(speaker: RegieSpeaker): string {
  const who = [speaker.job_title, speaker.organization].filter(Boolean).join(', ')
  let status: string
  if (speaker.source === 'lien') status = k('register.signedUpOn', moment(speaker.created_at, props.timezone))
  else if (speaker.link_sent_at) status = k('register.sentOn', moment(speaker.link_sent_at, props.timezone))
  else status = k(speaker.email ? 'register.notSent' : 'register.noEmail')
  return who ? `${who} · ${status}` : status
}

async function showOnPage(speaker: RegieSpeaker): Promise<void> {
  const id = sessionId.value
  if (!id || listing.value) return
  listing.value = speaker.speaker_id
  listError.value = null
  try {
    detail.value = await api.adminRegie.listSpeaker(id, speaker.speaker_id)
  } catch (caught) {
    listError.value = apiErrorMessage(caught, t)
  } finally {
    listing.value = null
  }
}

async function submit(): Promise<void> {
  const id = sessionId.value
  if (!id || phase.value !== 'form') return
  const problems = onlineSpeakerProblems(form.value)
  errors.value = Object.fromEntries(Object.entries(problems).map(([field, kind]) => [field, t(`validation.${kind}`)]))
  submitError.value = null
  if (Object.keys(problems).length) return
  const input = { ...cleanOnlineSpeaker(form.value), consent: true }
  phase.value = 'sending'
  let speakerId: string
  try {
    speakerId = (await api.adminRegie.addSpeaker(id, input)).speaker_id
  } catch (caught) {
    submitError.value = apiErrorMessage(caught, t)
    const field = caught instanceof ApiRequestError ? caught.field : null
    if (field && FIELDS.has(field)) errors.value = { ...errors.value, [field]: submitError.value }
    phase.value = 'form'
    return
  }
  phase.value = 'waiting'
  waiter = new AbortController()
  const signal = waiter.signal
  const access = props.canManageMeeting
    ? await awaitZoomLink(() => api.meetings.speakerZoom(id, speakerId), { signal }).catch(() => null)
    : null
  if (signal.aborted) return
  done.value = { name: `${input.first_name} ${input.last_name}`, email: input.email, joinUrl: access?.join_url ?? null }
  phase.value = 'done'
  void load()
}

const title = computed(() => (done.value ? k('register.done.title', { name: done.value.name }) : k('register.title')))
const description = computed(() => {
  if (!done.value) return props.session ? k('register.description', { title: tr(props.session.title), when: when.value }) : undefined
  return done.value.joinUrl ? k('register.done.sentTo', { email: done.value.email }) : k('register.done.later')
})
const busy = computed(() => phase.value === 'sending' || phase.value === 'waiting')
</script>

<template>
  <UiModal
    :open="props.open"
    size="lg"
    fullscreen-mobile
    :dismissible="false"
    :title="title"
    :description="description"
    @update:open="emit('update:open', $event)"
  >
    <div v-if="done" class="grid gap-4">
      <template v-if="done.joinUrl">
        <UiInput id="regie-speaker-zoom" :model-value="done.joinUrl" readonly :label="k('register.done.link')" hide-optional />
      </template>
      <p class="text-[0.8rem] text-text-muted">{{ cc }}</p>
    </div>

    <form v-else id="regie-speaker-form" class="grid gap-6" novalidate @submit.prevent="submit">
      <section class="grid gap-2" aria-labelledby="regie-speakers-listed">
        <div class="flex flex-wrap items-center justify-between gap-2">
          <h3 id="regie-speakers-listed" class="font-condensed text-base font-semibold text-text-secondary">{{ k('register.listed') }}</h3>
          <UiCopyButton
            v-if="detail"
            :text="detail.signup_url"
            :label="k('register.copySignup')"
            :label-copied="k('register.copiedSignup')"
          />
        </div>
        <UiLoadingState v-if="state === 'loading' && !detail" variant="text" :lines="3" :label="k('register.loading')" />
        <UiErrorState
          v-else-if="state === 'error'"
          compact
          :title="k('register.loadError')"
          :description="loadError ?? undefined"
          :retry-label="t('common.actions.retry')"
          @retry="load"
        />
        <p v-else-if="detail && !detail.speakers.length" class="text-sm text-text-muted">{{ k('register.empty') }}</p>
        <ul v-else-if="detail" class="rounded-md border border-border-subtle">
          <li
            v-for="speaker in detail.speakers"
            :key="speaker.speaker_id"
            class="flex flex-wrap items-center gap-x-3 gap-y-1.5 border-b border-border-subtle px-3 py-2.5 text-sm last:border-b-0"
          >
            <div class="mr-auto min-w-0">
              <p class="font-semibold">{{ speaker.display_name }}</p>
              <p class="text-xs text-text-muted">{{ detailLine(speaker) }}</p>
            </div>
            <UiBadge v-if="speaker.is_listed" intent="success" size="sm" :label="k('register.onPage')" />
            <UiButton
              v-else
              variant="secondary"
              :loading="listing === speaker.speaker_id"
              @click="showOnPage(speaker)"
            >
              {{ k('register.showOnPage') }}
            </UiButton>
          </li>
        </ul>
        <UiAlert v-if="listError" intent="danger" compact live :message="listError" />
      </section>

      <section class="grid gap-3" aria-labelledby="regie-speaker-new">
        <h3 id="regie-speaker-new" class="font-condensed text-base font-semibold text-text-secondary">{{ k('register.newSpeaker') }}</h3>
        <SpeakerOnlineSpeakerFields v-model="form" id-prefix="regie-speaker" :errors="errors" :disabled="busy" />
        <p class="text-[0.8rem] text-text-muted">{{ k('register.note') }} {{ cc }}</p>
        <UiAlert v-if="submitError" intent="danger" compact live :message="submitError" />
        <p v-if="phase === 'waiting'" class="text-sm text-text-secondary" role="status">{{ k('register.waiting') }}</p>
      </section>
    </form>

    <template #footer>
      <template v-if="done">
        <UiCopyButton
          v-if="done.joinUrl"
          :text="done.joinUrl"
          select-target="regie-speaker-zoom"
          :label="k('register.done.copy')"
          :label-copied="k('register.done.copied')"
        />
        <UiButton variant="secondary" @click="reset">{{ k('register.done.another') }}</UiButton>
        <UiButton @click="emit('update:open', false)">{{ k('register.done.finish') }}</UiButton>
      </template>
      <template v-else>
        <UiButton variant="ghost" :disabled="phase === 'sending'" @click="emit('update:open', false)">{{ k('register.close') }}</UiButton>
        <UiButton type="submit" form="regie-speaker-form" :loading="busy">{{ k('register.submit') }}</UiButton>
      </template>
    </template>
  </UiModal>
</template>
