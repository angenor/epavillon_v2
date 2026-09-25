<script setup lang="ts">
import type { FaqEntry } from '~/types/negotiation-savoir'
import { lectureACompter, liees } from '~/utils/guide-nego/faq'
import { CLE_FAQ_LUES, lireCle, poserCle } from '~/utils/guide-nego/stockage'

/**
 * Une entrée de la FAQ — maquette 05, écran 02 —, lue dans le savoir gardé. Une
 * entrée « à revoir » reste lisible, avec sa date et la ligne de relecture (FR-019 bis).
 * Retours et signalements sont posés, inactifs, jusqu'au récit 5. Public.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const route = useRoute()
const savoir = useGnSavoir()
const connexion = useGnConnexion()
const api = useApi().guideNegoSavoir

const FAQ = '/guide-nego/ressources/faq'
const retour = ref(FAQ)

onMounted(() => {
  const avant: unknown = window.history.state?.back
  if (typeof avant === 'string' && avant !== route.fullPath) retour.value = avant
  void savoir.assurer()
})

const id = computed(() => String(route.params.id ?? ''))
const entree = computed<FaqEntry | null>(() => savoir.entreeDeFaq(id.value))
const etat = computed(() => savoir.etat.value)
const chargement = computed(() => !etat.value.pret)
const jamaisLu = computed(() => !chargement.value && !etat.value.valeur)

const rubrique = computed(() => savoir.rubriques.value.find((r) => r.code === entree.value?.section_code)?.label)
const paragraphes = computed(() => (entree.value?.answer ?? '').split(/\n\s*\n/u).filter((p) => p.trim()))
const questionsLiees = computed(() => (entree.value ? liees(entree.value, savoir.faq.value) : []))

const jourLocal = (d: Date) =>
  `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`

// Une fois par entrée, par jour et par téléphone, avec réseau seulement : hors connexion, rien ne se compte (R12).
watch(
  [() => entree.value?.id ?? null, () => connexion.etat.value.enLigne],
  ([lue, enLigne]) => {
    if (!lue || !enLigne || !import.meta.client) return
    const { compter, garder } = lectureACompter(lireCle(CLE_FAQ_LUES), lue, jourLocal(new Date()))
    if (!compter) return
    poserCle(CLE_FAQ_LUES, garder)
    api.lireUneEntreeDeFaq(lue).catch(() => undefined)
  },
  { immediate: true },
)

useHead({ title: computed(() => entree.value?.question ?? t('guide-nego.faq-entree.titre')) })
</script>

<template>
  <GnEcran
    :titre="entree?.question ?? t('guide-nego.faq-entree.titre')"
    :surtitre="rubrique"
    :retour="retour"
    titre-long
  >
    <template #connexion>
      <GnLigneConnexion :en-ligne="connexion.etat.value.enLigne" :lu-a="etat.luA" />
    </template>
    <template v-if="entree" #pied>
      <GnVerifieLe :jour="entree.verified_on" :a-revoir="entree.status === 'to_review'" />
      <p v-if="entree.status === 'to_review'" class="gn-faq-entree__relue">
        <GnPicto nom="clock" :taille="16" />{{ t('guide-nego.faq-entree.a-revoir') }}
      </p>
    </template>

    <GnChargement v-if="chargement" forme="squelette" :lignes="5" :libelle="t('guide-nego.faq-entree.chargement')" />

    <GnEtatErreur
      v-else-if="jamaisLu"
      :titre="t('guide-nego.faq.erreur.titre')"
      :texte="t('guide-nego.faq.erreur.texte')"
    />

    <GnEtatVide
      v-else-if="!entree"
      picto="quiz"
      :titre="t('guide-nego.faq-entree.absente.titre')"
      :texte="t('guide-nego.faq-entree.absente.texte')"
      :sortie="t('guide-nego.faq-entree.absente.sortie')"
      :sortie-vers="FAQ"
    />

    <article v-else class="gn-faq-entree">
      <div class="gn-faq-entree__reponse">
        <p v-for="(p, i) in paragraphes" :key="i">{{ p }}</p>
      </div>

      <section v-if="entree.sources.length">
        <GnEnteteGroupe :titre="t('guide-nego.faq-entree.sources')" :compteur="entree.sources.length" />
        <GnSource v-for="(source, i) in entree.sources" :key="i" :source="source" />
      </section>

      <div class="gn-faq-entree__retour">
        <p class="gn-faq-entree__aidee">{{ t('guide-nego.faq-entree.aidee') }}</p>
        <div class="gn-faq-entree__boutons">
          <GnBouton variante="secondaire" largeur="demie" desactive aria-describedby="gn-faq-entree-bientot">{{ t('guide-nego.faq-entree.oui') }}</GnBouton>
          <GnBouton variante="secondaire" largeur="demie" desactive aria-describedby="gn-faq-entree-bientot">{{ t('guide-nego.faq-entree.non') }}</GnBouton>
        </div>
        <GnBouton variante="secondaire" picto="warn" desactive aria-describedby="gn-faq-entree-bientot" class="gn-faq-entree__signaler">
          {{ t('guide-nego.faq-entree.depasse') }}
        </GnBouton>
        <p id="gn-faq-entree-bientot" class="gn-faq-entree__bientot">{{ t('guide-nego.faq-entree.bientot') }}</p>
      </div>

      <section v-if="questionsLiees.length">
        <GnEnteteGroupe :titre="t('guide-nego.faq-entree.liees')" />
        <ul role="list">
          <li v-for="e in questionsLiees" :key="e.id">
            <GnLigneQuestion :entree="e" :vers="`${FAQ}/${e.id}`" />
          </li>
        </ul>
      </section>
    </article>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-faq-entree__relue {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-faq-entree {
  display: flex;
  flex-direction: column;
  padding-bottom: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-faq-entree__reponse {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
  padding-top: var(--gn-espace-12);
  max-width: var(--gn-mesure-lecture);
}

[data-app="guide-nego"] .gn-faq-entree__retour {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-block: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-faq-entree__aidee {
  color: var(--gn-titre);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-faq-entree__boutons {
  display: flex;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-faq-entree__signaler .gn-picto {
  color: var(--gn-danger);
}

[data-app="guide-nego"] .gn-faq-entree__bientot {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}
</style>
