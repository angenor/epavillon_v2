<script setup lang="ts">
import type { OptionDeFeuille } from '~/components/guide-nego/GnFeuilleBasse.vue'
/**
 * Section 5, quatrième lot : ce qui s'affiche à la place du contenu ou par-dessus lui —
 * marque d'état, chargement, état vide, **verrou d'un module réservé**, erreur, message
 * éphémère, feuille basse, boîte de confirmation.
 *
 * Les trois dernières se téléportent dans `#gn-portail` et couvrent l'écran : sur une
 * planche, elles ne peuvent pas rester ouvertes. Chacune a donc son bouton d'ouverture,
 * et la phrase qui dit ce qu'on y verra — c'est le seul moyen honnête de les montrer.
 */
const ETATS_DE_SESSION = ['prevue', 'en-cours', 'deplacee', 'annulee', 'terminee', 'non-annoncee'] as const

const { t } = useI18n()

const feuilleOptions = ref(false)
const feuilleLibre = ref(false)
const confirmationSimple = ref(false)
const confirmationDangereuse = ref(false)

const dernierChoix = ref<string | null>(null)
const derniereConfirmation = ref<string | null>(null)

const ephemere = ref<'sans-action' | 'avec-action' | null>(null)
const envoi = ref(0)

function montrerEphemere(forme: 'sans-action' | 'avec-action') {
  envoi.value += 1
  ephemere.value = forme
}

const motifs = computed<OptionDeFeuille[]>(() => [
  { valeur: 'deplacee', libelle: t('gn-planche-composants-surfaces.motif-deplacee'), picto: 'moved' },
  { valeur: 'annulee', libelle: t('gn-planche-composants-surfaces.motif-annulee'), picto: 'x-circle' },
  { valeur: 'non-annoncee', libelle: t('gn-planche-composants-surfaces.motif-non-annoncee'), picto: 'diamond' },
  {
    valeur: 'retirer',
    libelle: t('gn-planche-composants-surfaces.motif-retirer'),
    picto: 'close',
    dangereuse: true,
  },
])

const adaptation = ref(true)
const genre = ref(false)
</script>

<template>
  <div class="gn-planche-composants__lot">
    <GnPlancheSection
      :titre="t('gn-planche-composants-surfaces.marque')"
      :propos="t('gn-planche-composants-surfaces.marque-propos')"
    >
      <div class="gn-planche-composants__rangee">
        <GnMarqueEtat v-for="etat in ETATS_DE_SESSION" :key="etat" :etat="etat" />
      </div>
      <div class="gn-planche-composants__rangee">
        <GnMarqueEtat etat="remplace" :libelle="t('gn-planche-composants-surfaces.marque-remplace')" />
        <GnMarqueEtat etat="synchronise" :libelle="t('gn-planche-composants-surfaces.marque-synchronise')" />
      </div>
      <p class="gn-planche-note">{{ t('gn-planche-composants-surfaces.marque-note') }}</p>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-surfaces.chargement')"
      :propos="t('gn-planche-composants-surfaces.chargement-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <span class="gn-planche-composants__legende">{{ t('gn-planche-composants-surfaces.chargement-arc') }}</span>
        <div class="gn-planche-composants__rangee">
          <GnChargement :taille="20" />
          <GnChargement :taille="24" />
        </div>

        <span class="gn-planche-composants__legende">{{ t('gn-planche-composants-surfaces.chargement-squelette') }}</span>
        <GnChargement forme="squelette" :lignes="4" />
      </div>
      <p class="gn-planche-note">{{ t('gn-planche-composants-surfaces.chargement-note') }}</p>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-surfaces.vide')"
      :propos="t('gn-planche-composants-surfaces.vide-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnEtatVide
          picto="calendar"
          :titre="t('gn-planche-composants-surfaces.vide-titre')"
          :texte="t('gn-planche-composants-surfaces.vide-texte')"
          :sortie="t('gn-planche-composants-surfaces.vide-sortie')"
          sortie-vers="/guide-nego/negociations"
        />
      </div>
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnEtatVide
          picto="chat"
          :titre="t('gn-planche-composants-surfaces.vide-sans-titre')"
          :texte="t('gn-planche-composants-surfaces.vide-sans-texte')"
        />
      </div>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-surfaces.erreur')"
      :propos="t('gn-planche-composants-surfaces.erreur-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnEtatErreur
          :titre="t('gn-planche-composants-surfaces.erreur-titre')"
          :texte="t('gn-planche-composants-surfaces.erreur-texte')"
          :sortie="t('gn-planche-composants-surfaces.erreur-reessayer')"
          :sortie-secondaire="t('gn-planche-composants-surfaces.erreur-officiel')"
        />
      </div>
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnEtatErreur
          :titre="t('gn-planche-composants-surfaces.erreur-une-titre')"
          :texte="t('gn-planche-composants-surfaces.erreur-une-texte')"
          :sortie="t('gn-planche-composants-surfaces.erreur-reessayer')"
        />
      </div>
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnEtatErreur
          :titre="t('gn-planche-composants-surfaces.erreur-sans-titre')"
          :texte="t('gn-planche-composants-surfaces.erreur-sans-texte')"
        />
      </div>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-surfaces.ephemere')"
      :propos="t('gn-planche-composants-surfaces.ephemere-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnBouton variante="secondaire" @clic="montrerEphemere('avec-action')">
          {{ t('gn-planche-composants-surfaces.ephemere-avec') }}
        </GnBouton>
        <GnBouton variante="secondaire" @clic="montrerEphemere('sans-action')">
          {{ t('gn-planche-composants-surfaces.ephemere-sans') }}
        </GnBouton>
      </div>
      <p class="gn-planche-note">{{ t('gn-planche-composants-surfaces.ephemere-note') }}</p>

      <GnMessageEphemere
        v-if="ephemere === 'avec-action'"
        :key="`avec-${envoi}`"
        :texte="t('gn-planche-composants-surfaces.ephemere-texte')"
        :action="t('gn-planche-composants-surfaces.ephemere-action')"
        @fini="ephemere = null"
      />
      <GnMessageEphemere
        v-if="ephemere === 'sans-action'"
        :key="`sans-${envoi}`"
        :texte="t('gn-planche-composants-surfaces.ephemere-texte-sans')"
        @fini="ephemere = null"
      />
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-surfaces.feuille')"
      :propos="t('gn-planche-composants-surfaces.feuille-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnBouton variante="secondaire" @clic="feuilleOptions = true">
          {{ t('gn-planche-composants-surfaces.feuille-options') }}
        </GnBouton>
        <GnBouton variante="secondaire" @clic="feuilleLibre = true">
          {{ t('gn-planche-composants-surfaces.feuille-libre') }}
        </GnBouton>
        <p v-if="dernierChoix" class="gn-planche-valeur" role="status">
          {{ t('gn-planche-composants-surfaces.feuille-choix', { option: dernierChoix }) }}
        </p>
      </div>
      <p class="gn-planche-note">{{ t('gn-planche-composants-surfaces.feuille-note') }}</p>

      <GnFeuilleBasse
        v-model="feuilleOptions"
        :titre="t('gn-planche-composants-surfaces.feuille-signaler')"
        :sous-titre="t('gn-planche-composants-surfaces.feuille-signaler-sous')"
        :options="motifs"
        @choisir="dernierChoix = $event.libelle"
      />

      <GnFeuilleBasse
        v-model="feuilleLibre"
        :titre="t('gn-planche-composants-surfaces.feuille-filtrer')"
      >
        <div class="gn-planche-liste">
          <GnCase v-model="adaptation" :libelle="t('gn-planche-composants-surfaces.filtre-adaptation')" />
          <GnCase v-model="genre" :libelle="t('gn-planche-composants-surfaces.filtre-genre')" derniere />
        </div>
      </GnFeuilleBasse>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-surfaces.confirmation')"
      :propos="t('gn-planche-composants-surfaces.confirmation-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnBouton variante="secondaire" @clic="confirmationSimple = true">
          {{ t('gn-planche-composants-surfaces.confirmation-simple') }}
        </GnBouton>
        <GnBouton variante="dangereux" @clic="confirmationDangereuse = true">
          {{ t('gn-planche-composants-surfaces.confirmation-dangereuse') }}
        </GnBouton>
        <p v-if="derniereConfirmation" class="gn-planche-valeur" role="status">
          {{ t('gn-planche-composants-surfaces.confirmation-faite', { action: derniereConfirmation }) }}
        </p>
      </div>
      <p class="gn-planche-note">{{ t('gn-planche-composants-surfaces.confirmation-note') }}</p>

      <GnConfirmation
        v-model="confirmationSimple"
        :question="t('gn-planche-composants-surfaces.confirmation-question')"
        :phrase="t('gn-planche-composants-surfaces.confirmation-phrase')"
        :action="t('gn-planche-composants-surfaces.confirmation-envoyer')"
        picto="send"
        @confirmer="derniereConfirmation = t('gn-planche-composants-surfaces.confirmation-envoyer')"
      />

      <GnConfirmation
        v-model="confirmationDangereuse"
        dangereuse
        :question="t('gn-planche-composants-surfaces.confirmation-question-rouge')"
        :phrase="t('gn-planche-composants-surfaces.confirmation-phrase-rouge')"
        :action="t('gn-planche-composants-surfaces.confirmation-retirer')"
        :retour="t('gn-planche-composants-surfaces.confirmation-garder')"
        @confirmer="derniereConfirmation = t('gn-planche-composants-surfaces.confirmation-retirer')"
      />
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-surfaces.verrou')"
      :propos="t('gn-planche-composants-surfaces.verrou-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnVerrou
          :propos="t('gn-planche-composants-surfaces.verrou-phrase')"
          :contenus="[
            t('gn-planche-composants-surfaces.verrou-canaux'),
            t('gn-planche-composants-surfaces.verrou-annuaire'),
            t('gn-planche-composants-surfaces.verrou-experts'),
          ]"
          :reste-ouvert="t('gn-planche-composants-surfaces.verrou-reste')"
        />
      </div>
      <p class="gn-planche-note">{{ t('gn-planche-composants-surfaces.verrou-note') }}</p>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-surfaces.picto')"
      :propos="t('gn-planche-composants-surfaces.picto-propos')"
    >
      <p class="gn-planche-note">{{ t('gn-planche-composants-surfaces.picto-note') }}</p>
    </GnPlancheSection>
  </div>
</template>
