<script setup lang="ts">
/**
 * Section 5, premier lot : ce qui tient l'écran — barre d'onglets, en-tête, bandeau et
 * ligne de connexion, en-tête de groupe, ligne d'information, étiquettes.
 *
 * L'heure de lecture est figée à l'ouverture de la planche : recalculée à chaque rendu,
 * elle sauterait d'une minute sous les yeux et ferait douter du composant.
 */
const { t } = useI18n()

const luA = new Date(Date.now() - 12 * 60 * 1000).toISOString()

const COMPTEURS = { echanges: 3 }
</script>

<template>
  <div class="gn-planche-composants__lot">
    <GnPlancheSection
      :titre="t('gn-planche-composants-cadre.onglets')"
      :propos="t('gn-planche-composants-cadre.onglets-propos')"
    >
      <p class="gn-planche-composants__legende">{{ t('gn-planche-composants-cadre.onglets-quatre') }}</p>
      <div class="gn-planche-composants__cadre gn-planche-cadre-onglets">
        <GnBarreOnglets forcer-actif="negociations" />
      </div>

      <p class="gn-planche-composants__legende">{{ t('gn-planche-composants-cadre.onglets-cinq') }}</p>
      <div class="gn-planche-composants__cadre gn-planche-cadre-onglets">
        <GnBarreOnglets echanges-ouverts :compteurs="COMPTEURS" forcer-actif="echanges" />
      </div>

      <p class="gn-planche-note">{{ t('gn-planche-composants-cadre.onglets-largeur') }}</p>
      <p class="gn-planche-note">{{ t('gn-planche-composants-cadre.onglets-actif') }}</p>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-cadre.entete')"
      :propos="t('gn-planche-composants-cadre.entete-propos')"
    >
      <p class="gn-planche-composants__legende">{{ t('gn-planche-composants-cadre.entete-principal') }}</p>
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnEntete
          :titre="t('gn-planche-composants-cadre.entete-titre')"
          :sous-titre="t('gn-planche-composants-cadre.entete-sous-titre')"
        >
          <template #connexion>
            <GnLigneConnexion :en-ligne="true" :lu-a="luA" />
          </template>
        </GnEntete>
      </div>

      <p class="gn-planche-composants__legende">{{ t('gn-planche-composants-cadre.entete-secondaire') }}</p>
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnEntete
          retour="/guide-nego"
          :titre="t('gn-planche-composants-cadre.entete-retour-titre')"
        >
          <template #connexion>
            <GnLigneConnexion :en-ligne="false" :lu-a="luA" />
          </template>
        </GnEntete>
      </div>

      <p class="gn-planche-composants__legende">{{ t('gn-planche-composants-cadre.entete-lexique') }}</p>
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnEntete lexique-ouvert :titre="t('gn-planche-composants-cadre.entete-lexique-titre')">
          <template #connexion>
            <GnLigneConnexion :en-ligne="true" :lu-a="null" />
          </template>
        </GnEntete>
      </div>

      <p class="gn-planche-note">{{ t('gn-planche-composants-cadre.entete-note') }}</p>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-cadre.connexion')"
      :propos="t('gn-planche-composants-cadre.connexion-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <span class="gn-planche-composants__legende">{{ t('gn-planche-composants-cadre.bandeau-avec') }}</span>
        <GnBandeauConnexion :lu-a="luA" />
        <span class="gn-planche-composants__legende">{{ t('gn-planche-composants-cadre.bandeau-sans') }}</span>
        <GnBandeauConnexion :lu-a="null" />
        <span class="gn-planche-composants__legende">{{ t('gn-planche-composants-cadre.ligne-en-ligne') }}</span>
        <GnLigneConnexion :en-ligne="true" :lu-a="luA" />
        <span class="gn-planche-composants__legende">{{ t('gn-planche-composants-cadre.ligne-hors') }}</span>
        <GnLigneConnexion :en-ligne="false" :lu-a="luA" />
        <span class="gn-planche-composants__legende">{{ t('gn-planche-composants-cadre.ligne-hors-sans') }}</span>
        <GnLigneConnexion :en-ligne="false" :lu-a="null" />
      </div>
      <p class="gn-planche-note">{{ t('gn-planche-composants-cadre.connexion-note') }}</p>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-cadre.groupe')"
      :propos="t('gn-planche-composants-cadre.groupe-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnEnteteGroupe :titre="t('gn-planche-composants-cadre.groupe-matin')" :compteur="7" />
        <GnEnteteGroupe :titre="t('gn-planche-composants-cadre.groupe-epingle')" picto="pin" />
        <GnEnteteGroupe :titre="t('gn-planche-composants-cadre.groupe-nu')" />
      </div>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-cadre.info')"
      :propos="t('gn-planche-composants-cadre.info-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnLigneInformation :texte="t('gn-planche-composants-cadre.info-seule')" />
        <GnLigneInformation
          :texte="t('gn-planche-composants-cadre.info-avec')"
          :sortie="t('gn-planche-composants-cadre.info-sortie')"
          vers="/guide-nego"
        />
      </div>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-cadre.etiquette')"
      :propos="t('gn-planche-composants-cadre.etiquette-propos')"
    >
      <div class="gn-planche-composants__rangee">
        <GnEtiquette :texte="t('gn-planche-composants-cadre.etiquette-source')" picto="check-circle" />
        <GnEtiquette :texte="t('gn-planche-composants-cadre.etiquette-traduction')" picto="translate" />
        <GnEtiquette :texte="t('gn-planche-composants-cadre.etiquette-nue')" />
      </div>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-cadre.ecran')"
      :propos="t('gn-planche-composants-cadre.ecran-propos')"
    >
      <p class="gn-planche-note">{{ t('gn-planche-composants-cadre.ecran-note') }}</p>
    </GnPlancheSection>
  </div>
</template>

<style>
/* La barre d'onglets est en position fixe : sans ancêtre transformé, elle se collerait
   au bas de la fenêtre au lieu de rester dans sa vitrine. `translateZ(0)` fait du cadre
   son bloc conteneur, et la barre s'y centre comme elle le ferait dans l'application. */
[data-app="guide-nego"] .gn-planche-cadre-onglets {
  position: relative;
  transform: translateZ(0);
  height: var(--gn-barre-onglets);
  border: var(--gn-filet-1) solid var(--gn-filet);
}
</style>
