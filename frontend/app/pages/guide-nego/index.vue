<script setup lang="ts">
import {
  CLE_OUVERTURE_VUE,
  CLE_THEMATIQUES_PROPOSEES,
  lireCle,
  poserCle,
} from '~/utils/guide-nego/stockage'
import { propositionAFaire } from '~/utils/guide-nego/thematiques'

definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const session = useGnSession()
const thematiques = useGnThematiques()

/**
 * **Le choix des thématiques se propose une fois, et ne retient personne.**
 *
 * Qui arrive ici sans rien suivre est envoyé une fois à l'écran de choix, qui
 * porte sa sortie « Plus tard ». La clé se pose au moment de proposer : refuser
 * est aussi une réponse, et redemander à chaque ouverture enfermerait quelqu'un
 * dans une marche qu'il a déjà écartée.
 */
async function proposerLesThematiques(): Promise<void> {
  await session.assurer()
  if (!session.connectee.value) return
  await thematiques.assurer()

  const aFaire = propositionAFaire({
    connectee: session.connectee.value,
    pret: thematiques.pret.value,
    nombreSuivi: thematiques.mesCodes.value.length,
    dejaProposee: lireCle(CLE_THEMATIQUES_PROPOSEES) !== null,
  })
  if (!aFaire) return

  poserCle(CLE_THEMATIQUES_PROPOSEES)
  await navigateTo('/guide-nego/thematiques')
}

// La première venue passe par l'écran d'ouverture ; ensuite, l'accueil s'ouvre seul.
onMounted(() => {
  if (!lireCle(CLE_OUVERTURE_VUE)) return void navigateTo('/guide-nego/ouverture', { replace: true })
  void proposerLesThematiques()
})

useHead({ title: t('guide-nego.accueil.titre') })
</script>

<template>
  <GnEcran :titre="t('guide-nego.accueil.titre')">
    <GnEtatVide
      picto="calendar"
      :titre="t('guide-nego.accueil.vide.titre')"
      :texte="t('guide-nego.accueil.vide.texte')"
    />
  </GnEcran>
</template>
