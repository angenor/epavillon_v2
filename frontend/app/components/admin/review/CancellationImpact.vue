<script setup lang="ts">
import type { CancellationImpact } from '~/types/admin-review'
import type { Uuid } from '~/types/shared'

/**
 * Avant d'annuler des dossiers retenus : ce qui disparaîtra du programme public
 * et combien d'inscrits en seront prévenus. Rien de publié, rien d'affiché.
 */

interface Props {
  proposalIds: Uuid[]
}

const props = defineProps<Props>()

const { t } = useI18n()
const api = useApi()

const impact = ref<CancellationImpact | null>(null)
const loadError = ref<string | null>(null)
let request = 0

async function load(ids: Uuid[]): Promise<void> {
  const current = ++request
  impact.value = null
  loadError.value = null
  if (ids.length === 0) return
  try {
    const impacts = await Promise.all(ids.map((id) => api.review.cancellationImpact(id)))
    if (current === request) impact.value = sumCancellationImpacts(impacts)
  } catch (failure) {
    if (current === request) loadError.value = apiErrorMessage(failure, t)
  }
}

watch(() => props.proposalIds.join(','), () => void load(props.proposalIds), { immediate: true })

const message = computed(() => {
  const shown = impact.value
  if (!touchesPublicProgramme(shown)) return ''
  const sessions = t('admin-cancellation-impact.sessions', shown.published_sessions)
  return shown.registrants > 0
    ? t('admin-cancellation-impact.summary', {
        sessions,
        registrants: t('admin-cancellation-impact.registrants', shown.registrants),
      })
    : t('admin-cancellation-impact.summaryAlone', { sessions })
})
</script>

<template>
  <UiAlert v-if="loadError" intent="warning" live :message="loadError" />
  <UiAlert v-else-if="message" intent="warning" live :title="t('admin-cancellation-impact.title')">
    <p>{{ message }}</p>
    <p class="mt-1">{{ t('admin-cancellation-impact.reason') }}</p>
  </UiAlert>
</template>
