<script setup lang="ts">
/**
 * ANNULER SON ACTIVITÉ (04/11) : la fenêtre dit ce qui va se passer avant que
 * ça se passe, et exige un motif. Le bouton rouge reste éteint tant que le
 * motif est vide. Le refus de l'API s'affiche tel quel.
 */

interface Props {
  open: boolean
  /** Titre de l'activité. */
  title: string
  /** Le créneau, fuseau compris ; vide tant que l'activité n'est pas programmée. */
  when: string
  busy: boolean
  error: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:open': [value: boolean]; confirm: [reason: string] }>()

const { t } = useI18n()

const reason = ref('')
const ready = computed(() => reason.value.trim().length > 0)

// Chaque ouverture repart d'un motif vide : une annulation ne se prépare pas d'avance.
watch(
  () => props.open,
  (open) => {
    if (open) reason.value = ''
  },
)

function close(): void {
  if (!props.busy) emit('update:open', false)
}
</script>

<template>
  <UiModal
    :open="props.open"
    :title="t('organization.workspace.proposal.cancel.title')"
    :description="[props.title, props.when || t('organization.workspace.proposal.cancel.unscheduled')].join('. ')"
    :dismissible="!props.busy"
    @update:open="close"
  >
    <ul class="grid list-disc gap-1.5 border-l-4 border-danger bg-danger-surface py-3 pr-4 pl-8 text-[0.92rem] text-text">
      <li>{{ t('organization.workspace.proposal.cancel.consequences.programme') }}</li>
      <li>{{ t('organization.workspace.proposal.cancel.consequences.attendees') }}</li>
      <li>{{ t('organization.workspace.proposal.cancel.consequences.team') }}</li>
      <li>
        <b class="font-bold">{{ t('organization.workspace.proposal.cancel.consequences.final') }}</b>
        {{ t('organization.workspace.proposal.cancel.consequences.finalDetail') }}
      </li>
    </ul>

    <UiTextarea
      id="cancel-reason"
      v-model="reason"
      class="mt-4"
      :rows="4"
      :label="t('organization.workspace.proposal.cancel.reason')"
      :hint="t('organization.workspace.proposal.cancel.reasonHint')"
      hide-optional
      :disabled="props.busy"
    />

    <UiAlert v-if="props.error" class="mt-4" intent="danger" :message="props.error" live />

    <template #footer>
      <UiButton variant="secondary" :disabled="props.busy" @click="close">
        {{ t('organization.workspace.proposal.cancel.keep') }}
      </UiButton>
      <UiButton variant="danger" :disabled="!ready" :loading="props.busy" @click="emit('confirm', reason.trim())">
        {{ t('organization.workspace.proposal.cancel.confirm') }}
      </UiButton>
    </template>
  </UiModal>
</template>
