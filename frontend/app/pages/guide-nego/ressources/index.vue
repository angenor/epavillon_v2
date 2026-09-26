<script setup lang="ts">
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const compte = useGnSession()
const acces = useGnAcces()

// La validation n'apparaît qu'à qui porte la permission (FR-009) ; le compteur est relu à chaque visite.
onMounted(async () => {
  await compte.assurer()
  if (compte.connectee.value) void acces.rafraichir()
})

const peutValider = computed(() => compte.connectee.value && (acces.acces.value.can_validate_reports ?? false))
const aTraiter = computed(() => {
  const n = acces.acces.value.reports_to_review
  return n ? t('guide-nego.ressources.validation-compteur', { count: n }) : undefined
})

useHead({ title: t('guide-nego.ressources.titre') })
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.ressources.titre')"
    :sous-titre="t('guide-nego.ressources.sous-titre')"
    cloche
  >
    <GnLigneReglage
      :libelle="t('guide-nego.ressources.documents')"
      picto="doc"
      vers="/guide-nego/ressources/documents"
    />
    <GnLigneReglage
      :libelle="t('guide-nego.ressources.mes-documents')"
      picto="download"
      vers="/guide-nego/ressources/mes-documents"
    />
    <GnLigneReglage
      :libelle="t('guide-nego.ressources.profil')"
      picto="user"
      vers="/guide-nego/ressources/reglages"
      :derniere="!peutValider"
    />
    <GnLigneReglage
      v-if="peutValider"
      :libelle="t('guide-nego.ressources.validation')"
      :valeur="aTraiter"
      picto="shield-check"
      vers="/guide-nego/validation"
      derniere
    />
  </GnEcran>
</template>
