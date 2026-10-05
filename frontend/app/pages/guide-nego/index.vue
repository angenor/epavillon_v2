<script setup lang="ts">
import type { OfficialSession } from '~/types/negotiation-sessions'
import { dayKeyInZone, formatTime } from '~/utils/datetime'
import {
  CLE_OUVERTURE_VUE,
  CLE_THEMATIQUES_PROPOSEES,
  lireCle,
  poserCle,
} from '~/utils/guide-nego/stockage'
import { avecBarreFinale, sansBarreFinale } from '~/utils/guide-nego/barre-finale'
import { propositionAFaire } from '~/utils/guide-nego/thematiques'
import { lireProgression, lireRecents, type Recent } from '~/utils/guide-nego/appareil-lecture'
import {
  changementsDuJour,
  compteARebours,
  documentsRecents,
  filDuJour,
  jourDeLaCop,
  jourLisible,
  momentDeLaCop,
  type ChangementDuJour,
  type CompteARebours,
} from '~/utils/guide-nego/journee'
import { premiereVivante, prochaineSession, sessionsDeLAgenda } from '~/utils/guide-nego/agenda'
import { etatAffiche, sessionsDuJour, titreDeSession } from '~/utils/guide-nego/sessions'
import { reunionsDuJour } from '~/utils/guide-nego/reunions'
import { activitesDuJour, etatDeLActivite } from '~/utils/guide-nego/pavillon'

/**
 * L'accueil (ADR-023, maquettes Nuit 01, 04, 05) : **le temps au centre**. Pendant la
 * COP, les minutes avant la prochaine session et le fil des trois agendas ; avant, les
 * jours avant l'ouverture. Sans compte, rien n'est fermé : l'écran invite.
 */
definePageMeta({
  layout: 'guide-nego',
  middleware: (to) =>
    sansBarreFinale(to.path) ? navigateTo({ path: avecBarreFinale(to.path), query: to.query, hash: to.hash }, { replace: true }) : undefined,
})
defineI18nRoute(false)

const { t, locale } = useI18n()
const session = useGnSession()
const thematiques = useGnThematiques()
const { momentLisible } = useGnMomentLecture()
const bibliotheque = useGnDocuments()
const agenda = useGnAgenda()
const lectureDesSessions = useGnSessions()
const reunions = useGnReunions()
const pavillon = useGnPavillon()
const notifications = useGnNotifications()
const { timeRange } = useDateTime()

const stockage = { lire: lireCle, poser: poserCle }
const recents = ref<Recent[]>([])

const maintenant = shallowRef(new Date())
let horloge: ReturnType<typeof setInterval> | undefined
onBeforeUnmount(() => clearInterval(horloge))

const edition = lectureDesSessions.edition
const fuseau = computed(() => lectureDesSessions.fuseau.value ?? edition.value?.timezone ?? 'UTC')
const ville = computed(() => lectureDesSessions.ville.value ?? edition.value?.city ?? null)
const connectee = computed(() => session.connectee.value)
const sansCompte = computed(() => session.pret.value && !connectee.value)
const moment = computed(() => momentDeLaCop(edition.value, maintenant.value))
const editionDatee = computed(() => {
  const e = edition.value
  return e?.debut && e.fin ? { ...e, debut: e.debut, fin: e.fin } : null
})
const heureDe = (iso: string | Date) => formatTime(iso, { timeZone: fuseau.value, locale: locale.value })
const majuscule = (texte: string) => texte.charAt(0).toLocaleUpperCase(locale.value) + texte.slice(1)

// --- L'en-tête ----------------------------------------------------------------

const jour = computed(() => jourLisible(maintenant.value, locale.value, edition.value ? fuseau.value : undefined))
const contexte = computed(() => {
  const e = edition.value
  if (!e) return ''
  if (moment.value === 'avant') return t('guide-nego.accueil.contexte.avant', { edition: e.libelle })
  const rang = jourDeLaCop(e, maintenant.value, fuseau.value)
  if (!rang) return e.libelle
  return connectee.value
    ? t('guide-nego.accueil.contexte.pendant', { edition: e.libelle, ...rang })
    : t('guide-nego.accueil.contexte.pendant-court', { edition: e.libelle, jour: rang.jour })
})
const avatar = computed(() =>
  connectee.value ? { prenom: session.compte.value.prenom, nom: session.compte.value.nom } : undefined,
)
// La maquette d'avant la COP n'a que la loupe : la cloche n'y paraît que s'il y a du nouveau.
const cloche = computed(() => moment.value === 'pendant' || notifications.nonLues.value > 0)

// --- Le compte à rebours ------------------------------------------------------

function texteDuCompte(c: CompteARebours): string {
  if (c.unite === 'minutes') return t('guide-nego.accueil.compte.minutes', { n: c.minutes })
  if (c.unite === 'heures')
    return c.minutes
      ? t('guide-nego.accueil.compte.heures', { h: c.heures, m: String(c.minutes).padStart(2, '0') })
      : t('guide-nego.accueil.compte.heures-pile', { h: c.heures })
  if (c.unite === 'demain') return t('guide-nego.accueil.compte.demain')
  return t('guide-nego.accueil.compte.jours', { n: c.jours }, c.jours)
}

/** Connecté : la mienne (agenda, sinon thématiques) ; sans compte : celle de toute la COP. */
const visee = computed<{ session: OfficialSession; source: 'agenda' | 'thematiques' | 'cop' } | null>(() => {
  const sessions = lectureDesSessions.sessions.value
  if (!connectee.value) {
    const premiere = premiereVivante(sessions, maintenant.value, fuseau.value)
    return premiere ? { session: premiere, source: 'cop' } : null
  }
  return prochaineSession(
    sessionsDeLAgenda(agenda.agenda.value, sessions),
    sessions,
    thematiques.mesCodes.value,
    maintenant.value,
    fuseau.value,
  )
})

const compte = computed(() => {
  const v = visee.value
  if (!v) return null
  const s = v.session
  const qui = v.source === 'cop' ? 'cop' : 'mienne'
  const plage = timeRange(s.start_at, s.end_at, fuseau.value, ville.value ?? undefined)
  const base = {
    titre: titreDeSession(s, locale.value),
    quand: s.venue ? t('guide-nego.accueil.compte.plage-et-salle', { plage, salle: s.venue }) : plage,
    vers: `/guide-nego/negociations/${s.id}${v.source === 'agenda' ? '?depuis=agenda' : ''}`,
  }
  if (etatAffiche(s, maintenant.value, fuseau.value) !== 'en-cours') {
    const valeur = texteDuCompte(compteARebours(s.start_at, maintenant.value, fuseau.value))
    return { ...base, libelle: t(`guide-nego.accueil.compte.libelle.${qui}`), valeur }
  }
  if (s.end_at) {
    const valeur = texteDuCompte(compteARebours(s.end_at, maintenant.value, fuseau.value))
    return { ...base, libelle: t(`guide-nego.accueil.compte.libelle.${qui}-finit`), valeur }
  }
  return { ...base, libelle: t(`guide-nego.accueil.compte.libelle.${qui}-depuis`), valeur: heureDe(s.start_at) }
})

const ouverture = computed(() => {
  const e = editionDatee.value
  if (!e || moment.value !== 'avant') return null
  const memeMois = dayKeyInZone(e.debut, fuseau.value).slice(0, 7) === dayKeyInZone(e.fin, fuseau.value).slice(0, 7)
  const format = (iso: string, mois: boolean) =>
    new Intl.DateTimeFormat(locale.value, {
      weekday: 'long',
      day: 'numeric',
      month: mois ? 'long' : undefined,
      timeZone: fuseau.value,
    }).format(new Date(iso))
  const dates = t('guide-nego.accueil.ouverture.dates', { du: format(e.debut, !memeMois), au: format(e.fin, true) })
  return {
    libelle: t('guide-nego.accueil.ouverture.libelle', { edition: e.libelle }),
    valeur: texteDuCompte(compteARebours(e.debut, maintenant.value, fuseau.value)),
    quand: e.city ? t('guide-nego.accueil.ouverture.lieu-et-dates', { ville: e.city, dates }) : majuscule(dates),
  }
})

const sessionsPretes = computed(() => lectureDesSessions.etat.value.pret)

// --- Le fil du jour et les changements ---------------------------------------

/** Les miennes : dans l'agenda, ou d'une thématique suivie ; sans thématique ni compte, toutes. */
const miennes = computed(() => {
  const sessions = lectureDesSessions.sessions.value
  const codes = thematiques.mesCodes.value
  if (!connectee.value || codes.length === 0) return sessions
  const suivies = new Set(sessionsDeLAgenda(agenda.agenda.value, sessions).map((s) => s.id))
  return sessions.filter((s) => suivies.has(s.id) || (s.theme !== null && codes.includes(s.theme)))
})

const PISTES = ['negociation', 'francophonie', 'pavillon'] as const

const fil = computed(() => {
  const z = fuseau.value
  const leJour = dayKeyInZone(maintenant.value, z)
  const marquee = visee.value?.session.id
  const pistes = [
    sessionsDuJour(miennes.value, leJour, z)
      .filter((s) => s.status !== 'cancelled')
      .map((s) => ({ debut: s.start_at, fin: s.end_at, marque: s.id === marquee })),
    reunionsDuJour(reunions.reunions.value, leJour, z)
      .filter((r) => r.status !== 'cancelled')
      .map((r) => ({ debut: r.start_at, fin: r.end_at })),
    activitesDuJour(pavillon.activites.value, leJour, z)
      .filter((a) => !['annulee', 'reportee'].includes(etatDeLActivite(a, maintenant.value)))
      .map((a) => ({ debut: a.starts_at, fin: a.ends_at })),
  ]
  return {
    calcul: filDuJour(pistes, maintenant.value, z),
    pistes: PISTES.map((cle, i) => {
      const n = pistes[i]?.length ?? 0
      return { libelle: t(`guide-nego.accueil.fil.${cle}`), resume: t(`guide-nego.accueil.fil.resume.${cle}`, { count: n }, n) }
    }),
  }
})

function titreDuChangement(c: ChangementDuJour): string {
  const titre = titreDeSession(c.session, locale.value)
  return t(`guide-nego.accueil.changements.genre.${c.genre}`, { titre, salle: c.salle ?? '', heure: heureDe(c.debut) })
}

const changements = computed(() =>
  changementsDuJour(miennes.value, maintenant.value, fuseau.value).map((c) => ({
    cle: c.session.id,
    titre: titreDuChangement(c),
    precision: t(`guide-nego.accueil.changements.origine.${c.origine}`, { heure: heureDe(c.debut) }),
    point: c.genre === 'annulee' ? ('danger' as const) : ('attention' as const),
    vers: `/guide-nego/negociations/${c.session.id}`,
  })),
)

const lueA = computed(() => {
  const affichage = lectureDesSessions.affichage.value
  return momentLisible(affichage.etat === 'sert' ? affichage.luA : null)
})

// --- Documents récents --------------------------------------------------------

const lignesDeDocuments = computed(() =>
  documentsRecents(recents.value, bibliotheque.documents.value, (id, version) => lireProgression(stockage, id, version)).map(
    ({ document, page, lu }) => {
      const quand = momentLisible(lu) ?? ''
      return {
        document,
        precision:
          page === null
            ? t('guide-nego.accueil.blocs.documents.lu', { moment: quand })
            : t('guide-nego.accueil.blocs.documents.lu-page', { moment: quand, page }),
      }
    },
  ),
)

/**
 * **Le choix des thématiques se propose une fois, et ne retient personne.** La clé se
 * pose au moment de proposer : refuser est aussi une réponse.
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
  void lectureDesSessions.rafraichir()
  void reunions.rafraichir()
  void pavillon.rafraichir()
  void session.assurer().then(() => {
    if (!session.connectee.value) return
    agenda.assurer()
    void notifications.assurer()
  })
  recents.value = lireRecents(stockage)
  void bibliotheque.rafraichir()
  void proposerLesThematiques()
})

useHead({ title: t('guide-nego.accueil.titre') })
</script>

<template>
  <GnEcran :titre="t('guide-nego.accueil.titre')">
    <template #entete>
      <GnJourneeEntete
        :titre="t('guide-nego.accueil.titre')"
        :jour="jour"
        :contexte="contexte"
        :avatar="avatar"
        :cloche="cloche"
        :non-lues="notifications.nonLues.value"
      />
    </template>

    <div class="gn-journee">
      <template v-if="ouverture && editionDatee">
        <GnJourneeCompteARebours :libelle="ouverture.libelle" :valeur="ouverture.valeur" :quand="ouverture.quand" />
        <GnJourneeInvitation v-if="sansCompte" />
        <GnJourneeAvantLaCop :edition="editionDatee" :maintenant="maintenant" :fuseau="fuseau" />
      </template>

      <template v-else>
        <GnJourneeCompteARebours
          v-if="compte"
          :libelle="compte.libelle"
          :valeur="compte.valeur"
          :titre="compte.titre"
          :quand="compte.quand"
          :vers="compte.vers"
        />
        <section v-else-if="sessionsPretes" class="gn-journee__bloc">
          <h2 class="gn-journee__bloc-titre">{{ t('guide-nego.accueil.blocs.prochaine-session.titre') }}</h2>
          <p class="gn-journee__bloc-texte">{{ t('guide-nego.accueil.blocs.prochaine-session.vide') }}</p>
        </section>

        <GnJourneeInvitation v-if="sansCompte" />

        <GnJourneeFil :titre="t('guide-nego.accueil.blocs.trois-agendas.titre')" :fil="fil.calcul" :heure="heureDe(maintenant)" :pistes="fil.pistes" />

        <section class="gn-journee__section">
          <div class="gn-journee__tete">
            <h2 class="gn-journee__titre">{{ t('guide-nego.accueil.blocs.changements.titre') }}</h2>
            <span v-if="lueA" class="gn-journee__note">{{ lueA }}</span>
          </div>
          <ul v-if="changements.length" role="list">
            <li v-for="c in changements" :key="c.cle">
              <GnJourneeLigne :vers="c.vers" :titre="c.titre" :precision="c.precision" :point="c.point" />
            </li>
          </ul>
          <p v-else-if="sessionsPretes" class="gn-journee__vide">{{ t('guide-nego.accueil.blocs.changements.vide') }}</p>
        </section>
      </template>

      <section class="gn-journee__section">
        <h2 class="gn-journee__titre gn-journee__titre--seul">{{ t('guide-nego.accueil.blocs.documents.titre') }}</h2>
        <p v-if="!lignesDeDocuments.length && bibliotheque.etat.value.pret" class="gn-journee__vide">
          {{ t('guide-nego.accueil.blocs.documents.vide') }}
        </p>
        <ul role="list">
          <li v-for="l in lignesDeDocuments" :key="l.document.id">
            <GnJourneeLigne
              :vers="`/guide-nego/ressources/documents/${l.document.id}`"
              picto="doc"
              :titre="l.document.title"
              :precision="l.precision"
            />
          </li>
          <li>
            <GnJourneeLigne
              vers="/guide-nego/lexique"
              picto="translate"
              :titre="t('guide-nego.accueil.blocs.lexique.titre')"
              :precision="t('guide-nego.accueil.blocs.lexique.precision')"
            />
          </li>
        </ul>
      </section>
    </div>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-journee {
  padding-top: var(--gn-entre-blocs);
  display: flex;
  flex-direction: column;
  gap: var(--gn-entre-blocs);
}

[data-app="guide-nego"] .gn-journee__bloc {
  padding: 18px var(--gn-espace-16);
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  background: var(--gn-fond-2);
  border-radius: var(--gn-rayon-24);
}

[data-app="guide-nego"] .gn-journee__bloc-titre {
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-16);
  line-height: var(--gn-interligne-16);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-journee__bloc-texte {
  color: var(--gn-texte-lecture);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-journee__section {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-4);
}

[data-app="guide-nego"] .gn-journee__tete {
  padding-bottom: var(--gn-espace-4);
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-journee__titre {
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-18);
  line-height: var(--gn-interligne-18);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-journee__titre--seul {
  margin-bottom: var(--gn-espace-4);
}

[data-app="guide-nego"] .gn-journee__note {
  flex: none;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
}

[data-app="guide-nego"] .gn-journee__vide {
  padding-block: 10px;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
}
</style>
