<script setup lang="ts">
import type { PlatformMaintenance } from '~/types/platform'

// Seuls les administrateurs en portée globale la voient : l'API refuse les autres
// (403), et le composant se tait alors plutôt que d'afficher un refus.
const { t } = useI18n()
const api = useApi()
const features = useFeatureStore()

const { data: state, error: loadError } = await useAsyncData<PlatformMaintenance | null>(
  'admin-platform-maintenance',
  () => api.platform.maintenance(),
  { default: () => null },
)

const isHidden = computed(() => isForbiddenError(loadError.value))
const isConfirmOpen = ref(false)
const isSaving = ref(false)
const saveError = ref<string | null>(null)

function onToggle(next: boolean): void {
  saveError.value = null
  if (next) isConfirmOpen.value = true
  else void apply(false)
}

async function apply(enabled: boolean): Promise<void> {
  isSaving.value = true
  saveError.value = null
  try {
    state.value = await api.platform.setMaintenance({ enabled })
    isConfirmOpen.value = false
    await features.reload()
  } catch (error) {
    saveError.value = apiErrorMessage(error, t)
  } finally {
    isSaving.value = false
  }
}
</script>

<template>
  <UiCard v-if="!isHidden && (state || loadError)" class="mt-8">
    <UiAlert v-if="loadError" intent="danger" compact :message="apiErrorMessage(loadError, t)" />
    <template v-else-if="state">
      <UiSwitch
        :model-value="state.enabled"
        :label="t('admin-maintenance-switch.label')"
        :hint="t(state.enabled ? 'admin-maintenance-switch.hintOn' : 'admin-maintenance-switch.hintOff')"
        :loading="isSaving && !isConfirmOpen"
        label-position="start"
        @update:model-value="onToggle"
      />
      <UiAlert v-if="saveError && !isConfirmOpen" class="mt-3" intent="danger" live :message="saveError" />
    </template>

    <UiModal
      v-model:open="isConfirmOpen"
      :title="t('admin-maintenance-switch.confirm.title')"
      :description="t('admin-maintenance-switch.confirm.description')"
    >
      <div class="space-y-4">
        <UiAlert v-if="saveError" intent="danger" live :message="saveError" />
        <div class="flex flex-wrap justify-end gap-3">
          <UiButton variant="ghost" type="button" @click="isConfirmOpen = false">
            {{ t('common.actions.cancel') }}
          </UiButton>
          <UiButton variant="danger" type="button" :loading="isSaving" @click="apply(true)">
            {{ t('admin-maintenance-switch.confirm.submit') }}
          </UiButton>
        </div>
      </div>
    </UiModal>
  </UiCard>
</template>
