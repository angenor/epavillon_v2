<script setup lang="ts">
import type { OnlineSpeakerInput, SpeakerLinkInvite } from '~/types/programme/online-speaker'
import type { OnlineSpeakerField } from '~/utils/speaker-links'

/**
 * LE LIEN D'INSCRIPTION DES INTERVENANTS EN LIGNE, sans compte : le porteur le
 * transmet, la personne s'inscrit et reçoit aussitôt son lien Zoom personnel.
 *
 * Le jeton se lit dans l'adresse, dans le navigateur seulement, et part en
 * en-tête (`X-Speaker-Link`) : ni rendu serveur, ni état sérialisé. L'adresse
 * est fixe : les courriels de l'API y mènent.
 */

definePageMeta({ layout: 'public' })
defineI18nRoute({ paths: { fr: '/intervenant', en: '/speaker' } })

const route = useRoute()
const api = useApi()
const { t } = useI18n()
const { tr } = useI18nText()
const { dayLong, time, zoneLabel } = useDateTime()
const k = (key: string, params: Record<string, unknown> = {}) => t(`speaker.${key}`, params)

type State = 'loading' | 'invalid' | 'error' | 'form' | 'done'
const state = ref<State>('loading')
const token = ref<string | null>(null)
const invite = ref<SpeakerLinkInvite | null>(null)
const loadError = ref<string | null>(null)

async function load(): Promise<void> {
  const value = token.value
  if (!value) {
    state.value = 'invalid'
    return
  }
  state.value = 'loading'
  loadError.value = null
  try {
    invite.value = await api.speakerLinks.invite(value)
    state.value = invite.value ? 'form' : 'invalid'
  } catch (error) {
    loadError.value = apiErrorMessage(error, t)
    state.value = 'error'
  }
}

let waiter: AbortController | null = null
onMounted(() => {
  const value = route.query.jeton
  token.value = typeof value === 'string' && value.trim() ? value.trim() : null
  void load()
})
onBeforeUnmount(() => waiter?.abort())

const zone = computed(() => (invite.value ? zoneLabel(invite.value.timezone, invite.value.city ?? undefined) : ''))
const when = computed(() => {
  const value = invite.value
  if (!value) return ''
  return k('when', {
    day: dayLong(value.starts_at, value.timezone),
    start: time(value.starts_at, value.timezone),
    end: time(value.ends_at, value.timezone),
    zone: zone.value,
  })
})

const form = ref<OnlineSpeakerInput>(blankOnlineSpeaker())
const errors = ref<Partial<Record<OnlineSpeakerField, string>>>({})
const consentError = ref<string | undefined>(undefined)
const submitError = ref<string | null>(null)
const phase = ref<'idle' | 'sending' | 'waiting'>('idle')
const sentTo = ref('')
const joinUrl = ref<string | null>(null)
const FIELDS = new Set<string>(['first_name', 'last_name', 'job_title', 'organization', 'email'])

async function submit(): Promise<void> {
  const value = token.value
  if (!value || phase.value !== 'idle') return
  const problems = onlineSpeakerProblems(form.value)
  errors.value = Object.fromEntries(Object.entries(problems).map(([field, kind]) => [field, t(`validation.${kind}`)]))
  consentError.value = form.value.consent ? undefined : k('consentRequired')
  submitError.value = null
  if (Object.keys(problems).length || !form.value.consent) return
  const input = cleanOnlineSpeaker(form.value)
  phase.value = 'sending'
  let speakerId: string
  try {
    speakerId = (await api.speakerLinks.register(value, input)).speaker_id
  } catch (error) {
    if (error instanceof ApiRequestError && error.status === 404) {
      state.value = 'invalid'
      phase.value = 'idle'
      return
    }
    submitError.value = apiErrorMessage(error, t)
    const field = error instanceof ApiRequestError ? error.field : null
    if (field && FIELDS.has(field)) errors.value = { ...errors.value, [field]: submitError.value }
    phase.value = 'idle'
    return
  }
  phase.value = 'waiting'
  waiter = new AbortController()
  const access = await awaitZoomLink(() => api.speakerLinks.zoom(value, speakerId), { signal: waiter.signal }).catch(() => null)
  sentTo.value = input.email
  joinUrl.value = access?.join_url ?? null
  phase.value = 'idle'
  state.value = 'done'
}

useHead(() => ({
  title: invite.value ? k('title', { title: tr(invite.value.title) }) : k('titleBare'),
  // Le jeton est dans l'adresse : ni indexée, ni transmise aux sites qu'on ouvre d'ici.
  meta: [
    { name: 'robots', content: 'noindex' },
    { name: 'referrer', content: 'no-referrer' },
  ],
}))
</script>

<template>
  <div data-direction="condensee" class="min-h-[60vh] font-sans text-text">
    <div class="mx-auto max-w-2xl">
      <ClientOnly>
        <UiLoadingState v-if="state === 'loading'" variant="form" :lines="4" :label="k('loading')" />

        <div v-else-if="state === 'invalid'" class="flex flex-col items-start gap-3 rounded-lg bg-surface-raised p-5 shadow-lg sm:p-6">
          <p class="font-condensed text-[1.25rem] font-bold tracking-[0.03em] text-text-secondary uppercase">{{ k('invalid.title') }}</p>
          <p class="text-sm leading-relaxed">{{ k('invalid.description') }}</p>
        </div>

        <UiErrorState v-else-if="state === 'error'" :title="k('error')" :description="loadError ?? undefined" @retry="load" />

        <article v-else-if="invite" class="overflow-hidden rounded-lg bg-surface-raised shadow-lg">
          <header class="bg-surface-inverse px-5 py-5 text-text-on-inverse sm:px-6">
            <p class="font-condensed text-[0.95rem] font-semibold tracking-[0.05em] uppercase opacity-85">{{ tr(invite.event_title) }}</p>
            <h1 class="mt-0.5 font-condensed text-[1.9rem] leading-[1.05] font-bold text-balance text-inherit">{{ tr(invite.title) }}</h1>
            <p class="mt-1.5 text-sm first-letter:uppercase">{{ when }}</p>
          </header>

          <form v-if="state === 'form'" class="grid gap-5 p-5 sm:p-6" novalidate @submit.prevent="submit">
            <p class="text-[0.92rem] leading-relaxed text-text-secondary">{{ k('intro') }}</p>
            <SpeakerOnlineSpeakerFields v-model="form" id-prefix="speaker" :errors="errors" :disabled="phase !== 'idle'" />
            <UiCheckbox
              id="speaker-consent"
              v-model="form.consent"
              :label="k('consent')"
              :error="consentError"
              required
              :disabled="phase !== 'idle'"
            />
            <UiAlert v-if="submitError" intent="danger" live :message="submitError" />
            <div class="flex flex-col items-start gap-2">
              <UiButton type="submit" size="lg" :loading="phase !== 'idle'">{{ k('submit') }}</UiButton>
              <p v-if="phase === 'waiting'" class="text-sm text-text-secondary" role="status">{{ k('waiting') }}</p>
            </div>
          </form>

          <div v-else class="flex flex-col items-start gap-4 p-5 sm:p-6" role="status">
            <span class="grid size-11 place-items-center rounded-full bg-success-surface text-success" aria-hidden="true">
              <UiIcon name="check" size="1.4rem" :stroke-width="2.4" />
            </span>
            <h2 class="font-condensed text-[1.6rem] leading-[1.1] font-bold">{{ joinUrl ? k('done.title') : k('done.laterTitle') }}</h2>
            <p class="max-w-[52ch] text-[0.92rem] text-text-secondary">
              {{ joinUrl ? k('done.lead', { email: sentTo }) : k('done.later', { email: sentTo }) }}
            </p>
            <template v-if="joinUrl">
              <UiButton size="lg" icon="video" :href="joinUrl">{{ k('done.open') }}</UiButton>
              <UiCopyButton :text="joinUrl" :label="k('done.copy')" :label-copied="k('done.copied')" />
            </template>
            <p class="text-[0.8rem] text-text-muted">{{ k('done.connect', { time: time(invite.starts_at, invite.timezone), zone }) }}</p>
          </div>
        </article>

        <template #fallback>
          <UiLoadingState variant="form" :lines="4" :label="k('loading')" />
        </template>
      </ClientOnly>
    </div>
  </div>
</template>
