<script setup lang="ts">
import type { SegmentDeChoix } from '~/components/guide-nego/GnSegmente.vue'
/**
 * Section 5, deuxième lot : ce qu'on actionne — boutons, onglets de filtre, pilules,
 * sélecteur segmenté.
 *
 * Les boutons sont montrés en pleine largeur, leur largeur par défaut : c'est celle de
 * l'action principale en bas d'écran, et la seule qu'une vitrine puisse montrer sans
 * forcer une taille que le composant n'a pas.
 */
const { t } = useI18n()

const VARIANTES = ['principal', 'secondaire', 'discret', 'dangereux'] as const

const filtreCourt = computed(() => [
  { valeur: 'miennes', libelle: t('gn-planche-composants-commandes.filtre-miennes') },
  { valeur: 'toutes', libelle: t('gn-planche-composants-commandes.filtre-toutes') },
  { valeur: 'salle', libelle: t('gn-planche-composants-commandes.filtre-salle') },
])

const filtreLong = computed(() => [
  ...filtreCourt.value,
  { valeur: 'signalees', libelle: t('gn-planche-composants-commandes.filtre-signalees') },
  { valeur: 'terminees', libelle: t('gn-planche-composants-commandes.filtre-terminees') },
])

const jours = computed(() => [
  { valeur: 'jour', libelle: t('gn-planche-composants-commandes.segment-aujourdhui') },
  { valeur: 'demain', libelle: t('gn-planche-composants-commandes.segment-demain') },
  { valeur: 'semaine', libelle: t('gn-planche-composants-commandes.segment-semaine') },
])

const themes = computed<SegmentDeChoix[]>(() => [
  { valeur: 'clair', libelle: t('gn-planche-composants-commandes.segment-clair'), picto: 'sun' },
  { valeur: 'sombre', libelle: t('gn-planche-composants-commandes.segment-sombre'), picto: 'moon' },
  { valeur: 'systeme', libelle: t('gn-planche-composants-commandes.segment-systeme'), picto: 'eye' },
])

const onglet = ref('miennes')
const ongletLong = ref('signalees')
const jour = ref('jour')
const theme = ref('clair')

const adaptation = ref(true)
const genre = ref(false)
const suggeree = ref(true)
const typeOuvert = ref(false)
</script>

<template>
  <div class="gn-planche-composants__lot">
    <GnPlancheSection
      :titre="t('gn-planche-composants-commandes.boutons')"
      :propos="t('gn-planche-composants-commandes.boutons-propos')"
    >
      <ul class="gn-planche-grille gn-planche-grille--large">
        <li
          v-for="variante in VARIANTES"
          :key="variante"
          class="gn-planche-composants__vitrine"
        >
          <span class="gn-planche-jeton">{{ t(`gn-planche-composants-commandes.variante.${variante}`) }}</span>
          <GnBouton :variante="variante">
            {{ t(`gn-planche-composants-commandes.verbe.${variante}`) }}
          </GnBouton>
          <GnBouton :variante="variante" desactive>
            {{ t('gn-planche-composants-commandes.bouton-desactive') }}
          </GnBouton>
        </li>
      </ul>
      <p class="gn-planche-note">{{ t('gn-planche-composants-commandes.boutons-note') }}</p>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-commandes.bouton-etats')"
      :propos="t('gn-planche-composants-commandes.bouton-etats-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <span class="gn-planche-composants__legende">{{ t('gn-planche-composants-commandes.bouton-repos') }}</span>
        <GnBouton variante="secondaire" picto="star">
          {{ t('gn-planche-composants-commandes.bouton-favori') }}
        </GnBouton>

        <span class="gn-planche-composants__legende">{{ t('gn-planche-composants-commandes.bouton-actif') }}</span>
        <GnBouton variante="secondaire" picto="star" actif>
          {{ t('gn-planche-composants-commandes.bouton-favori') }}
        </GnBouton>

        <span class="gn-planche-composants__legende">{{ t('gn-planche-composants-commandes.bouton-picto') }}</span>
        <GnBouton picto="download">
          {{ t('gn-planche-composants-commandes.bouton-telecharger') }}
        </GnBouton>

        <span class="gn-planche-composants__legende">{{ t('gn-planche-composants-commandes.bouton-chargement') }}</span>
        <GnBouton chargement>{{ t('gn-planche-composants-commandes.bouton-en-cours') }}</GnBouton>

        <span class="gn-planche-composants__legende">{{ t('gn-planche-composants-commandes.bouton-lien') }}</span>
        <GnBouton variante="discret" vers="/guide-nego">
          {{ t('gn-planche-composants-commandes.bouton-plus-tard') }}
        </GnBouton>
      </div>
      <p class="gn-planche-note">{{ t('gn-planche-composants-commandes.bouton-chargement-note') }}</p>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-commandes.largeurs')"
      :propos="t('gn-planche-composants-commandes.largeurs-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnBouton>{{ t('gn-planche-composants-commandes.bouton-envoyer') }}</GnBouton>
        <div class="gn-planche-composants__paire">
          <GnBouton variante="secondaire" largeur="demie">
            {{ t('gn-planche-composants-commandes.bouton-revenir') }}
          </GnBouton>
          <GnBouton largeur="demie">
            {{ t('gn-planche-composants-commandes.bouton-envoyer') }}
          </GnBouton>
        </div>
      </div>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-commandes.onglets-filtre')"
      :propos="t('gn-planche-composants-commandes.onglets-filtre-propos')"
    >
      <div class="gn-planche-composants__cadre">
        <GnOngletsFiltre
          v-model="onglet"
          :onglets="filtreCourt"
          :libelle="t('gn-planche-composants-commandes.filtre-libelle')"
        />
      </div>
      <p class="gn-planche-composants__legende">{{ t('gn-planche-composants-commandes.onglets-filtre-defile') }}</p>
      <div class="gn-planche-composants__cadre">
        <GnOngletsFiltre
          v-model="ongletLong"
          :onglets="filtreLong"
          :libelle="t('gn-planche-composants-commandes.filtre-libelle')"
        />
      </div>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-commandes.pilules')"
      :propos="t('gn-planche-composants-commandes.pilules-propos')"
    >
      <div class="gn-planche-composants__rangee">
        <GnPilule v-model:choisie="adaptation">
          {{ t('gn-planche-composants-commandes.pilule-adaptation') }}
        </GnPilule>
        <GnPilule v-model:choisie="genre">
          {{ t('gn-planche-composants-commandes.pilule-genre') }}
        </GnPilule>
        <GnPilule desactive>
          {{ t('gn-planche-composants-commandes.pilule-article') }}
        </GnPilule>
      </div>

      <p class="gn-planche-composants__legende">{{ t('gn-planche-composants-commandes.pilule-decochable') }}</p>
      <div class="gn-planche-composants__rangee">
        <GnPilule v-model:choisie="suggeree" variante="decochable">
          {{ t('gn-planche-composants-commandes.pilule-finance') }}
        </GnPilule>
      </div>

      <p class="gn-planche-composants__legende">{{ t('gn-planche-composants-commandes.pilule-chevron') }}</p>
      <div class="gn-planche-composants__rangee">
        <GnPilule variante="chevron" :ouverte="typeOuvert" @clic="typeOuvert = !typeOuvert">
          {{ t('gn-planche-composants-commandes.pilule-type') }}
        </GnPilule>
      </div>
      <p class="gn-planche-note">{{ t('gn-planche-composants-commandes.pilule-note') }}</p>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-commandes.segmente')"
      :propos="t('gn-planche-composants-commandes.segmente-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnSegmente
          v-model="jour"
          :segments="jours"
          :libelle="t('gn-planche-composants-commandes.segmente-jour-libelle')"
        />
        <GnSegmente
          v-model="theme"
          :segments="themes"
          :libelle="t('gn-planche-composants-commandes.segmente-theme-libelle')"
        />
      </div>
      <p class="gn-planche-note">{{ t('gn-planche-composants-commandes.segmente-note') }}</p>
    </GnPlancheSection>
  </div>
</template>
