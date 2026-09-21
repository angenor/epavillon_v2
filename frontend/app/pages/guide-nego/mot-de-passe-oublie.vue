<script setup lang="ts">
/**
 * Le même formulaire que la feuille basse de l'écran de connexion, en écran
 * plein — pour qui arrive par un lien direct plutôt que par la connexion.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()

useHead({ title: t('guide-nego.mot-de-passe-oublie.titre') })
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.mot-de-passe-oublie.titre')"
    retour="/guide-nego/connexion"
    :onglets="false"
  >
    <GnMotDePasseOublie @annuler="navigateTo('/guide-nego/connexion')" />
  </GnEcran>
</template>
