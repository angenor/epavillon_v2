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
  <section class="rounded-lg bg-surface-raised p-6 font-sans" aria-labelledby="questions-titre">
    <h2 id="questions-titre" class="font-sans text-xl leading-tight font-bold text-text">
      {{ t('activity.questions.title') }}
    </h2>

    <form v-if="auth.isAuthenticated" class="mt-3 flex flex-col gap-2" @submit.prevent="ask">
      <UiTextarea
        v-model="draft"
        :label="t('activity.questions.label')"
        hide-label
        block
        :rows="3"
        :maxlength="2000"
        :show-counter="false"
        :placeholder="t('activity.questions.placeholder')"
      />
      <UiButton type="submit" variant="primary" size="lg" block :loading="sending" :disabled="draft.trim().length < 3">
        {{ t('activity.questions.send') }}
      </UiButton>
    </form>
    <p v-else class="mt-3 text-sm text-text-muted">
      <NuxtLink :to="loginTo" class="inline-flex min-h-11 items-center font-bold text-accent underline underline-offset-4">
        {{ t('activity.questions.login') }}
      </NuxtLink>
      {{ t('activity.questions.loginHint') }}
    </p>
    <p v-if="error" class="mt-2 text-sm text-danger" role="alert">{{ error }}</p>

    <UiSkeletonLoader v-if="status === 'loading'" class="mt-5" variant="text" :lines="3" />
    <p v-else-if="status === 'failed'" class="mt-4 text-sm text-text-muted">
      {{ t('activity.questions.failed') }}
      <button type="button" class="ml-1 inline-flex min-h-11 cursor-pointer items-center font-bold text-accent underline underline-offset-4" @click="load">
        {{ t('common.actions.retry') }}
      </button>
    </p>

    <template v-else>
      <p class="mt-3 text-[13px] text-text-muted">
        {{ questions.length ? t('activity.questions.count', questions.length) : t('activity.questions.empty') }}
      </p>
      <ol v-if="questions.length" class="mt-3 border-t border-border-subtle">
        <li v-for="question in shown" :key="question.id" class="flex items-start gap-3 border-b border-border-subtle py-3">
          <button
            type="button"
            class="flex size-12 shrink-0 flex-col items-center justify-center rounded-md border text-sm leading-none font-bold tabular-nums"
            :class="[
              question.has_voted ? 'border-accent-solid bg-accent-solid text-accent-contrast' : 'border-border bg-surface text-text',
              auth.isAuthenticated ? 'cursor-pointer hover:border-accent' : 'cursor-default',
            ]"
            :aria-pressed="question.has_voted"
            :aria-label="t(question.has_voted ? 'activity.questions.unsupport' : 'activity.questions.support', { count: question.vote_count })"
            :disabled="!auth.isAuthenticated || busy === question.id"
            @click="support(question)"
          >
            <UiIcon name="chevron-up" size="1.125rem" />
            {{ question.vote_count }}
          </button>
          <div class="min-w-0 pt-0.5 text-[15px] leading-snug text-text">
            <p class="break-words">{{ question.body }}</p>
            <p
              v-if="question.is_mine"
              class="mt-1 text-[11px] font-bold text-accent uppercase"
              :style="{ letterSpacing: 'var(--tracking-caps)' }"
            >
              {{ t('activity.questions.mine') }}
            </p>
            <div
              v-for="answer in question.answers"
              :key="answer.id"
              class="mt-2 rounded-md bg-surface px-3 py-2 text-sm"
            >
              <p class="text-[11px] font-bold text-text-muted uppercase" :style="{ letterSpacing: 'var(--tracking-caps)' }">
                {{ t(answer.is_official ? 'activity.questions.officialAnswer' : 'activity.questions.answer') }}
              </p>
              <p class="mt-0.5 break-words">{{ answer.body }}</p>
            </div>
          </div>
        </li>
      </ol>
      <button
        v-if="questions.length > VISIBLE"
        type="button"
        class="mt-1 inline-flex min-h-11 cursor-pointer items-center text-sm font-bold text-accent underline underline-offset-4"
        @click="expanded = !expanded"
      >
        {{ expanded ? t('activity.questions.less') : t('activity.questions.more', questions.length - VISIBLE) }}
      </button>
    </template>
  </section>
</template>
