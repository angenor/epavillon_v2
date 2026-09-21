<script setup lang="ts">
import { CLE_OUVERTURE_VUE, poserCle } from '~/utils/guide-nego/stockage'

/**
 * Ce que l'application donne sans rien demander, avec un compte, avec un code. Seule
 * la première colonne est ouverte à cette étape : compte et code viennent en 0b.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const { installee } = useGnInstallation()

const SANS_COMPTE = ['documents', 'faq', 'lexique', 'sessions', 'francophonie']
const AVEC_COMPTE = ['thematiques', 'favoris', 'inscriptions']
const AVEC_CODE = ['echanges', 'reserves']

function entrer() {
  poserCle(CLE_OUVERTURE_VUE)
  void navigateTo('/guide-nego')
}

useHead({ title: t('guide-nego.ouverture.titre') })
</script>

<template>
  <GnEcran :titre="t('guide-nego.ouverture.titre')" :sous-titre="t('guide-nego.ouverture.sous-titre')" :onglets="false">
    <GnEnteteGroupe :titre="t('guide-nego.ouverture.sans-compte.titre')" picto="eye" />
    <ul>
      <li v-for="(cle, index) in SANS_COMPTE" :key="cle" class="gn-ouverture__ligne" :class="{ 'gn-ouverture__ligne--derniere': index === SANS_COMPTE.length - 1 }">
        <GnPicto nom="check" :taille="20" class="gn-ouverture__oui" />
        {{ t(`guide-nego.ouverture.sans-compte.${cle}`) }}
      </li>
    </ul>

    <GnEnteteGroupe :titre="t('guide-nego.ouverture.avec-compte.titre')" picto="user" />
    <ul>
      <li v-for="(cle, index) in AVEC_COMPTE" :key="cle" class="gn-ouverture__ligne gn-ouverture__ligne--plus" :class="{ 'gn-ouverture__ligne--derniere': index === AVEC_COMPTE.length - 1 }">
        <GnPicto nom="plus" :taille="20" class="gn-ouverture__accent" />
        {{ t(`guide-nego.ouverture.avec-compte.${cle}`) }}
      </li>
    </ul>

    <GnEnteteGroupe :titre="t('guide-nego.ouverture.avec-code.titre')" picto="lock" />
    <ul>
      <li v-for="(cle, index) in AVEC_CODE" :key="cle" class="gn-ouverture__ligne gn-ouverture__ligne--ferme" :class="{ 'gn-ouverture__ligne--derniere': index === AVEC_CODE.length - 1 }">
        <GnPicto nom="lock" :taille="18" />
        {{ t(`guide-nego.ouverture.avec-code.${cle}`) }}
      </li>
    </ul>

    <div class="gn-ouverture__sorties">
      <GnBouton variante="principal" @clic="entrer">{{ t('guide-nego.ouverture.continuer') }}</GnBouton>
      <GnBouton v-if="!installee" variante="discret" vers="/guide-nego/installer">
        {{ t('guide-nego.ouverture.installer') }}
      </GnBouton>
    </div>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-ouverture__ligne {
  min-height: var(--gn-cible);
  display: flex;
  align-items: center;
  gap: 10px;
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
  font-size: var(--gn-taille-17);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-ouverture__ligne--plus {
  font-weight: var(--gn-graisse-regulier);
}

/* Ce qui demande un code se lit comme fermé : gris, cadenas, sans graisse. */
[data-app="guide-nego"] .gn-ouverture__ligne--ferme {
  color: var(--gn-texte-2);
  font-weight: var(--gn-graisse-regulier);
}

[data-app="guide-nego"] .gn-ouverture__ligne--derniere {
  border-bottom: none;
}

[data-app="guide-nego"] .gn-ouverture__oui {
  color: var(--gn-succes);
}

[data-app="guide-nego"] .gn-ouverture__accent {
  color: var(--gn-accent);
}

[data-app="guide-nego"] .gn-ouverture__sorties {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-entre-blocs);
}
</style>
