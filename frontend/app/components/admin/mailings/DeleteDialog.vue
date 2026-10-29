<script setup lang="ts">
import type { MailingStatus } from '~/types/admin-mailings'
import type { Uuid } from '~/types/shared'

/**
 * Confirmer la suppression d'un envoi, depuis l'historique comme depuis sa
 * fiche. Un envoi en cours n'est jamais proposé ici : l'API le refuse, on
 * l'arrête d'abord.
 */
interface Target {
  id: Uuid
  subject: string
  status: MailingStatus
}

const props = defineProps<{ target: Target | null }>()
const emit = defineEmits<{ close: []; deleted: [id: Uuid] }>()

const { t } = useI18n()
const api = useApi()

const busy = ref(false)
const error = ref<string | null>(null)

watch(
  () => props.target?.id,
  () => {
    error.value = null
  },
)

const open = computed({
  get: () => props.target !== null,
  set: (value: boolean) => {
    if (!value && !busy.value) emit('close')
  },
})

async function confirm(): Promise<void> {
  const target = props.target
  if (!target) return
  busy.value = true
  error.value = null
  try {
    await api.adminMailings.remove(target.id)
    emit('deleted', target.id)
  } catch (thrown) {
    error.value = apiErrorMessage(thrown, t)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <UiModal
    v-model:open="open"
    :title="t('admin.mailing.list.delete.title')"
    :description="props.target?.status === 'draft' ? t('admin.mailing.list.delete.draft') : t('admin.mailing.list.delete.sent')"
  >
    <p v-if="props.target" class="font-semibold wrap-break-word">
      {{ props.target.subject || t('admin.mailing.common.noSubject') }}
    </p>
    <UiAlert v-if="error" class="mt-4" intent="danger" live :message="error" />
    <template #footer>
      <UiButton variant="secondary" :disabled="busy" @click="emit('close')">
        {{ t('admin.mailing.common.back') }}
      </UiButton>
      <UiButton variant="danger" icon="trash" :loading="busy" @click="confirm">
        {{ t('admin.mailing.list.delete.confirm') }}
      </UiButton>
    </template>
  </UiModal>
</template>
