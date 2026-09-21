<script setup lang="ts">
/**
 * Section 5, troisième lot : ce qu'on remplit ou règle — recherche, champ, zone de
 * texte, case, interrupteur, cercle, ligne de réglage.
 *
 * Les lignes de 56 px sont montrées à 360 px, la largeur sur laquelle leur cible a été
 * arrêtée : une vitrine large ferait croire à une ligne confortable qui ne l'est pas.
 */
interface OptionDeQuiz {
  valeur: string
  libelle: string
  detail?: string
  marque?: 'juste' | 'faux'
  desactive?: boolean
}

const { t } = useI18n()

const recherche = ref('')
const rechercheSaisie = ref('adapt')
const code = ref('')
const codeFaux = ref('NEGO-2')
const pays = ref('Sénégal')
const heure = ref('14:30')
const restitution = ref('')
const signalement = ref(
  "La session a commencé avec vingt minutes de retard ; le président a annoncé une reprise à 15:00 en salle 9.",
)

const thematique = ref(true)
const veilleTexte = ref(false)
const notifications = ref(true)
const telechargementAuto = ref(false)

const partage = ref('groupe')
const reponse = ref('b')

const partages = computed<OptionDeQuiz[]>(() => [
  {
    valeur: 'groupe',
    libelle: t('gn-planche-composants-saisie.partage-groupe'),
    detail: t('gn-planche-composants-saisie.partage-groupe-detail'),
  },
  { valeur: 'ifdd', libelle: t('gn-planche-composants-saisie.partage-ifdd') },
  { valeur: 'personne', libelle: t('gn-planche-composants-saisie.partage-personne') },
])

const reponses = computed<OptionDeQuiz[]>(() => [
  { valeur: 'a', libelle: t('gn-planche-composants-saisie.reponse-a') },
  { valeur: 'b', libelle: t('gn-planche-composants-saisie.reponse-b'), marque: 'faux' },
  { valeur: 'c', libelle: t('gn-planche-composants-saisie.reponse-c'), marque: 'juste' },
])
</script>

<template>
  <div class="gn-planche-composants__lot">
    <GnPlancheSection
      :titre="t('gn-planche-composants-saisie.recherche')"
      :propos="t('gn-planche-composants-saisie.recherche-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <span class="gn-planche-composants__legende">{{ t('gn-planche-composants-saisie.recherche-repos') }}</span>
        <GnChampRecherche
          v-model="recherche"
          :libelle="t('gn-planche-composants-saisie.recherche-libelle')"
          :indication="t('gn-planche-composants-saisie.recherche-indication')"
        />

        <span class="gn-planche-composants__legende">{{ t('gn-planche-composants-saisie.recherche-saisie') }}</span>
        <GnChampRecherche
          v-model="rechercheSaisie"
          :libelle="t('gn-planche-composants-saisie.recherche-libelle')"
        />

        <span class="gn-planche-composants__legende">{{ t('gn-planche-composants-saisie.recherche-chargement') }}</span>
        <GnChampRecherche
          :model-value="rechercheSaisie"
          :libelle="t('gn-planche-composants-saisie.recherche-libelle')"
          chargement
        />
      </div>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-saisie.champ')"
      :propos="t('gn-planche-composants-saisie.champ-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnChamp
          v-model="code"
          :libelle="t('gn-planche-composants-saisie.champ-code')"
          :indication="t('gn-planche-composants-saisie.champ-code-indication')"
          :aide="t('gn-planche-composants-saisie.champ-code-aide')"
        />
        <GnChamp
          v-model="codeFaux"
          :libelle="t('gn-planche-composants-saisie.champ-code')"
          :erreur="t('gn-planche-composants-saisie.champ-code-erreur')"
        />
        <GnChamp
          v-model="pays"
          :libelle="t('gn-planche-composants-saisie.champ-pays')"
          desactive
        />
        <GnChamp
          v-model="heure"
          type="time"
          :libelle="t('gn-planche-composants-saisie.champ-heure')"
          :aide="t('gn-planche-composants-saisie.champ-heure-aide')"
        />
      </div>
      <p class="gn-planche-note">{{ t('gn-planche-composants-saisie.champ-note') }}</p>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-saisie.zone')"
      :propos="t('gn-planche-composants-saisie.zone-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnZoneTexte
          v-model="signalement"
          :libelle="t('gn-planche-composants-saisie.zone-libelle')"
          :aide="t('gn-planche-composants-saisie.zone-aide')"
        />
        <GnZoneTexte
          v-model="restitution"
          :libelle="t('gn-planche-composants-saisie.zone-restitution')"
          :indication="t('gn-planche-composants-saisie.zone-indication')"
          :erreur="t('gn-planche-composants-saisie.zone-erreur')"
          :maximum="280"
        />
      </div>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-saisie.case')"
      :propos="t('gn-planche-composants-saisie.case-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <div class="gn-planche-liste">
          <GnCase
            v-model="thematique"
            :libelle="t('gn-planche-composants-saisie.case-adaptation')"
            :detail="t('gn-planche-composants-saisie.case-adaptation-detail')"
          />
          <GnCase v-model="veilleTexte" :libelle="t('gn-planche-composants-saisie.case-genre')" />
          <GnCase
            :libelle="t('gn-planche-composants-saisie.case-article')"
            desactive
            derniere
          />
        </div>
      </div>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-saisie.interrupteur')"
      :propos="t('gn-planche-composants-saisie.interrupteur-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <div class="gn-planche-liste">
          <GnInterrupteur
            v-model="notifications"
            :libelle="t('gn-planche-composants-saisie.interrupteur-alertes')"
            :detail="t('gn-planche-composants-saisie.interrupteur-alertes-detail')"
          />
          <GnInterrupteur
            v-model="telechargementAuto"
            :libelle="t('gn-planche-composants-saisie.interrupteur-telechargement')"
          />
          <GnInterrupteur
            :libelle="t('gn-planche-composants-saisie.interrupteur-desactive')"
            desactive
            derniere
          />
        </div>
      </div>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-saisie.cercle')"
      :propos="t('gn-planche-composants-saisie.cercle-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <span class="gn-planche-composants__legende">{{ t('gn-planche-composants-saisie.cercle-choix') }}</span>
        <GnCercle
          v-model="partage"
          :options="partages"
          :libelle="t('gn-planche-composants-saisie.cercle-libelle')"
        />

        <span class="gn-planche-composants__legende">{{ t('gn-planche-composants-saisie.cercle-corrige') }}</span>
        <GnCercle
          v-model="reponse"
          :options="reponses"
          :libelle="t('gn-planche-composants-saisie.cercle-quiz-libelle')"
          desactive
        />
      </div>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="t('gn-planche-composants-saisie.reglage')"
      :propos="t('gn-planche-composants-saisie.reglage-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <div class="gn-planche-liste">
          <GnLigneReglage
            picto="text-size"
            :libelle="t('gn-planche-composants-saisie.reglage-lecture')"
            :valeur="t('gn-planche-composants-saisie.reglage-lecture-valeur')"
            vers="/guide-nego"
          />
          <GnLigneReglage picto="bell" :libelle="t('gn-planche-composants-saisie.reglage-alertes')">
            <GnInterrupteur
              v-model="notifications"
              dans-ligne
              :libelle="t('gn-planche-composants-saisie.reglage-alertes')"
            />
          </GnLigneReglage>
          <GnLigneReglage
            picto="download"
            :libelle="t('gn-planche-composants-saisie.reglage-garde')"
            :valeur="t('gn-planche-composants-saisie.reglage-garde-valeur')"
            derniere
          />
        </div>
      </div>
      <p class="gn-planche-note">{{ t('gn-planche-composants-saisie.reglage-note') }}</p>
    </GnPlancheSection>
  </div>
</template>
