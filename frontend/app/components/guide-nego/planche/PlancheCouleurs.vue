<script setup lang="ts">
import { useJetonsLus } from './PlancheSection.vue'

/**
 * Section 1 — toutes les couleurs du système, dans l'ordre du thème.
 *
 * Aucune valeur n'est recopiée : chaque pastille prend sa couleur par `var()` et
 * affiche ce que le navigateur en a fait. Basculer le thème suffit donc à voir ce
 * que le thème sombre redéfinit — et ce qu'il laisse intact.
 */
const CHARTE = [
  '--gn-charte-vert-tres-fonce',
  '--gn-charte-vert-fonce',
  '--gn-charte-vert-moyen',
  '--gn-charte-vert-de-fond',
  '--gn-charte-vert-de-charte',
  '--gn-charte-jaune-fonce',
  '--gn-charte-jaune-tres-fonce',
  '--gn-charte-jaune-de-fond',
  '--gn-charte-jaune-de-charte',
  '--gn-charte-noir',
  '--gn-charte-gris',
  '--gn-charte-gris-pale',
  '--gn-charte-blanc',
  '--gn-charte-rouge',
  '--gn-charte-violet',
  '--gn-charte-cyan-fonce',
] as const

const NUANCES = [
  '--gn-nuance-sombre-fond',
  '--gn-nuance-sombre-bloc',
  '--gn-nuance-sombre-texte',
  '--gn-nuance-sombre-texte-2',
  '--gn-nuance-sombre-vert',
  '--gn-nuance-sombre-vert-filet',
  '--gn-nuance-sombre-jaune',
  '--gn-nuance-sombre-cyan',
  '--gn-nuance-sombre-rouge',
  '--gn-nuance-sombre-violet',
  '--gn-nuance-sombre-gris',
  '--gn-nuance-sombre-squelette',
  '--gn-nuance-sombre-jaune-bloc',
  '--gn-nuance-rouge-aplat',
] as const

const ROLES = [
  {
    cle: 'fonds',
    jetons: [
      '--gn-fond',
      '--gn-fond-2',
      '--gn-texte',
      '--gn-texte-2',
      '--gn-titre',
      '--gn-accent',
      '--gn-accent-inv',
      '--gn-sur-titre',
      '--gn-danger-texte',
      '--gn-bulle-envoyee',
      '--gn-bulle-envoyee-texte',
    ],
  },
  { cle: 'filets', jetons: ['--gn-filet', '--gn-filet-fort'] },
  {
    cle: 'semantique',
    jetons: [
      '--gn-attention',
      '--gn-attention-aplat',
      '--gn-attention-aplat-texte',
      '--gn-attention-fond',
      '--gn-succes',
      '--gn-information',
      '--gn-danger',
      '--gn-danger-aplat',
      '--gn-reseau',
      '--gn-terminee',
      '--gn-prevue',
    ],
  },
  { cle: 'sur-vert', jetons: ['--gn-vif-jaune', '--gn-vif-vert'] },
  {
    cle: 'composants',
    jetons: [
      '--gn-presse',
      '--gn-focus',
      '--gn-desactive-fond',
      '--gn-desactive-texte',
      '--gn-squelette',
      '--gn-jauge-fond',
      '--gn-voile',
    ],
  },
  { cle: 'pictos', jetons: ['--gn-picto', '--gn-picto-secondaire', '--gn-picto-document'] },
] as const

/** Un état ne porte pas de couleur : il renvoie au rôle qui la porte. */
const ETATS: readonly (readonly [string, string])[] = [
  ['--gn-etat-prevue', '--gn-prevue'],
  ['--gn-etat-en-cours', '--gn-succes'],
  ['--gn-etat-deplacee', '--gn-information'],
  ['--gn-etat-annulee', '--gn-danger'],
  ['--gn-etat-terminee', '--gn-terminee'],
  ['--gn-etat-non-annoncee', '--gn-reseau'],
  ['--gn-etat-ouverte', '--gn-succes'],
  ['--gn-etat-acces-limite', '--gn-attention'],
  ['--gn-etat-envoye', '--gn-information'],
  ['--gn-etat-valide', '--gn-succes'],
  ['--gn-etat-non-retenu', '--gn-terminee'],
  ['--gn-etat-a-jour', '--gn-succes'],
  ['--gn-etat-remplace', '--gn-attention'],
  ['--gn-etat-depasse', '--gn-danger'],
  ['--gn-etat-telecharge', '--gn-accent'],
  ['--gn-etat-reserve', '--gn-attention'],
  ['--gn-etat-lien-externe', '--gn-texte-2'],
  ['--gn-etat-nouveau', '--gn-prevue'],
  ['--gn-etat-propose', '--gn-attention'],
  ['--gn-etat-classe', '--gn-succes'],
  ['--gn-etat-relu', '--gn-succes'],
  ['--gn-etat-non-relu', '--gn-attention'],
  ['--gn-etat-en-relecture', '--gn-information'],
  ['--gn-etat-reussi', '--gn-succes'],
  ['--gn-etat-a-refaire', '--gn-terminee'],
  ['--gn-etat-hors-connexion', '--gn-attention'],
  ['--gn-etat-synchronise', '--gn-succes'],
  ['--gn-etat-a-envoyer', '--gn-attention'],
  ['--gn-etat-source-officielle', '--gn-texte-2'],
  ['--gn-etat-traduction', '--gn-texte-2'],
  ['--gn-etat-lecture-impossible', '--gn-danger'],
  ['--gn-etat-verifie', '--gn-succes'],
  ['--gn-etat-inscrite', '--gn-succes'],
  ['--gn-etat-liste-attente', '--gn-attention'],
  ['--gn-etat-complet', '--gn-terminee'],
  ['--gn-etat-rediffusion', '--gn-accent'],
  ['--gn-etat-en-attente', '--gn-attention'],
  ['--gn-etat-repondue', '--gn-succes'],
  ['--gn-etat-ajoutee-faq', '--gn-accent'],
  ['--gn-etat-chevauche', '--gn-attention'],
  ['--gn-etat-expert', '--gn-succes'],
  ['--gn-etat-a-confirmer', '--gn-attention'],
]

const COULEURS = [
  ...CHARTE,
  ...NUANCES,
  ...ROLES.flatMap((groupe) => groupe.jetons),
  ...ETATS.map(([jeton]) => jeton),
]

const { t } = useI18n()
const racine = useTemplateRef<HTMLElement>('racine')
const lus = useJetonsLus(racine, COULEURS, ['--gn-voile-opacite'])

const valeur = (jeton: string): string => lus.value[jeton] ?? '…'
const usage = (jeton: string): string => t(`gn-planche-couleurs.usage.${jeton.slice(5)}`)
const aplat = (jeton: string) => ({ background: `var(${jeton})` })
</script>

<template>
  <GnPlancheSection
    numero="1"
    :titre="t('gn-planche-couleurs.titre')"
    :propos="t('gn-planche-couleurs.propos')"
  >
    <div ref="racine" class="gn-planche-couleurs">
      <p class="gn-planche-note">{{ t('gn-planche-couleurs.regle') }}</p>
      <p class="gn-planche-note">{{ t('gn-planche-section.lecture') }}</p>

      <GnPlancheSection
        :titre="t('gn-planche-couleurs.charte')"
        :propos="t('gn-planche-couleurs.charte-propos')"
      >
        <div class="gn-planche-grille">
          <div v-for="jeton in CHARTE" :key="jeton" class="gn-planche-echantillon">
            <div class="gn-planche-echantillon__aplat" :style="aplat(jeton)" />
            <div class="gn-planche-legende">
              <span class="gn-planche-jeton">{{ jeton }}</span>
              <span class="gn-planche-valeur">{{ valeur(jeton) }}</span>
              <span class="gn-planche-valeur">{{ usage(jeton) }}</span>
            </div>
          </div>
        </div>
      </GnPlancheSection>

      <GnPlancheSection
        :titre="t('gn-planche-couleurs.nuances')"
        :propos="t('gn-planche-couleurs.nuances-propos')"
      >
        <div class="gn-planche-grille">
          <div v-for="jeton in NUANCES" :key="jeton" class="gn-planche-echantillon">
            <div class="gn-planche-echantillon__aplat" :style="aplat(jeton)" />
            <div class="gn-planche-legende">
              <span class="gn-planche-jeton">{{ jeton }}</span>
              <span class="gn-planche-valeur">{{ valeur(jeton) }}</span>
              <span class="gn-planche-valeur">{{ usage(jeton) }}</span>
            </div>
          </div>
        </div>
      </GnPlancheSection>

      <GnPlancheSection
        :titre="t('gn-planche-couleurs.roles')"
        :propos="t('gn-planche-couleurs.roles-propos')"
      >
        <GnPlancheSection
          v-for="groupe in ROLES"
          :key="groupe.cle"
          :titre="t(`gn-planche-couleurs.groupe.${groupe.cle}`)"
        >
          <div class="gn-planche-grille">
            <div v-for="jeton in groupe.jetons" :key="jeton" class="gn-planche-echantillon">
              <div class="gn-planche-echantillon__aplat" :style="aplat(jeton)" />
              <div class="gn-planche-legende">
                <span class="gn-planche-jeton">{{ jeton }}</span>
                <span class="gn-planche-valeur">{{ valeur(jeton) }}</span>
                <span class="gn-planche-valeur">{{ usage(jeton) }}</span>
              </div>
            </div>
          </div>
          <p v-if="groupe.cle === 'composants'" class="gn-planche-note">
            --gn-voile-opacite : {{ lus['--gn-voile-opacite'] || '…' }} —
            {{ t('gn-planche-couleurs.voile-opacite') }}
          </p>
        </GnPlancheSection>
      </GnPlancheSection>

      <GnPlancheSection
        :titre="t('gn-planche-couleurs.etats')"
        :propos="t('gn-planche-couleurs.etats-propos')"
      >
        <ul class="gn-planche-grille gn-planche-grille--large">
          <li v-for="[jeton, role] in ETATS" :key="jeton" class="gn-planche-etat">
            <span class="gn-planche-etat__pastille" :style="aplat(jeton)" />
            <span class="gn-planche-etat__texte">
              <span class="gn-planche-jeton">{{ jeton }}</span>
              <span class="gn-planche-valeur">{{ t('gn-planche-section.renvoi') }} {{ role }}</span>
              <span class="gn-planche-valeur">{{ valeur(jeton) }}</span>
            </span>
          </li>
        </ul>
      </GnPlancheSection>
    </div>
  </GnPlancheSection>
</template>

<style>
[data-app="guide-nego"] .gn-planche-couleurs {
  display: flex;
  flex-direction: column;
  gap: var(--gn-entre-blocs);
}

[data-app="guide-nego"] .gn-planche-etat {
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-8);
  min-height: var(--gn-cible);
  padding: var(--gn-espace-8);
  border: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-planche-etat__pastille {
  flex: none;
  width: var(--gn-case);
  height: var(--gn-case);
  border: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-planche-etat__texte {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-4);
  min-width: 0;
}
</style>
