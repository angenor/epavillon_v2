<script setup lang="ts">
/**
 * NÉGOCIATIONS — l'espace réservé aux négociateurs francophones.
 *
 * FERMÉ DANS CE JALON. Le module existe dans le modèle (`100_negotiations.sql` :
 * espaces, réunions, documents, canaux) mais son interface n'est pas ouverte, et
 * `platform.feature_flags` ne sème que `negotiation.channels` — les seuls canaux
 * d'échange, pas l'espace entier. La page annonce donc l'état plutôt que de
 * laisser un lien mort dans la barre de navigation.
 *
 * ACCÈS RESTREINT, le jour venu : l'espace n'est pas public, il se teste par
 * permission et par portée, jamais par nom de rôle.
 */

definePageMeta({ layout: 'public' })
defineI18nRoute({ paths: { fr: '/negociations', en: '/negotiations' } })

const { t } = useI18n()

useHead({ title: t('nav.main.negotiations') })
</script>

<template>
  <UiMaintenanceState
    :title="t('nav.main.negotiations')"
    :description="t('maintenance.modules.negotiation.description')"
  />
</template>
