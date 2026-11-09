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

async function confirm(): Promise<void> {
  const id = props.session?.session_id
  if (!id || sending.value) return
  sending.value = true
  sendError.value = null
  try {
    const { recipients } = await api.adminRegie.sendOwnerLink(id)
    toast.success(k('owner.done', { count: recipients }, recipients))
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
    :title="k('owner.title')"
    :description="k('owner.description')"
    :dismissible="!sending"
    @update:open="emit('update:open', $event)"
  >
    <UiLoadingState v-if="state === 'loading' && !detail" variant="text" :lines="2" :label="k('register.loading')" />
    <UiErrorState
      v-else-if="state === 'error'"
      compact
      :title="k('register.loadError')"
      :description="loadError ?? undefined"
      :retry-label="t('common.actions.retry')"
      @retry="load"
    />
    <div v-else-if="detail" class="grid gap-3">
      <p v-if="!detail.owner_emails.length" class="text-sm text-text-muted">{{ k('row.ownerNone') }}</p>
      <template v-else>
        <h3 class="font-condensed text-base font-semibold text-text-secondary">{{ k('owner.recipients') }}</h3>
        <ul class="rounded-md border border-border-subtle">
          <li
            v-for="email in detail.owner_emails"
            :key="email"
            class="border-b border-border-subtle px-3 py-2.5 text-sm font-semibold break-all last:border-b-0"
          >
            {{ email }}
          </li>
        </ul>
      </template>
      <p class="text-[0.8rem] text-text-muted">{{ k('ccEach', { first: SPEAKER_LINKS_CC[0], second: SPEAKER_LINKS_CC[1] }) }}</p>
    </div>
    <UiAlert v-if="sendError" class="mt-3" intent="danger" live compact :message="sendError" />
    <template #footer>
      <UiButton variant="ghost" autofocus :disabled="sending" @click="emit('update:open', false)">{{ k('owner.cancel') }}</UiButton>
      <UiButton :loading="sending" :disabled="!detail?.owner_emails.length" @click="confirm">{{ k('owner.confirm') }}</UiButton>
    </template>
  </UiModal>
</template>
