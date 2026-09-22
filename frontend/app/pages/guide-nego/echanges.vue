<script setup lang="ts">
/**
 * Les Échanges — **le premier module réservé**, et donc le premier verrou.
 *
 * DEUX PORTES, ET ELLES NE DISENT PAS LA MÊME CHOSE. Le drapeau
 * `negotiation.channels` décide si le module EXISTE : éteint, l'onglet disparaît
 * et l'adresse forgée renvoie à l'accueil. L'accès, lui, décide si son contenu
 * s'ouvre : sans lui, le module existe, se voit, et dit ce qu'il contient — le
 * verrou n'est pas une porte fermée, c'est une invitation.
 *
 * LE VERROU SUIT `me/access`, jamais un état retenu côté client (FR-032), et sa
 * sortie suit le mode d'admission — le code, la demande, ou le retour à une
 * demande en attente. Rien de cela n'est décidé ici : `GnVerrou` s'en charge,
 * pour que le prochain module réservé n'ait pas à le refaire.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const { echangesOuverts, pret } = useGnDrapeaux()
const acces = useGnAcces()

// L'onglet disparaît avec le drapeau ; son adresse, forgée ou gardée en favori, doit
// disparaître avec lui.
watchEffect(() => {
  if (pret.value && !echangesOuverts.value) void navigateTo('/guide-nego', { replace: true })
})

/** Ce que le verrou énumère : trois lignes, celles de la page 12 de la maquette. */
const contenus = computed(() => [
  t('guide-nego.echanges.verrou.canaux'),
  t('guide-nego.echanges.verrou.annuaire'),
  t('guide-nego.echanges.verrou.experts'),
])

useHead({ title: t('guide-nego.echanges.titre') })
</script>

<template>
  <GnEcran :titre="t('guide-nego.echanges.titre')" :sous-titre="t('guide-nego.echanges.sous-titre')">
    <!-- Sans accès, le verrou remplace le contenu — titre et sous-titre de
         l'écran restent visibles, comme la maquette les garde. -->
    <GnVerrou
      v-if="!acces.ouvert.value"
      :propos="t('guide-nego.echanges.verrou.propos')"
      :contenus="contenus"
      :reste-ouvert="t('guide-nego.echanges.verrou.reste-ouvert')"
    />

    <template v-else>
      <GnEtatVide
        picto="chat"
        :titre="t('guide-nego.echanges.vide.titre')"
        :texte="t('guide-nego.echanges.vide.texte')"
      />
      <p class="gn-echanges__mention">{{ t('guide-nego.echanges.vide.mention') }}</p>
    </template>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-echanges__mention {
  padding: var(--gn-espace-12) var(--gn-espace-16);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  text-align: center;
}
</style>
