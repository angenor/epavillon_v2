<script setup lang="ts">
import type { ExtractionState } from '~/types/admin-negotiation-documents'

// L'interrupteur montre ce que reçoit le téléphone ; le toucher pose un choix
// explicite, que « Revenir au verdict » efface (`null`).
const props = defineProps<{
  extraction: ExtractionState
  canPublish: boolean
  saving?: boolean
}>()

const emit = defineEmits<{ 'update:choice': [choice: boolean | null] }>()

const { t } = useI18n()

const sansTexte = computed(() => !props.extraction.has_text)
const choixPose = computed(() => props.extraction.large_text_choice !== null)
const verdict = computed(() => props.extraction.is_reflowable === true)

const aide = computed(() => {
  if (sansTexte.value) return t('admin-large-text-choice.noText')
  if (!props.canPublish) return t('admin-large-text-choice.readOnly')
  const parDefaut = t(verdict.value ? 'admin-large-text-choice.byVerdictOn' : 'admin-large-text-choice.byVerdictOff')
  return choixPose.value ? `${t('admin-large-text-choice.chosen')} ${parDefaut}` : parDefaut
})
</script>

<template>
  <div class="space-y-2">
    <UiSwitch
      :model-value="props.extraction.large_text"
      :label="t('admin-large-text-choice.label')"
      :hint="aide"
      :disabled="!props.canPublish || sansTexte"
      :loading="props.saving"
      @update:model-value="(valeur: boolean) => emit('update:choice', valeur)"
    />
    <UiButton
      v-if="props.canPublish && choixPose && !sansTexte"
      variant="ghost"
      size="sm"
      :disabled="props.saving"
      @click="emit('update:choice', null)"
    >
      {{ t('admin-large-text-choice.backToVerdict') }}
    </UiButton>
  </div>
</template>
