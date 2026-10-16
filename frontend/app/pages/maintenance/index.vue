<script setup lang="ts">
/**
 * PAGE D'ATTENTE de la plateforme fermée (drapeau `platform.maintenance`).
 *
 * Atteinte par le middleware global `maintenance`, jamais par un lien. Aucune
 * date de réouverture, comme la page d'un module fermé : une échéance manquée
 * coûte plus qu'un silence.
 *
 * 503 au rendu serveur : un moteur de recherche lit une indisponibilité
 * passagère et garde les pages indexées, là où un 200 le ferait indexer cette
 * page à la place de toutes les autres.
 */
definePageMeta({ layout: 'auth' })

const { t } = useI18n()
const localePath = useLocalePath()
const auth = useAuthStore()

if (import.meta.server) {
  setResponseStatus(useRequestEvent()!, 503)
  useResponseHeader('Retry-After').value = '3600'
}

useHead({
  title: t('maintenance.platform.title'),
  meta: [{ name: 'robots', content: 'noindex' }],
})
</script>

<template>
  <UiMaintenanceState
    :title="t('maintenance.platform.title')"
    :description="t('maintenance.platform.description')"
  >
    <template #actions>
      <p v-if="auth.isAuthenticated" class="text-sm text-text-secondary">
        {{ t('maintenance.platform.signedIn') }}
      </p>
      <UiButton v-else variant="ghost" :to="localePath('auth-login')">
        {{ t('maintenance.platform.adminAccess') }}
      </UiButton>
    </template>
  </UiMaintenanceState>
</template>
