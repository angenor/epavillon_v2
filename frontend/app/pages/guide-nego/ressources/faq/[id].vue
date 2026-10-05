<script setup lang="ts">
import type { OptionDeFeuille } from '~/components/guide-nego/GnFeuilleBasse.vue'
import type { FaqEntry, FaqMissingReason, FaqReportReason } from '~/types/negotiation-savoir'
import { lectureACompter, liees } from '~/utils/guide-nego/faq'
import { retourAGarder } from '~/utils/guide-nego/parcours'
import { CLE_FAQ_LUES, CLE_RETOUR_APRES_CONNEXION, lireCle, poserCle } from '~/utils/guide-nego/stockage'

/**
 * Une entrée de la FAQ — maquette 05, écran 02 —, lue dans le savoir gardé. Une
 * entrée « à revoir » reste lisible, avec sa date et la ligne de relecture (FR-019 bis).
 * Retours et signalements (maquette 02b, 02c) demandent un compte : sans lui, le geste
 * mène à la connexion, qui ramène ici. Public.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const route = useRoute()
const savoir = useGnSavoir()
const connexion = useGnConnexion()
const api = useApi().guideNegoSavoir
const retours = useGnRetoursFaq()

const FAQ = '/guide-nego/ressources/faq'
const retour = ref(FAQ)

onMounted(() => {
  const avant: unknown = window.history.state?.back
  if (typeof avant === 'string' && avant !== route.fullPath) retour.value = avant
  void savoir.assurer()
  void retours.assurer()
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

const voix = computed(() => (entree.value ? retours.voixSur(entree.value.id) : null))
const feuilleCompte = ref(false)
const feuilleManque = ref(false)
const feuilleSignaler = ref(false)
const MOTIFS: FaqReportReason[] = ['rule_changed', 'wrong', 'source_mismatch']
const motifs = reactive<Record<FaqReportReason, boolean>>({ rule_changed: false, wrong: false, source_mismatch: false })
const precision = ref('')
const choisis = computed(() => MOTIFS.filter((m) => motifs[m]))
const message = ref<{ texte: string; rang: number } | null>(null)

const manques = computed<OptionDeFeuille[]>(() => [
  { valeur: 'too_vague', libelle: t('guide-nego.faq-entree.manque.too_vague') },
  { valeur: 'off_topic', libelle: t('guide-nego.faq-entree.manque.off_topic') },
  {
    valeur: 'outdated',
    libelle: t('guide-nego.faq-entree.manque.outdated'),
    detail: t('guide-nego.faq-entree.manque.experts'),
    detailPicto: 'shield-check',
  },
])

/** Sans compte, la feuille propose de se connecter ; l'entrée est gardée pour y revenir. */
function avecUnCompte(): boolean {
  if (retours.connectee.value) return true
  poserCle(CLE_RETOUR_APRES_CONNEXION, retourAGarder(route.fullPath, Date.now()))
  feuilleCompte.value = true
  return false
}

function oui(): void {
  if (entree.value && avecUnCompte()) void retours.voter(entree.value.id, true)
}

function non(): void {
  if (avecUnCompte()) feuilleManque.value = true
}

function manque(option: OptionDeFeuille): void {
  if (entree.value) void retours.voter(entree.value.id, false, option.valeur as FaqMissingReason)
}

function ouvrirLeSignalement(): void {
  if (!avecUnCompte()) return
  for (const m of MOTIFS) motifs[m] = false
  precision.value = ''
  feuilleSignaler.value = true
}

async function envoyer(): Promise<void> {
  if (!entree.value || !choisis.value.length) return
  feuilleSignaler.value = false
  await retours.signaler(entree.value.id, choisis.value, precision.value)
  const cle = connexion.etat.value.enLigne ? 'envoye' : 'garde'
  message.value = { texte: t(`guide-nego.faq-entree.signaler.${cle}`), rang: (message.value?.rang ?? 0) + 1 }
}

useHead({ title: computed(() => entree.value?.question ?? t('guide-nego.faq-entree.titre')) })
</script>

<template>
  <GnEcran
    :titre="entree?.question ?? t('guide-nego.faq-entree.titre')"
    :surtitre="rubrique"
    :retour="retour"
    titre-long
  >
    <template v-if="!connexion.etat.value.enLigne" #connexion>
      <GnLigneConnexion :en-ligne="false" :lu-a="etat.luA" />
    </template>
    <template v-if="entree" #pied>
      <GnVerifieLe v-if="entree.verified_on" :jour="entree.verified_on" :a-revoir="entree.status === 'to_review'" />
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
        <GnRetourUtile :merci="voix !== null" @oui="oui" @non="non" />
        <GnBouton variante="secondaire" picto="warn" class="gn-faq-entree__signaler" @clic="ouvrirLeSignalement">
          {{ t('guide-nego.faq-entree.depasse') }}
        </GnBouton>
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

    <GnFeuilleBasse
      v-model="feuilleManque"
      :titre="t('guide-nego.faq-entree.manque.titre')"
      :options="manques"
      @choisir="manque"
    >
      <p class="gn-faq-entree__anonyme">{{ t('guide-nego.faq-entree.anonyme') }}</p>
    </GnFeuilleBasse>

    <GnFeuilleBasse
      v-model="feuilleSignaler"
      :titre="t('guide-nego.faq-entree.depasse')"
      :sous-titre="t('guide-nego.faq-entree.signaler.texte')"
      picto="warn"
      picto-danger
    >
      <fieldset class="gn-faq-entree__motifs">
        <legend class="gn-hors-ecran">{{ t('guide-nego.faq-entree.signaler.motifs') }}</legend>
        <GnCase
          v-for="(m, rang) in MOTIFS"
          :key="m"
          v-model="motifs[m]"
          :libelle="t(`guide-nego.faq-entree.signaler.motif.${m}`)"
          :derniere="rang === MOTIFS.length - 1"
        />
      </fieldset>
      <GnZoneTexte
        v-model="precision"
        :libelle="t('guide-nego.faq-entree.signaler.precision')"
        :indication="t('guide-nego.faq-entree.signaler.indication')"
        :maximum="600"
      />
      <GnBouton picto="send" :desactive="!choisis.length" @clic="envoyer">
        {{ t('guide-nego.faq-entree.signaler.envoyer') }}
      </GnBouton>
    </GnFeuilleBasse>

    <GnFeuilleBasse
      v-model="feuilleCompte"
      :titre="t('guide-nego.faq-entree.compte.titre')"
      :sous-titre="t('guide-nego.faq-entree.compte.texte')"
    >
      <div class="gn-faq-entree__compte">
        <GnBouton vers="/guide-nego/compte" picto="user">{{ t('guide-nego.faq-entree.compte.creer') }}</GnBouton>
        <GnBouton variante="secondaire" vers="/guide-nego/connexion">{{ t('guide-nego.faq-entree.compte.connexion') }}</GnBouton>
      </div>
    </GnFeuilleBasse>

    <GnMessageEphemere v-if="message" :key="message.rang" :texte="message.texte" @fini="message = null" />
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-faq-entree__relue {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-faq-entree {
  display: flex;
  flex-direction: column;
}

/* La réponse se lit comme une définition du lexique : 17/1,6 en texte de lecture. */
[data-app="guide-nego"] .gn-faq-entree__reponse {
  display: flex;
  flex-direction: column;
  gap: var(--gn-entre-paragraphes);
  max-width: var(--gn-mesure-lecture);
  color: var(--gn-texte-lecture);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
}

[data-app="guide-nego"] .gn-faq-entree__retour {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-espace-20);
}

[data-app="guide-nego"] .gn-faq-entree__signaler .gn-picto {
  color: var(--gn-danger);
}

[data-app="guide-nego"] .gn-faq-entree li:last-child > .gn-ligne-question {
  border-bottom: none;
}

[data-app="guide-nego"] .gn-faq-entree__anonyme {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
}

[data-app="guide-nego"] .gn-faq-entree__motifs {
  border: none;
  margin: 0;
  padding: 0;
  min-width: 0;
}

[data-app="guide-nego"] .gn-faq-entree__compte {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
}
</style>
