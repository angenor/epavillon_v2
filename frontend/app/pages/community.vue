<script setup lang="ts">
/**
 * COMMUNAUTÉ — l'espace des organisations, des personnes et des échanges.
 *
 * FERMÉ DANS CE JALON, et la page existe quand même : la barre de navigation la
 * propose, et une entrée de menu qui répond 404 est pire que pas d'entrée du
 * tout — c'est la leçon du prompt A3. Elle affiche donc l'état de maintenance,
 * qui dit ce qui se prépare plutôt que de laisser croire à une panne.
 *
 * Ce que l'espace portera vit déjà dans le modèle : `org.organizations` et son
 * annuaire, `identity.people` avec `is_directory_visible`, et le schéma
 * `engagement` pour les échanges. Rien à inventer le jour où le drapeau ouvre.
 */

definePageMeta({ layout: 'public' })
defineI18nRoute({ paths: { fr: '/communaute', en: '/community' } })

const { t } = useI18n()

useHead({ title: t('nav.main.community') })
</script>

<template>
  <UiMaintenanceState :title="t('nav.main.community')" :description="t('nav.maintenance.spaces.community')" />
</template>
