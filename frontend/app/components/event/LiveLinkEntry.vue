<script setup lang="ts">
import type { MeetingAccess } from '~/types/live'

/**
 * L'ENTRÉE DANS LE DIRECT PAR LE LIEN DU COURRIEL, pour qui s'est inscrit sans
 * compte (journée ou activité). Demande l'accès avec le jeton et renvoie aussitôt
 * au lien Zoom personnel. Pas encore prêt (409) : attend et réessaie seule.
 *
 * Le jeton se lit dans l'adresse, dans le navigateur seulement, et part en en-tête.
 */

interface Props {
  /** Préfixe des traductions de la page : mêmes clés, mots de la journée ou de l'activité. */
  messages: string
  /** Faux tant que la page n'a pas chargé ce qu'il faut pour demander l'accès. */
  ready: boolean
  requestAccess: (token: string) => Promise<MeetingAccess>
  isCancelled?: () => Promise<boolean>
  backTo: string
}

const props = withDefaults(defineProps<Props>(), { isCancelled: undefined })

const route = useRoute()
const { t } = useI18n()
const k = (key: string, params: Record<string, unknown> = {}) => t(`${props.messages}.${key}`, params)

type State = 'checking' | 'waiting' | 'redirecting' | 'invalid' | 'unregistered' | 'cancelled' | 'error'
const state = ref<State>('checking')
const message = ref<string | null>(null)
const token = ref<string | null>(null)
const joinUrl = ref<string | null>(null)
const retryIn = ref(0)

const RETRY_S = 20
let ticker: ReturnType<typeof setInterval> | undefined

function wait(): void {
  state.value = 'waiting'
  retryIn.value = RETRY_S
  clearInterval(ticker)
  ticker = setInterval(() => {
    retryIn.value -= 1
    if (retryIn.value <= 0) void attempt()
  }, 1000)
}

function go(access: MeetingAccess): void {
  if (!access.join_url) {
    message.value = k('noLink')
    state.value = 'error'
    return
  }
  joinUrl.value = access.join_url
  state.value = 'redirecting'
  window.location.replace(access.join_url)
}

async function attempt(): Promise<void> {
  clearInterval(ticker)
  if (!token.value || !props.ready) {
    state.value = 'invalid'
    return
  }
  state.value = 'checking'
  message.value = null
  try {
    go(await props.requestAccess(token.value))
  } catch (error) {
    if (error instanceof ApiRequestError) {
      if (error.code === 'LIVE_MEETING_NOT_READY') return wait()
      if (error.code === 'LIVE_MEETING_CANCELLED') {
        state.value = 'cancelled'
        return
      }
      if (error.status === 404 || error.status === 401) {
        state.value = 'invalid'
        return
      }
      if (error.code === 'LIVE_MEETING_REGISTRATION_REQUIRED') {
        message.value = error.message
        state.value = 'unregistered'
        return
      }
    }
    if (await props.isCancelled?.()) {
      state.value = 'cancelled'
      return
    }
    message.value = apiErrorMessage(error, t)
    state.value = 'error'
  }
}

onMounted(() => {
  const value = route.query.jeton
  token.value = typeof value === 'string' && value.trim() ? value.trim() : null
  if (props.ready) void attempt()
})
watch(
  () => props.ready,
  (ready) => {
    if (ready && import.meta.client) void attempt()
  },
)
onBeforeUnmount(() => clearInterval(ticker))
</script>

<template>
  <div class="flex flex-col items-start gap-3 p-5 sm:p-6" aria-live="polite">
    <ClientOnly>
      <p v-if="state === 'checking'" class="flex items-center gap-2 text-sm" role="status">
        <UiSpinner size="1rem" />{{ k('checking') }}
      </p>

      <template v-else-if="state === 'waiting'">
        <p class="font-condensed text-[1.25rem] font-bold tracking-[0.03em] text-warning uppercase">{{ k('waiting.title') }}</p>
        <p class="text-sm leading-relaxed">{{ k('waiting.description') }}</p>
        <p class="text-sm text-text-muted tabular-nums" role="status">{{ t(`${props.messages}.waiting.retry`, { seconds: retryIn }, retryIn) }}</p>
        <UiButton variant="secondary" icon="refresh" @click="attempt">{{ k('waiting.now') }}</UiButton>
      </template>

      <template v-else-if="state === 'redirecting'">
        <p class="flex items-center gap-2 text-sm" role="status"><UiSpinner size="1rem" />{{ k('redirecting') }}</p>
        <UiButton v-if="joinUrl" variant="live" icon="video" :href="joinUrl">{{ k('open') }}</UiButton>
      </template>

      <template v-else-if="state === 'invalid'">
        <p class="font-condensed text-[1.25rem] font-bold tracking-[0.03em] text-text-secondary uppercase">{{ k('invalid.title') }}</p>
        <p class="text-sm leading-relaxed">{{ k('invalid.description') }}</p>
        <UiButton variant="secondary" :to="props.backTo">{{ k('back') }}</UiButton>
      </template>

      <template v-else-if="state === 'unregistered'">
        <p class="font-condensed text-[1.25rem] font-bold tracking-[0.03em] text-text-secondary uppercase">{{ k('unregistered') }}</p>
        <p class="text-sm leading-relaxed">{{ message }}</p>
        <UiButton variant="signup" :to="props.backTo">{{ k('register') }}</UiButton>
      </template>

      <template v-else-if="state === 'cancelled'">
        <p class="font-condensed text-[1.25rem] font-bold tracking-[0.03em] text-danger uppercase">{{ k('cancelled.title') }}</p>
        <p class="text-sm leading-relaxed">{{ k('cancelled.description') }}</p>
        <UiButton variant="secondary" :to="props.backTo">{{ k('back') }}</UiButton>
      </template>

      <template v-else>
        <UiAlert intent="danger" :message="message ?? k('error')" />
        <div class="flex flex-wrap gap-x-5 gap-y-2">
          <UiButton icon="refresh" @click="attempt">{{ k('retry') }}</UiButton>
          <slot name="error-actions" />
        </div>
      </template>

      <template #fallback>
        <p class="flex items-center gap-2 text-sm"><UiSpinner size="1rem" />{{ k('checking') }}</p>
      </template>
    </ClientOnly>
  </div>
</template>
