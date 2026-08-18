<script setup lang="ts">
import type { ProposalStatus } from '~/types/programme/proposal'
import type { DecisionOption } from '~/utils/review-scoring'

/**
 * CONFIRMER UNE DÉCISION — retenir, demander des corrections, rejeter.
 *
 * ELLE DIT CE QU'ELLE FAIT, EN TOUTES LETTRES : « le dossier passe de En
 * évaluation à Non retenu ». Une confirmation qui se contente de « êtes-vous
 * sûr ? » ne rappelle rien à personne, et c'est celle qu'on clique sans lire.
 *
 * LE MOTIF EST EXIGÉ QUAND LA BASE L'EXIGE, et pas selon une règle réécrite
 * ici : `proposal_transitions_allowed.requires_reason` est une colonne, et le
 * trigger `tg_guard_proposal_status()` refuse la transition sans
 * `decision_reason`. Le demander avant l'envoi évite un aller-retour dont on
 * connaît d'avance l'issue — et rappelle que ce texte part à l'organisation.
 */

interface Props {
  open: boolean
  option: DecisionOption | null
  referenceCode: string
  currentStatus: ProposalStatus
  busy?: boolean
  error?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [value: boolean]
  submit: [payload: { toStatus: ProposalStatus; reason: string | null }]
}>()

const { t } = useI18n()

const reason = ref('')
const showReasonError = ref(false)

watch(
  () => props.open,
  (open) => {
    if (open) {
      reason.value = ''
      showReasonError.value = false
    }
  },
)

const actionLabel = computed(() =>
  props.option ? t(`admin.proposal.review.decision.action.${props.option.to_status}`) : '',
)

function submit(): void {
  if (!props.option) return
  if (props.option.requires_reason && !reason.value.trim()) {
    showReasonError.value = true
    return
  }
  emit('submit', { toStatus: props.option.to_status, reason: reason.value.trim() || null })
}
</script>

<template>
  <UiModal
    :open="props.open"
    :title="
      t('admin.proposal.review.decision.confirm.title', {
        action: actionLabel,
        reference: props.referenceCode,
      })
    "
    size="md"
    @update:open="(value: boolean) => emit('update:open', value)"
  >
    <div v-if="props.option" class="space-y-4">
      <UiAlert v-if="props.error" intent="danger" live :message="props.error" />

      <p class="text-text-secondary">
        {{
          t('admin.proposal.review.decision.confirm.description', {
            from: t(`admin.proposal.review.status.${props.currentStatus}`),
            to: t(`admin.proposal.review.status.${props.option.to_status}`),
          })
        }}
      </p>

      <!-- Le motif est proposé dans les deux cas, mais annoncé différemment :
           obligatoire ici, facultatif là. Un champ identique de part et d'autre
           se remplit au hasard ou se saute. -->
      <UiTextarea
        v-model="reason"
        :label="t('admin.proposal.review.decision.confirm.reason')"
        :hint="
          props.option.requires_reason
            ? t('admin.proposal.review.decision.confirm.reasonHint')
            : t('admin.proposal.review.decision.confirm.reasonOptionalHint')
        "
        :error="
          showReasonError && !reason.trim()
            ? t('admin.proposal.review.decision.confirm.reasonRequired')
            : undefined
        "
        :rows="3"
        auto-grow
        block
        :required="props.option.requires_reason"
        :disabled="props.busy"
      />
    </div>

    <template #footer>
      <UiButton variant="ghost" :disabled="props.busy" @click="emit('update:open', false)">
        {{ t('common.actions.cancel') }}
      </UiButton>
      <UiButton
        :variant="
          props.option?.to_status === 'rejected' || props.option?.to_status === 'cancelled'
            ? 'danger'
            : 'primary'
        "
        :loading="props.busy"
        @click="submit()"
      >
        {{ t('admin.proposal.review.decision.confirm.submit') }}
      </UiButton>
    </template>
  </UiModal>
</template>
