<script setup lang="ts">
import type { RegieSession } from '~/types/admin-regie'

interface Props {
  open: boolean
  session: RegieSession | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:open': [value: boolean] }>()

const { t } = useI18n()
const api = useApi()
const toast = useToast()
const k = (key: string, params: Record<string, unknown> = {}, plural?: number) =>
  plural === undefined ? t(`admin.regie.speakers.${key}`, params) : t(`admin.regie.speakers.${key}`, params, plural)

const { detail, state, error: loadError, load } = useRegieSpeakers(() => props.session?.session_id ?? null)
const sending = ref(false)
const sendError = ref<string | null>(null)

watch(
  () => props.open,
  (open) => {
    if (!open) return
    sendError.value = null
    void load()
  },
)

const reachable = computed(() => detail.value?.speakers.filter((speaker) => speaker.email).length ?? 0)

async function confirm(): Promise<void> {
  const id = props.session?.session_id
  if (!id || sending.value) return
  sending.value = true
  sendError.value = null
  try {
    const { recipients } = await api.adminRegie.resendZoomLinks(id)
    toast.success(k('resend.done', { count: recipients }, recipients))
    emit('update:open', false)
  } catch (caught) {
    sendError.value = apiErrorMessage(caught, t)
  } finally {
    sending.value = false
  }
}
</script>

<template>
  <UiModal
    :open="props.open"
    :title="k('resend.title')"
    :description="k('resend.description')"
    :dismissible="!sending"
    @update:open="emit('update:open', $event)"
  >
    <UiLoadingState v-if="state === 'loading' && !detail" variant="text" :lines="3" :label="k('register.loading')" />
    <UiErrorState
      v-else-if="state === 'error'"
      compact
      :title="k('register.loadError')"
      :description="loadError ?? undefined"
      :retry-label="t('common.actions.retry')"
      @retry="load"
    />
    <div v-else-if="detail" class="grid gap-3">
      <p v-if="!detail.speakers.length" class="text-sm text-text-muted">{{ k('register.empty') }}</p>
      <ul v-else class="rounded-md border border-border-subtle">
        <li
          v-for="speaker in detail.speakers"
          :key="speaker.speaker_id"
          class="flex flex-wrap items-center gap-x-3 gap-y-1.5 border-b border-border-subtle px-3 py-2.5 text-sm last:border-b-0"
        >
          <div class="mr-auto min-w-0">
            <p class="font-semibold">{{ speaker.display_name }}</p>
            <p class="text-xs break-all text-text-muted">{{ speaker.email ?? k('resend.noEmail') }}</p>
          </div>
          <UiBadge v-if="!speaker.email" intent="warning" size="sm" :label="k('resend.noEmailTag')" />
        </li>
      </ul>
      <p class="text-[0.8rem] text-text-muted">
        {{ k('resend.mails', { count: reachable }, reachable) }}
        {{ k('ccEach', { first: SPEAKER_LINKS_CC[0], second: SPEAKER_LINKS_CC[1] }) }}
      </p>
    </div>
    <UiAlert v-if="sendError" class="mt-3" intent="danger" live compact :message="sendError" />
    <template #footer>
      <UiButton variant="ghost" autofocus :disabled="sending" @click="emit('update:open', false)">{{ k('resend.cancel') }}</UiButton>
      <UiButton :loading="sending" :disabled="!reachable" @click="confirm">{{ k('resend.confirm', { count: reachable }, reachable) }}</UiButton>
    </template>
  </UiModal>
</template>
