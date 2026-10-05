<script setup lang="ts">
import type { NomDePicto } from '~/utils/guide-nego/pictogrammes'

/**
 * « Une journée à votre mesure » (Nuit 04) : l'accueil sans compte invite, il ne bloque
 * rien. La connexion est dans l'en-tête ; ici, la création du compte.
 */
const { t } = useI18n()

const AVANTAGES: { cle: string; picto: NomDePicto }[] = [
  { cle: 'thematiques', picto: 'filter' },
  { cle: 'alerte', picto: 'bell' },
  { cle: 'documents', picto: 'download' },
]
</script>

<template>
  <section class="gn-invitation">
    <div class="gn-invitation__tete">
      <h2 class="gn-invitation__titre">{{ t('gn-journee-invitation.titre') }}</h2>
      <i18n-t keypath="gn-journee-invitation.texte" tag="p" class="gn-invitation__texte" scope="global">
        <template #vos>
          <em class="gn-invitation__appui">{{ t('gn-journee-invitation.vos') }}</em>
        </template>
      </i18n-t>
    </div>
    <ul role="list" class="gn-invitation__liste">
      <li v-for="a in AVANTAGES" :key="a.cle" class="gn-invitation__avantage">
        <span class="gn-invitation__pastille"><GnPicto :nom="a.picto" :taille="18" /></span>
        {{ t(`gn-journee-invitation.avantages.${a.cle}`) }}
      </li>
    </ul>
    <div class="gn-invitation__sortie">
      <GnBouton variante="principal" vers="/guide-nego/compte">{{ t('gn-journee-invitation.creer') }}</GnBouton>
      <p class="gn-invitation__note">{{ t('gn-journee-invitation.ouvert') }}</p>
    </div>
  </section>
</template>

<style>
[data-app="guide-nego"] .gn-invitation {
  padding: var(--gn-espace-20);
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-16);
  background: var(--gn-fond-2);
  border: var(--gn-filet-1) solid var(--gn-filet);
  border-radius: var(--gn-rayon-24);
}

[data-app="guide-nego"] .gn-invitation__tete {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

[data-app="guide-nego"] .gn-invitation__titre {
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-20);
  line-height: 1.2;
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-invitation__texte {
  color: var(--gn-texte-lecture);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-invitation__appui {
  font-style: normal;
  color: var(--gn-texte);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-invitation__liste {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-invitation__avantage {
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-invitation__pastille {
  flex: none;
  width: var(--gn-pastille-icone-petite);
  height: var(--gn-pastille-icone-petite);
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--gn-rayon-12);
  background: var(--gn-bloc-releve);
  color: var(--gn-accent);
}

[data-app="guide-nego"] .gn-invitation__sortie {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-4);
}

[data-app="guide-nego"] .gn-invitation__note {
  padding-top: 6px;
  text-align: center;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
}
</style>
