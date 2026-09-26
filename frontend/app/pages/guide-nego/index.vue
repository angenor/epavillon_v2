<script setup lang="ts">
import {
  CLE_OUVERTURE_VUE,
  CLE_THEMATIQUES_PROPOSEES,
  lireCle,
  poserCle,
} from '~/utils/guide-nego/stockage'
import { propositionAFaire } from '~/utils/guide-nego/thematiques'
import { lireProgression, lireRecents, type Recent } from '~/utils/guide-nego/appareil-lecture'
import { marquesDeLigne } from '~/utils/guide-nego/documents'
import { BLOCS_DE_MA_JOURNEE, documentsRecents, jourLisible } from '~/utils/guide-nego/journee'
import { prochaineSession, sessionsDeLAgenda } from '~/utils/guide-nego/agenda'
import { etatAffiche } from '~/utils/guide-nego/sessions'

/**
 * Écrans 07 et 08 — « Ma journée », l'accueil quotidien.
 *
 * Chaque bloc que son module ne remplit pas encore tient en **une ligne** sous son
 * en-tête — ce qui manque, et quand cela viendra : quatre états vides d'écran entier,
 * à la suite, feraient l'écran d'une panne. « Documents récents » se remplit depuis
 * le téléphone, sans réseau ; il reste vide sur un appareil neuf.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t, locale } = useI18n()
const session = useGnSession()
const connexion = useGnConnexion()
const thematiques = useGnThematiques()
const { aujourdhui } = useGnAujourdhui()
const { momentLisible } = useGnMomentLecture()
const bibliotheque = useGnDocuments()
const agenda = useGnAgenda()
const lectureDesSessions = useGnSessions()
const { dayLong, zoneOf } = useDateTime()

const BLOCS = BLOCS_DE_MA_JOURNEE.filter((bloc) => bloc !== 'lexique')
const stockage = { lire: lireCle, poser: poserCle }

const recents = ref<Recent[]>([])

const lignesDeDocuments = computed(() => {
  const enLigne = connexion.etat.value.enLigne
  return documentsRecents(recents.value, bibliotheque.documents.value, (id, version) =>
    lireProgression(stockage, id, version),
  ).map(({ document, page, lu }) => {
    const moment = momentLisible(lu) ?? ''
    return {
      document,
      marques: marquesDeLigne(document, {
        telecharge: bibliotheque.telecharges.value.has(document.id),
        nouveau: bibliotheque.nouveaux.value.has(document.id),
        enLigne,
      }),
      precision:
        page === null
          ? t('guide-nego.accueil.blocs.documents.lu', { moment })
          : t('guide-nego.accueil.blocs.documents.lu-page', { moment, page }),
    }
  })
})

// --- « Votre prochaine session de négociation » (FR-036) ---------------------

const maintenant = shallowRef(new Date())
let horloge: ReturnType<typeof setInterval> | undefined
onBeforeUnmount(() => clearInterval(horloge))

const prochaine = computed(() => {
  if (!session.connectee.value) return null
  const sessions = lectureDesSessions.sessions.value
  const fuseau = lectureDesSessions.fuseau.value ?? 'UTC'
  const trouvee = prochaineSession(
    sessionsDeLAgenda(agenda.agenda.value, sessions),
    sessions,
    thematiques.mesCodes.value,
    maintenant.value,
    fuseau,
  )
  if (!trouvee) return null
  const ville = lectureDesSessions.ville.value
  const zone = ville ?? fuseau.split('/').pop()?.replace(/_/g, ' ') ?? ''
  return {
    ...trouvee,
    fuseau,
    ville,
    etat: etatAffiche(trouvee.session, maintenant.value, fuseau),
    thematique: trouvee.session.theme ? thematiques.nomDe(trouvee.session.theme) : null,
    quand: t('guide-nego.accueil.blocs.prochaine-session.quand', {
      jour: dayLong(trouvee.session.start_at, fuseau),
      zone: zoneOf(zone),
    }),
  }
})

const sousTitre = computed(() => jourLisible(aujourdhui.value, locale.value))

const compte = session.compte
const sansCompte = computed(() => session.pret.value && !session.connectee.value)
const avatar = computed(() =>
  session.connectee.value ? { prenom: compte.value.prenom, nom: compte.value.nom } : undefined,
)

/** « mis à jour à 11:35 » en ligne, « connus à 11:35 » sans réseau (FR-017). */
const noteDesChangements = computed(() => {
  const moment = momentLisible(connexion.etat.value.luA)
  if (!moment) return undefined
  const cle = connexion.etat.value.enLigne ? 'mis-a-jour' : 'connus'
  return t(`guide-nego.accueil.blocs.changements.${cle}`, { moment })
})

/**
 * **Le choix des thématiques se propose une fois, et ne retient personne.**
 *
 * Qui arrive ici sans rien suivre est envoyé une fois à l'écran de choix, qui
 * porte sa sortie « Plus tard ». La clé se pose au moment de proposer : refuser
 * est aussi une réponse, et redemander à chaque ouverture enfermerait quelqu'un
 * dans une marche qu'il a déjà écartée.
 */
async function proposerLesThematiques(): Promise<void> {
  await session.assurer()
  if (!session.connectee.value) return
  await thematiques.assurer()

  const aFaire = propositionAFaire({
    connectee: session.connectee.value,
    pret: thematiques.pret.value,
    nombreSuivi: thematiques.mesCodes.value.length,
    dejaProposee: lireCle(CLE_THEMATIQUES_PROPOSEES) !== null,
  })
  if (!aFaire) return

  poserCle(CLE_THEMATIQUES_PROPOSEES)
  await navigateTo('/guide-nego/thematiques')
}

// La première venue passe par l'écran d'ouverture ; ensuite, l'accueil s'ouvre seul.
onMounted(() => {
  if (!lireCle(CLE_OUVERTURE_VUE)) return void navigateTo('/guide-nego/ouverture', { replace: true })
  session.relireAuRetourAuPremierPlan()
  horloge = setInterval(() => (maintenant.value = new Date()), 60_000)
  void session.assurer().then(() => {
    if (!session.connectee.value) return
    agenda.assurer()
    void lectureDesSessions.rafraichir()
  })
  recents.value = lireRecents(stockage)
  void bibliotheque.rafraichir()
  void proposerLesThematiques()
})

useHead({ title: t('guide-nego.accueil.titre') })
</script>

<template>
  <GnEcran :titre="t('guide-nego.accueil.titre')" :sous-titre="sousTitre" :avatar="avatar" cloche>
    <!-- Sans compte, l'écran s'ouvre quand même : il invite, il ne bloque pas. -->
    <template v-if="sansCompte">
      <p class="gn-journee__invitation">{{ t('guide-nego.accueil.sans-compte.texte') }}</p>
      <div class="gn-journee__sorties">
        <GnBouton variante="principal" vers="/guide-nego/compte">
          {{ t('guide-nego.accueil.sans-compte.creer-mon-compte') }}
        </GnBouton>
        <GnBouton variante="discret" vers="/guide-nego/connexion">
          {{ t('guide-nego.accueil.sans-compte.me-connecter') }}
        </GnBouton>
      </div>
    </template>

    <section v-for="bloc in BLOCS" :key="bloc" class="gn-journee__bloc">
      <GnEnteteGroupe
        :titre="t(`guide-nego.accueil.blocs.${bloc}.titre`)"
        :note="bloc === 'changements' ? noteDesChangements : undefined"
      />
      <div v-if="bloc === 'prochaine-session' && prochaine" class="gn-journee__prochaine">
        <p class="gn-journee__quand">{{ prochaine.quand }}</p>
        <GnLigneSession
          :session="prochaine.session"
          :etat="prochaine.etat"
          :fuseau="prochaine.fuseau"
          :ville="prochaine.ville"
          :thematique="prochaine.thematique"
          :vers="`/guide-nego/negociations/${prochaine.session.id}${prochaine.source === 'agenda' ? '?depuis=agenda' : ''}`"
        />
        <NuxtLink v-if="prochaine.source === 'agenda'" to="/guide-nego/negociations/agenda" class="gn-journee__agenda">
          <GnPicto nom="calendar" :taille="20" />
          {{ t('guide-nego.accueil.blocs.prochaine-session.mon-agenda') }}
        </NuxtLink>
      </div>
      <ul v-else-if="bloc === 'documents' && lignesDeDocuments.length" role="list">
        <li v-for="ligne in lignesDeDocuments" :key="ligne.document.id">
          <GnLigneDocument
            :document="ligne.document"
            :marques="ligne.marques"
            :vers="`/guide-nego/ressources/documents/${ligne.document.id}`"
            :morceaux="['pages', 'editeur']"
            :precision="ligne.precision"
          />
        </li>
      </ul>
      <p v-else-if="bloc !== 'documents' || bibliotheque.etat.value.pret" class="gn-journee__vide">{{ t(`guide-nego.accueil.blocs.${bloc}.vide`) }}</p>
    </section>

    <div class="gn-journee__lexique">
      <GnBouton variante="secondaire" vers="/guide-nego/lexique">
        <span class="gn-journee__aa" aria-hidden="true">Aa</span>
        {{ t('guide-nego.accueil.blocs.lexique.ouvrir') }}
      </GnBouton>
    </div>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-journee__invitation {
  padding-top: var(--gn-espace-16);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  max-width: var(--gn-mesure-lecture);
}

[data-app="guide-nego"] .gn-journee__sorties {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-journee__vide {
  padding-block: 10px;
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-journee__quand {
  padding-top: var(--gn-espace-8);
  color: var(--gn-accent);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-journee__quand::first-letter {
  text-transform: uppercase;
}

[data-app="guide-nego"] .gn-journee__prochaine .gn-ligne-session {
  border-bottom: none;
}

[data-app="guide-nego"] .gn-journee__agenda {
  width: fit-content;
  min-height: var(--gn-cible);
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--gn-accent);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-underline-offset: 4px;
}

[data-app="guide-nego"] .gn-journee__lexique {
  padding-block: var(--gn-espace-16);
}

/* « Aa » est un glyphe de bouton, hors échelle de texte (écarts 8 et 9). */
[data-app="guide-nego"] .gn-journee__aa {
  font-size: 18px;
  line-height: 1;
}
</style>
