<script setup lang="ts">
/**
 * Confirmation avant de rejoindre une organisation.
 *
 * DEUX CHOSES SE JOUENT ICI, ET UNE SEULE EST UN CHAMP DE SAISIE.
 *
 * La première est la FONCTION occupée (`memberships.job_title`) : demandée
 * maintenant ou jamais. Personne ne revient la renseigner depuis son profil, et
 * c'est pourtant ce que lira le référent qui accepte la demande — « Chargée de
 * plaidoyer climat » se valide seul, une demande anonyme s'enlise. Facultative
 * malgré tout : elle ne doit pas retenir quelqu'un qui veut déposer un dossier.
 *
 * La seconde est de DIRE CE QUI VA SE PASSER. Un rattachement par domaine
 * vérifié est immédiat ; partout ailleurs, un référent doit accepter, et la
 * personne restera sans organisation d'ici là. L'annoncer avant le clic évite la
 * question qui suit toujours — « c'est validé ? ».
 */

interface Props {
  open: boolean
  organizationName: string
  /** Le rattachement sera-t-il immédiat (domaine vérifié, `auto_join`) ? */
  immediate?: boolean
  submitting?: boolean
  /** Fonction déjà déclarée au profil, proposée par défaut. */
  defaultJobTitle?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [value: boolean]
  confirm: [jobTitle: string | null]
}>()

const { t } = useI18n()

const jobTitle = ref(props.defaultJobTitle ?? '')

// Le champ se recharge à chaque ouverture : la fiche visée a pu changer entre
// deux ouvertures, et une valeur restée d'un essai précédent serait trompeuse.
watch(
  () => props.open,
  (open) => {
    if (open) jobTitle.value = props.defaultJobTitle ?? ''
  },
)
</script>

<template>
  <UiModal
    :open="props.open"
    size="sm"
    :title="t('organization.join.confirm.title', { organization: props.organizationName })"
    :description="props.immediate
      ? t('organization.join.confirm.immediate')
      : t('organization.join.confirm.approval')"
    @update:open="emit('update:open', $event)"
  >
    <UiInput
      v-model="jobTitle"
      :label="t('organization.join.confirm.jobTitle')"
      :hint="t('organization.join.confirm.jobTitleHint')"
      :disabled="props.submitting"
      :maxlength="120"
    />

    <template #footer>
      <UiButton
        variant="ghost"
        :label="t('common.actions.cancel')"
        :disabled="props.submitting"
        @click="emit('update:open', false)"
      />
      <UiButton
        variant="primary"
        :loading="props.submitting"
        :label="props.immediate
          ? t('organization.join.confirm.submitImmediate')
          : t('organization.join.confirm.submitRequest')"
        @click="emit('confirm', jobTitle.trim() || null)"
      />
    </template>
  </UiModal>
</template>
