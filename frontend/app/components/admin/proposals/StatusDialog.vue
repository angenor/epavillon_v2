<script setup lang="ts">
import type { BulkStatusOption } from '~/types/admin-proposals'
import type { ProposalStatus } from '~/types/programme/proposal'
import type { Uuid } from '~/types/shared'

/**
 * CHANGER LE STATUT D'UNE SÉLECTION.
 *
 * LES TRANSITIONS OFFERTES SONT CELLES QUE LA BASE DÉCLARE —
 * `programme.proposal_transitions_allowed`, lue et non réécrite. Ajouter un
 * chemin en base ajoute une option ici, sans qu'une condition Vue soit touchée.
 *
 * CHAQUE OPTION DIT COMBIEN DE DOSSIERS ELLE TOUCHE VRAIMENT. Une sélection est
 * hétérogène : sur dix dossiers, quatre sont déposés et six déjà en évaluation.
 * « Passer en évaluation (4 sur 10) » évite de croire que six dossiers ont été
 * oubliés par erreur — et évite surtout de refaire l'action trois fois.
 *
 * LE MOTIF EST EXIGÉ QUAND LA BASE L'EXIGE. `requires_reason` est une colonne :
 * le trigger `tg_guard_proposal_status()` refuse la transition sans
 * `decision_reason`, et ce motif part à l'organisation. Le demander ici, c'est
 * éviter un aller-retour dont on connaît d'avance l'issue — et rappeler que ce
 * texte sera lu par quelqu'un. Rejeter ou annuler ne l'exige plus (28/10) : le
 * champ reste offert, et un rappel dit qu'il est vide.
 */

interface Props {
  open: boolean
  count: number
  options: BulkStatusOption[]
  /** Les dossiers retenus de la sélection : ceux dont l'annulation touche le programme public. */
  acceptedIds: Uuid[]
  busy?: boolean
  error?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [value: boolean]
  submit: [payload: { toStatus: ProposalStatus; reason: string | null }]
}>()

const { t } = useI18n()

const target = ref<ProposalStatus | ''>('')
const reason = ref('')
const showReasonError = ref(false)
const impactLoading = ref(false)

watch(
  () => props.open,
  (open) => {
    if (open) {
      target.value = ''
      reason.value = ''
      showReasonError.value = false
    }
  },
)

const selectedOption = computed(() => props.options.find((option) => option.to_status === target.value) ?? null)
const needsReason = computed(() => selectedOption.value?.requires_reason ?? false)
const offersReason = computed(() => needsReason.value || reasonExpected(target.value))
const reasonMissing = computed(() => !needsReason.value && reasonExpected(target.value) && !reason.value.trim())

/**
 * La portée réelle d'une transition n'est dite QUE lorsqu'elle est partielle.
 * « S'applique à 6 des 6 dossiers retenus » fait douter d'une évidence ; « 4 des
 * 20 » est exactement ce qu'il faut savoir avant de cliquer.
 */
const options = computed(() =>
  props.options.map((option) => ({
    value: option.to_status,
    label: t(`admin.proposals.status.${option.to_status}`),
    description:
      option.eligible < props.count
        ? t('admin.proposals.changeStatus.eligible', {
            eligible: option.eligible,
            selected: props.count,
          })
        : undefined,
  })),
)

function submit(): void {
  if (!target.value) return
  if (needsReason.value && !reason.value.trim()) {
    showReasonError.value = true
    return
  }
  emit('submit', { toStatus: target.value, reason: reason.value.trim() || null })
}
</script>

<template>
  <UiModal
    :open="props.open"
    :title="t('admin.proposals.changeStatus.title')"
    :description="t('admin.proposals.changeStatus.description')"
    size="md"
    @update:open="(value: boolean) => emit('update:open', value)"
  >
    <div class="space-y-4">
      <p class="text-sm text-text-secondary">
        {{ t('admin.proposals.selection.count', props.count) }}
      </p>

      <UiAlert v-if="props.error" intent="danger" live :message="props.error" />

      <UiAlert
        v-if="props.options.length === 0"
        intent="info"
        :message="t('admin.proposals.changeStatus.none')"
      />

      <template v-else>
        <UiSelect
          v-model="target"
          :label="t('admin.proposals.changeStatus.target')"
          :placeholder="t('common.actions.select')"
          :options="options"
          required
          block
          :disabled="props.busy"
        />

        <AdminReviewCancellationImpact
          v-if="props.open && target === 'cancelled'"
          :proposal-ids="props.acceptedIds"
          @loading="impactLoading = $event"
        />

        <!-- LE MOTIF N'APPARAÎT QUE QUAND LA BASE L'EXIGE OU QU'IL EST ATTENDU :
             afficher un champ facultatif partout mène à le remplir au hasard ou
             à le laisser vide. -->
        <UiTextarea
          v-if="offersReason"
          v-model="reason"
          :label="t('admin.proposals.changeStatus.reason')"
          :hint="
            needsReason
              ? t('admin.proposals.changeStatus.reasonHint')
              : t('admin.proposals.changeStatus.reasonOptionalHint')
          "
          :error="showReasonError && !reason.trim() ? t('admin.proposals.changeStatus.reasonRequired') : undefined"
          :rows="3"
          :required="needsReason"
          block
          :disabled="props.busy"
        />
        <UiAlert v-if="reasonMissing" intent="warning" :message="t('admin.proposals.changeStatus.reasonMissing')" />
      </template>
    </div>

    <template #footer>
      <UiButton variant="ghost" :disabled="props.busy" @click="emit('update:open', false)">
        {{ t('common.actions.cancel') }}
      </UiButton>
      <UiButton
        variant="primary"
        :loading="props.busy"
        :disabled="!target || props.options.length === 0 || impactLoading"
        @click="submit()"
      >
        {{ t('admin.proposals.changeStatus.submit') }}
      </UiButton>
    </template>
  </UiModal>
</template>
