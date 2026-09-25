<script setup lang="ts">
/**
 * Section 5 — les composants livrés à l'étape 0a, dans toutes leurs variantes.
 *
 * La section est découpée en lots — cadre, commandes, saisie, surfaces, documents, lecteur du PDF — pour
 * qu'une étape ajoute les siens sans rouvrir les autres, et pour qu'aucun fichier
 * n'approche la limite de mille lignes. Les classes communes aux lots vivent ici.
 *
 * Rien n'est dessiné à la main : ce qui n'est pas livré n'est pas montré, et la note
 * finale dit lesquels manquent. Une planche qui simule un composant ment deux fois —
 * sur son existence, et sur son dessin.
 */
const { t } = useI18n()
</script>

<template>
  <GnPlancheSection
    numero="5"
    :titre="t('gn-planche-composants.titre')"
    :propos="t('gn-planche-composants.propos')"
  >
    <div class="gn-planche-composants">
      <p class="gn-planche-note">{{ t('gn-planche-composants.etats-communs') }}</p>
      <p class="gn-planche-note">{{ t('gn-planche-composants.interactif') }}</p>

      <GnPlancheComposantsCadre />
      <GnPlancheComposantsCommandes />
      <GnPlancheComposantsSaisie />
      <GnPlancheComposantsSurfaces />
      <GnPlancheComposantsDocuments />
      <GnPlancheComposantsLecteurPdf />
      <GnPlancheComposantsSessions />

      <GnPlancheSection :titre="t('gn-planche-composants.absents')">
        <p class="gn-planche-note">{{ t('gn-planche-composants.absents-propos') }}</p>
      </GnPlancheSection>
    </div>
  </GnPlancheSection>
</template>

<style>
[data-app="guide-nego"] .gn-planche-composants,
[data-app="guide-nego"] .gn-planche-composants__lot {
  display: flex;
  flex-direction: column;
  gap: var(--gn-entre-blocs);
}

/* Le cadre d'un composant : il rend visible le bord d'un dessin qui n'en a pas. */
[data-app="guide-nego"] .gn-planche-composants__vitrine {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
  padding: var(--gn-espace-12);
  border: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-planche-composants__rangee {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--gn-espace-8);
}

/* Deux demi-largeurs : `flex: 1` n'a de sens que dans une rangée qui ne se replie pas. */
[data-app="guide-nego"] .gn-planche-composants__paire {
  display: flex;
  gap: var(--gn-entre-cibles);
}

[data-app="guide-nego"] .gn-planche-composants__legende {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  font-weight: var(--gn-graisse-demi-gras);
}

/* Clair et sombre côte à côte. Le conteneur porte `data-app` pour que le sombre s'y
   applique, et hérite donc du cadre de l'application, que ces déclarations reprennent. */
[data-app="guide-nego"] .gn-planche-composants__themes {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-planche-composants__theme {
  min-height: 0;
  width: auto;
  overflow-x: visible;
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding: var(--gn-espace-12);
  border: var(--gn-filet-1) solid var(--gn-filet);
}

/* Le cadre de référence : 360 px, largeur sur laquelle les mesures ont été arrêtées. */
[data-app="guide-nego"] .gn-planche-composants__cadre {
  width: min(100%, var(--gn-cadre-largeur));
}
</style>
