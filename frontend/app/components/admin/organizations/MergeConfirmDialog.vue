<script setup lang="ts">
import type { DuplicateSide } from '~/types/admin-organizations'

/**
 * LA CONFIRMATION DE FUSION.
 *
 * SAISIR LE NOM DE LA FICHE ABSORBÉE — la demande du prompt, et le seul garde-fou
 * de tout le back-office qui exige de recopier quelque chose. Il est là parce
 * qu'une fusion ne s'annule pas d'un clic : la fiche survit, mais ses
 * rattachements sont déplacés et il faudrait les reprendre un à un.
 *
 * C'EST LE NOM DE LA FICHE QUI DISPARAÎT DE LA LISTE, jamais celui de la fiche
 * conservée. Recopier le mauvais nom est le geste de quelqu'un qui a lu l'écran à
 * l'envers, et c'est exactement ce que ce contrôle doit attraper — d'où le rappel
 * des deux noms dans la phrase, et non du seul nom à saisir.
 *
 * LE SIGLE EST ACCEPTÉ. « OSED » désigne la fiche aussi sûrement que son nom
 * légal : c'est la règle métier n° 1 du projet, et la refuser ici la
 * contredirait sur le seul écran qui existe pour la faire respecter. Les accents
 * et la casse n'ont pas d'importance — `platform.normalize_label()` fait le même
 * travail en base.
 *
 * LE MOTIF EST SAISI DANS L'ÉCRAN, PAS ICI. Il appartient à la décision, pas à sa
 * confirmation : le demander dans le dialogue en ferait une formalité qu'on remplit
 * en pensant déjà au bouton.
 */

interface Props {
  open: boolean
  source: DuplicateSide
  target: DuplicateSide
  /** Une fusion est en cours d'envoi. */
  busy?: boolean
  /** Refus rendu par l'API — le nom saisi ne correspondait pas. */
  serverMismatch?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [value: boolean]
  confirm: [typedName: string]
}>()

const { t } = useI18n()

const typed = ref('')
/** L'erreur n'apparaît qu'après une tentative : elle ne blâme pas une saisie en cours. */
const attempted = ref(false)

watch(
  () => props.open,
  (open) => {
    if (open) {
      typed.value = ''
      attempted.value = false
    }
  },
)

const isValid = computed(() => isMergeConfirmationValid(typed.value, props.source))

const errorMessage = computed(() => {
  if (props.serverMismatch) return t('admin.organization.merge.confirm.mismatch')
  if (attempted.value && !isValid.value) return t('admin.organization.merge.confirm.mismatch')
  return undefined
})

function submit(): void {
  attempted.value = true
  if (!isValid.value) return
  emit('confirm', typed.value)
}
</script>

<template>
  <UiModal
    :open="props.open"
    :title="t('admin.organization.merge.confirm.title')"
    :description="
      t('admin.organization.merge.confirm.description', {
        source: props.source.legal_name,
        target: props.target.legal_name,
      })
    "
    size="md"
    @update:open="(value: boolean) => emit('update:open', value)"
  >
    <form @submit.prevent="submit">
      <UiInput
        v-model="typed"
        :label="t('admin.organization.merge.confirm.label')"
        :hint="t('admin.organization.merge.confirm.hint')"
        :error="errorMessage"
        :placeholder="props.source.legal_name"
        autocomplete="off"
        required
        block
      />
    </form>

    <template #footer>
      <UiButton variant="ghost" :disabled="props.busy" @click="emit('update:open', false)">
        {{ t('admin.organization.merge.confirm.cancel') }}
      </UiButton>
      <!-- Le bouton reste ACTIF tant que le nom n'est pas saisi : désactivé, il
           ne dirait pas pourquoi. Cliquer déclenche le message d'erreur, qui lui
           l'explique. -->
      <UiButton variant="danger" :loading="props.busy" @click="submit">
        {{ t('admin.organization.merge.confirm.confirm') }}
      </UiButton>
    </template>
  </UiModal>
</template>
