<script setup lang="ts">
import type { ProposalFacet } from '~/types/admin-proposals'
import type { TimeZoneName } from '~/types/shared'

/**
 * AFFECTER UNE SÉLECTION À UN MEMBRE DU COMITÉ.
 *
 * LA CHARGE DE CHACUN EST AFFICHÉE À CÔTÉ DE SON NOM, et ce n'est pas décoratif :
 * répartir sans voir qui porte déjà vingt dossiers, c'est refaire le déséquilibre
 * que `event.call_reviewers.workload_cap` existe pour éviter. Le choix reste
 * humain — l'écran informe, il n'arbitre pas.
 *
 * L'ÉCHÉANCE EST FACULTATIVE ET PORTE SON FUSEAU. `review_assignments.due_at`
 * accepte l'absence : toute revue n'a pas de date. Mais une date affichée sans
 * fuseau, sur une plateforme dont les équipes sont à Québec, Dakar et Belém,
 * est une date fausse pour deux personnes sur trois.
 *
 * LE COMITÉ VIENT DE L'APPEL (`event.call_reviewers`), pas du rôle : le rôle
 * `reviewer` dit qu'une personne peut évaluer, cette table dit qui SIÈGE sur
 * cet appel. Une édition sans comité constitué le dit franchement, plutôt que
 * d'offrir une liste vide.
 */

interface Props {
  open: boolean
  /** Nombre de dossiers retenus. */
  count: number
  /** Membres du comité, avec leur charge actuelle dans `count`. */
  committee: ProposalFacet[]
  timezone: TimeZoneName
  /** Nom de ville qui NOMME le fuseau — « heure de Belém ». */
  zoneLabel: string
  busy?: boolean
  /** Message d'échec de la dernière tentative. */
  error?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [value: boolean]
  submit: [payload: { reviewerId: string; dueAt: string | null }]
}>()

const { t } = useI18n()

const reviewerId = ref('')
const dueDate = ref('')

// Rouvrir le dialogue repart d'une feuille blanche : garder le choix précédent
// ferait affecter à la mauvaise personne au deuxième usage.
watch(
  () => props.open,
  (open) => {
    if (open) {
      reviewerId.value = ''
      dueDate.value = ''
    }
  },
)

const options = computed(() =>
  props.committee.map((member) => ({
    value: member.value,
    label: typeof member.label === 'string' ? member.label : member.value,
    description: t('admin.proposals.assign.workload', member.count),
  })),
)

function submit(): void {
  if (!reviewerId.value) return
  emit('submit', {
    reviewerId: reviewerId.value,
    // La date saisie est une date MURALE dans le fuseau de l'édition. Elle est
    // transmise telle quelle ; c'est l'API qui l'ancre — le navigateur d'un
    // membre du comité à Québec ne doit pas décider de l'heure d'une échéance
    // fixée à Belém.
    dueAt: dueDate.value ? `${dueDate.value}T23:59` : null,
  })
}
</script>

<template>
  <UiModal
    :open="props.open"
    :title="t('admin.proposals.assign.title')"
    :description="t('admin.proposals.assign.description')"
    size="md"
    @update:open="(value: boolean) => emit('update:open', value)"
  >
    <div class="space-y-4">
      <p class="text-sm text-text-secondary">
        {{ t('admin.proposals.selection.count', props.count) }}
      </p>

      <UiAlert v-if="props.error" intent="danger" live :message="props.error" />

      <UiAlert
        v-if="props.committee.length === 0"
        intent="warning"
        :message="t('admin.proposals.assign.noCommittee')"
      />

      <template v-else>
        <UiSelect
          v-model="reviewerId"
          :label="t('admin.proposals.assign.reviewer')"
          :placeholder="t('admin.proposals.assign.reviewerPlaceholder')"
          :options="options"
          required
          block
          :disabled="props.busy"
        />

        <UiDatePicker
          v-model="dueDate"
          :label="t('admin.proposals.assign.dueAt')"
          :hint="t('admin.proposals.assign.dueAtHint', { zone: props.zoneLabel })"
          :timezone-label="props.zoneLabel"
          block
          :disabled="props.busy"
        />
      </template>
    </div>

    <template #footer>
      <UiButton variant="ghost" :disabled="props.busy" @click="emit('update:open', false)">
        {{ t('common.actions.cancel') }}
      </UiButton>
      <UiButton
        variant="primary"
        :loading="props.busy"
        :disabled="!reviewerId || props.committee.length === 0"
        @click="submit()"
      >
        {{ t('admin.proposals.assign.submit') }}
      </UiButton>
    </template>
  </UiModal>
</template>
