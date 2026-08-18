<script setup lang="ts">
/**
 * SE DÉPORTER, EN DÉCLARANT SON LIEN.
 *
 * LE MOTIF N'EST PAS UNE FORMALITÉ : c'est LA raison d'être du geste. La colonne
 * `review_assignments.recusal_reason` existe pour que l'impartialité du comité
 * se relise — six mois plus tard, quand une organisation conteste une décision,
 * « Mme X s'est retirée » ne vaut rien à côté de « Mme X s'est retirée :
 * collaboration en cours avec l'organisation porteuse ».
 *
 * LE DÉPORT N'EFFACE PAS L'AFFECTATION, il la date et la motive. La déclaration
 * reste visible du comité et de l'écran d'affectation ; c'est ce qui distingue
 * un retrait déclaré d'un dossier simplement jamais évalué.
 */

interface Props {
  open: boolean
  busy?: boolean
  error?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [value: boolean]
  submit: [reason: string]
}>()

const { t } = useI18n()

const reason = ref('')
const showError = ref(false)

watch(
  () => props.open,
  (open) => {
    if (open) {
      reason.value = ''
      showError.value = false
    }
  },
)

function submit(): void {
  if (!reason.value.trim()) {
    showError.value = true
    return
  }
  emit('submit', reason.value.trim())
}
</script>

<template>
  <UiModal
    :open="props.open"
    :title="t('admin.proposal.review.recusal.title')"
    :description="t('admin.proposal.review.recusal.description')"
    size="md"
    @update:open="(value: boolean) => emit('update:open', value)"
  >
    <div class="space-y-4">
      <UiAlert v-if="props.error" intent="danger" live :message="props.error" />

      <UiTextarea
        v-model="reason"
        :label="t('admin.proposal.review.recusal.reason')"
        :hint="t('admin.proposal.review.recusal.reasonHint')"
        :error="showError && !reason.trim() ? t('admin.proposal.review.recusal.reasonRequired') : undefined"
        :rows="3"
        auto-grow
        block
        required
        :disabled="props.busy"
      />
    </div>

    <template #footer>
      <UiButton variant="ghost" :disabled="props.busy" @click="emit('update:open', false)">
        {{ t('common.actions.cancel') }}
      </UiButton>
      <UiButton variant="danger" :loading="props.busy" @click="submit()">
        {{ t('admin.proposal.review.recusal.submit') }}
      </UiButton>
    </template>
  </UiModal>
</template>
