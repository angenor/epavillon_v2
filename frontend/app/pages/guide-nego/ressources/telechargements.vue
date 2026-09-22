<script setup lang="ts">
/**
 * « Mes téléchargements » — ce qui est gardé sur ce téléphone, et la place que
 * Guide Négo y occupe.
 *
 * À cette étape aucun document n'est gardé : ils arrivent avec les documents de
 * négociation. L'écran montre malgré tout la place réelle — la coquille et les
 * données lues comptent déjà (FR-023).
 *
 * **Pas de bouton « Libérer »** : il ne videra que les documents, et il n'y en a
 * pas. Un geste sans effet trompe (principe XII). Il vient à l'étape 1.
 *
 * **Rien ici ne demande de compte** : ce qui est téléchargé vit sur le téléphone, pas
 * sur le compte. L'état « accès refusé » est donc sans objet, et c'est voulu.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const { place, mesuree, mesurer } = useGnPlace()

onMounted(() => void mesurer())

useHead({ title: t('guide-nego.telechargements.titre') })
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.telechargements.titre')"
    :sous-titre="t('guide-nego.telechargements.sous-titre')"
    retour="/guide-nego/ressources/reglages"
    :onglets="false"
  >
    <GnEtatVide
      picto="download"
      :titre="t('guide-nego.telechargements.vide.titre')"
      :texte="t('guide-nego.telechargements.vide.texte')"
    />

    <GnEnteteGroupe :titre="t('guide-nego.telechargements.place.titre')" />
    <div class="gn-telechargements__place">
      <GnChargement v-if="!mesuree" forme="squelette" :lignes="2" :libelle="t('guide-nego.telechargements.place.mesure')" />

      <!-- Le navigateur ne mesure pas : on le dit, jamais un zéro faux (R8). -->
      <GnLigneInformation v-else-if="!place" :texte="t('guide-nego.telechargements.place.impossible')" />

      <template v-else>
        <GnJauge :place="place" />
        <p class="gn-telechargements__aide">{{ t('guide-nego.telechargements.place.aide') }}</p>
      </template>
    </div>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-telechargements__place {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-telechargements__aide {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
  max-width: var(--gn-mesure-lecture);
}
</style>
