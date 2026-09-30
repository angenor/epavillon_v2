<script setup lang="ts">
import type { PublicSessionQuestion } from '~/types/programme/registration'
import { estInchange } from '~/composables/api/etiquete'

/** Les questions du public : on les lit sans compte, on en pose et on en soutient une fois connecté. */

interface Props {
  sessionId: string
}

const props = defineProps<Props>()

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()
const route = useRoute()
const localePath = useLocalePath()

const questions = ref<PublicSessionQuestion[]>([])
const status = ref<'loading' | 'ready' | 'failed'>('loading')
const draft = ref('')
const sending = ref(false)
const error = ref('')
const busy = ref<string | null>(null)

const VISIBLE = 5
const expanded = ref(false)
const shown = computed(() => (expanded.value ? questions.value : questions.value.slice(0, VISIBLE)))

async function load(): Promise<void> {
  status.value = 'loading'
  try {
    const read = await api.pavillon.questions(props.sessionId)
    if (!estInchange(read)) questions.value = read.valeur
    status.value = 'ready'
  } catch {
    status.value = 'failed'
  }
}

onMounted(load)

function replace(updated: PublicSessionQuestion): void {
  questions.value = questions.value
    .map((question) => (question.id === updated.id ? updated : question))
    .sort((a, b) => b.vote_count - a.vote_count || a.created_at.localeCompare(b.created_at))
}

async function ask(): Promise<void> {
  const body = draft.value.trim()
  if (!body || sending.value) return
  sending.value = true
  error.value = ''
  try {
    questions.value = [...questions.value, await api.pavillon.poserQuestion(props.sessionId, { body })]
    draft.value = ''
  } catch (failure) {
    error.value = apiErrorMessage(failure, t)
  } finally {
    sending.value = false
  }
}

async function support(question: PublicSessionQuestion): Promise<void> {
  if (!auth.isAuthenticated || busy.value) return
  busy.value = question.id
  error.value = ''
  try {
    replace(
      question.has_voted
        ? await api.pavillon.retirerVote(props.sessionId, question.id)
        : await api.pavillon.voter(props.sessionId, question.id),
    )
  } catch (failure) {
    error.value = apiErrorMessage(failure, t)
  } finally {
    busy.value = null
  }
}

const loginTo = computed(() => ({ path: localePath('auth-login'), query: { redirect: route.fullPath } }))
</script>

<template>
  <section class="rounded-lg border-2 border-poster-ink bg-poster-paper-raised p-5" aria-labelledby="questions-titre">
    <div class="flex items-baseline justify-between gap-3">
      <h2 id="questions-titre" class="font-poster text-[1.625rem] font-black uppercase font-stretch-[68%]">
        {{ t('activity.questions.title') }}
      </h2>
      <span v-if="status === 'ready'" class="font-poster-mono text-xs text-poster-ink-muted">
        {{ t('activity.questions.count', questions.length) }}
      </span>
    </div>

    <UiSkeletonLoader v-if="status === 'loading'" class="mt-4" variant="text" :lines="3" />
    <div v-else-if="status === 'failed'" class="mt-4 text-sm text-poster-ink-muted">
      {{ t('activity.questions.failed') }}
      <button type="button" class="ml-1 cursor-pointer font-semibold underline underline-offset-4" @click="load">
        {{ t('common.actions.retry') }}
      </button>
    </div>

    <template v-else>
      <p v-if="!questions.length" class="mt-3 text-sm text-poster-ink-muted">{{ t('activity.questions.empty') }}</p>
      <ol class="mt-3">
        <li v-for="question in shown" :key="question.id" class="flex items-start gap-3 border-t border-poster-line py-3">
          <button
            type="button"
            class="flex size-12 shrink-0 flex-col items-center justify-center rounded-md border-2 border-poster-ink font-poster-mono text-[0.8125rem] font-semibold"
            :class="[
              question.has_voted ? 'bg-poster-ink text-poster-on-ink-accent' : 'bg-poster-paper text-poster-ink',
              auth.isAuthenticated ? 'cursor-pointer' : 'cursor-default',
            ]"
            :aria-pressed="question.has_voted"
            :aria-label="t(question.has_voted ? 'activity.questions.unsupport' : 'activity.questions.support', { count: question.vote_count })"
            :disabled="!auth.isAuthenticated || busy === question.id"
            @click="support(question)"
          >
            <UiIcon name="chevron-up" size="0.875rem" />
            {{ question.vote_count }}
          </button>
          <div class="min-w-0 pt-0.5 text-sm leading-snug">
            <p>{{ question.body }}</p>
            <p v-if="question.is_mine" class="mt-1 font-poster-mono text-[0.6875rem] text-poster-ink-muted uppercase">
              {{ t('activity.questions.mine') }}
            </p>
            <p
              v-for="answer in question.answers"
              :key="answer.id"
              class="mt-2 rounded-md bg-poster-paper px-3 py-2 text-poster-ink"
            >
              <span class="block font-poster-mono text-[0.6875rem] font-semibold text-poster-ink-muted uppercase">
                {{ t(answer.is_official ? 'activity.questions.officialAnswer' : 'activity.questions.answer') }}
              </span>
              {{ answer.body }}
            </p>
          </div>
        </li>
      </ol>
      <button
        v-if="questions.length > VISIBLE"
        type="button"
        class="h-11 cursor-pointer text-sm font-semibold underline underline-offset-4"
        @click="expanded = !expanded"
      >
        {{ expanded ? t('activity.questions.less') : t('activity.questions.more', questions.length - VISIBLE) }}
      </button>

      <form v-if="auth.isAuthenticated" class="mt-2 flex gap-2" @submit.prevent="ask">
        <label class="sr-only" for="question-publique">{{ t('activity.questions.label') }}</label>
        <input
          id="question-publique"
          v-model="draft"
          type="text"
          maxlength="2000"
          class="h-12 min-w-0 flex-1 rounded-md border-2 border-poster-ink bg-poster-paper-raised px-3 text-poster-ink placeholder:text-poster-ink-muted"
          :placeholder="t('activity.questions.placeholder')"
        >
        <button
          type="submit"
          class="h-12 cursor-pointer rounded-md border-2 border-poster-ink bg-poster-ink px-4 font-bold text-poster-on-ink-accent disabled:cursor-not-allowed disabled:opacity-60"
          :disabled="sending || draft.trim().length < 3"
        >
          {{ t('activity.questions.send') }}
        </button>
      </form>
      <p v-else class="mt-3 text-sm text-poster-ink-muted">
        <NuxtLink :to="loginTo" class="font-semibold text-poster-ink underline underline-offset-4">
          {{ t('activity.questions.login') }}
        </NuxtLink>
        {{ t('activity.questions.loginHint') }}
      </p>
      <p v-if="error" class="mt-2 text-sm text-danger" role="alert">{{ error }}</p>
    </template>
  </section>
</template>
