<script setup lang="ts">
import type { CorrectionNote, LibraryDocument } from '~/types/negotiation-documents'
import { compterUneBascule } from '~/utils/guide-nego/appareil-lecture'
import { sectionDeLaPage } from '~/utils/guide-nego/forme-lisible'
import { tailleLisible } from '~/utils/guide-nego/place'
import { chercherDansLeDocument, pagesCherchees, passageDeLaPage, type Occurrence } from '~/utils/guide-nego/lecteur'
import { creerLAttente, DELAI_DES_SORTIES_MS, type EtatDAttente } from '~/utils/guide-nego/pdf/attente'
import type { CauseDeBascule } from '~/utils/guide-nego/pdf/bascule'
import type { Minuterie } from '~/utils/guide-nego/pdf/gestes'
import type { SuiviDesPlages } from '~/utils/guide-nego/pdf/transport'
import { lireCle, poserCle } from '~/utils/guide-nego/stockage'
import GnLecteurPages from '~/components/guide-nego/GnLecteurPages.client.vue'

/**
 * Le lecteur — maquette 04. Sans barre d'onglets : une ligne d'en-tête, les pages
 * d'origine (ADR-022) ou « Texte agrandi », la barre de lecture en bas, et ce qu'elle
 * ouvre — sommaire, recherche, réglages.
 */
// Le lecteur tient lui-même sa position — la page de reprise : ni le routeur ni le
// navigateur ne la recalent après lui.
definePageMeta({ layout: 'guide-nego', scrollToTop: false })
defineI18nRoute(false)

const { t, locale } = useI18n()
const route = useRoute()
const id = computed(() => String(route.params.id ?? ''))
const versLaFiche = computed(() => `/guide-nego/ressources/documents/${id.value}`)

const { documents, documentDe, notesDe, rafraichir: relireLaBibliotheque } = useGnDocuments()
const copies = useGnCopies()
const connexion = useGnConnexion()
const session = useGnSession()
const acces = useGnAcces()
const { momentLisible } = useGnMomentLecture()
const lecteur = useGnLecteur(id)
const { etat, lecture, reprise, pageEnCours, suivreLaPage } = lecteur

const document = computed<LibraryDocument | null>(() => documentDe(id.value))
const titre = computed(() => document.value?.title ?? t('guide-nego.lecteur.titre'))
const enLigne = computed(() => connexion.etat.value.enLigne)

const { taille } = useGnTailleDeLecture()

const elementDeLaPage = (index: number) => window.document.getElementById(`page-${index}`)

// --- Les pages, ou « Texte agrandi » -------------------------------------------

const pages = ref<InstanceType<typeof GnLecteurPages> | null>(null)
const sourceDuPdf = computed(() => (etat.value.etat === 'pret' ? etat.value.pdf : null))
const minuterie: Minuterie = {
  planifier: (fn, ms) => setTimeout(fn, ms),
  annuler: (poignee) => clearTimeout(poignee as ReturnType<typeof setTimeout>),
}

/** Ce téléphone n'affiche pas les pages : « Texte agrandi » s'il est offert, sinon le dire (FR-012 bis). */
const bascule = ref<CauseDeBascule | 'indisponible' | null>(null)
const annonceFermee = ref(false)
const impossible = computed(() => bascule.value !== null && !lecture.value?.large_text)

// En ligne, l'attente de la première page se voit, et offre le texte au bout de 3 s (FR-009 bis).
const attente = ref<EtatDAttente | null>(null)
let lAttente: ReturnType<typeof creerLAttente> | null = null
const plages = ref<{ recu: number; demande: number }>({ recu: 0, demande: 0 })

const modeAffiche = computed<'attente' | 'texte' | 'pages'>(() => {
  if (bascule.value !== null) return 'texte'
  return attente.value?.mode ?? 'pages'
})

// Le réseau tombé sur des pages non reçues ; revenu, le visionneur repart de la page lue.
const reseauPerdu = ref(false)
const cleDesPages = ref(0)
const pageDeDepart = ref(1)

function commencerLaLecture(): void {
  lAttente?.arreter()
  lAttente = null
  attente.value = null
  bascule.value = null
  annonceFermee.value = false
  reseauPerdu.value = false
  plages.value = { recu: 0, demande: 0 }
  pageDeDepart.value = reprise.value?.index ?? 1
  if (etat.value.etat !== 'pret' || etat.value.source !== 'reseau') return
  // La forme lisible est déjà arrivée : les 3 s se comptent depuis l'ouverture, pas depuis elle.
  const delai = Math.max(0, DELAI_DES_SORTIES_MS - lecteur.ouvertDepuis())
  lAttente = creerLAttente({ minuterie, delai, surChangement: (e) => (attente.value = e) })
  attente.value = lAttente.etat()
  lAttente.demarrer()
}

function surLesPlages(suivi: SuiviDesPlages): void {
  plages.value = { recu: suivi.recu, demande: suivi.demande }
}

function surLaPremierePage(): void {
  const avant = modeAffiche.value
  lAttente?.pagePrete()
  if (avant !== 'pages' && modeAffiche.value === 'pages') pages.value?.allerALaPage(pageEnCours.value)
  if (avant === 'attente' || avant === 'pages') lecteur.commencerLeSuivi()
}

function surLaBascule(cause: CauseDeBascule | 'indisponible'): void {
  // Seule la seconde sécurité se compte : la détection, elle, est sûre d'elle.
  if (cause !== 'indisponible') compterUneBascule({ lire: lireCle, poser: poserCle }, cause, new Date().toISOString())
  lAttente?.arreter()
  bascule.value = cause
  void nextTick().then(() => {
    elementDeLaPage(pageEnCours.value)?.scrollIntoView({ block: 'start', behavior: 'instant' })
    lecteur.commencerLeSuivi()
  })
}

function lireLeTexteEnAttendant(): void {
  lAttente?.lireLeTexte()
  void nextTick().then(() => {
    elementDeLaPage(pageEnCours.value)?.scrollIntoView({ block: 'start', behavior: 'instant' })
    lecteur.commencerLeSuivi()
  })
}

function resterSurLeTexte(): void {
  lAttente?.resterSurLeTexte()
}

function auRetourDuReseau(): void {
  pageDeDepart.value = pageEnCours.value
  reseauPerdu.value = false
  cleDesPages.value += 1
}

onBeforeUnmount(() => lAttente?.arreter())

/** Aller en tête d'une page, sans animation : l'observateur ne voit pas défiler les pages voisines. */
function allerALaPage(index: number): Promise<void> {
  if (modeAffiche.value === 'pages') return lecteur.sauter(index, () => pages.value?.allerALaPage(index))
  return lecteur.sauter(index, () => elementDeLaPage(index)?.scrollIntoView({ block: 'start', behavior: 'instant' }))
}

// Une autre taille recompose le texte : on reste sur la page qu'on lisait (FR-042).
watch(taille, async () => {
  const page = pageEnCours.value
  await nextTick()
  await allerALaPage(page)
})

onMounted(() => {
  void relireLaBibliotheque()
  void lecteur.ouvrir()
})
onBeforeRouteLeave(() => lecteur.arreterLeSuivi())

// Le réseau revenu, ou la demande faite ici partie à son retour : le document s'ouvre.
watch(
  [() => copies.copies.value.some((c) => c.id === id.value), () => connexion.etat.value.enLigne],
  ([gardee, enLigne]) => {
    if ((gardee || enLigne) && etat.value.etat === 'absent') void lecteur.ouvrir()
  },
)

// Un lien, ou un réservé sans l'accès, se dit sur sa fiche : elle sait l'ouvrir ou
// montrer le verrou. Hors connexion, la liste gardée le dit sans attendre l'API.
const pasPourLeLecteur = computed(() => {
  const d = document.value
  return !!d && (d.source !== 'file' || !d.accessible)
})
watch(
  [() => etat.value.etat, pasPourLeLecteur],
  ([e, horsLecteur]) => {
    if (e === 'lien' || e === 'reserve' || (e === 'absent' && horsLecteur)) {
      void navigateTo(versLaFiche.value, { replace: true })
    }
  },
)

// Un réservé ouvert se ferme sur son verrou dès que l'accès ou la session tombe.
watch([() => acces.ouvert.value, () => session.connectee.value], ([ouvert, connectee]) => {
  if (lecture.value && document.value?.restricted && (!ouvert || !connectee)) {
    void navigateTo(versLaFiche.value, { replace: true })
  }
})

const total = computed(() => lecture.value?.page_count ?? 0)
const pageCourante = computed(() => lecture.value?.pages.find((p) => p.index === pageEnCours.value) ?? null)
// Lues avec la liste, pas avec la copie : une note posée ou retirée ne demande aucun retéléchargement.
const notesParPage = computed(() => {
  const parPage = new Map<number, CorrectionNote[]>()
  for (const note of notesDe(id.value)) parPage.set(note.page_index, [...(parPage.get(note.page_index) ?? []), note])
  return parPage
})
const section = computed(() =>
  lecture.value ? (sectionDeLaPage(lecture.value.outline, pageEnCours.value)?.title ?? null) : null,
)

// --- La reprise -------------------------------------------------------------

const repriseAffichee = ref(false)

const uneImage = () => new Promise((fin) => requestAnimationFrame(() => requestAnimationFrame(fin)))

watch(lecture, async (lue) => {
  if (!lue) return
  repriseAffichee.value = !!reprise.value
  commencerLaLecture()
  // En « Pages », le suivi commence à la première page dessinée, à sa place.
  if (modeAffiche.value !== 'texte') return
  await nextTick()
  // La police chargée recompose le texte : un recalage fait avant glisserait de plusieurs écrans.
  await window.document.fonts?.ready
  if (reprise.value) window.document.getElementById(`page-${reprise.value.index}`)?.scrollIntoView({ block: 'start' })
  else window.scrollTo({ top: 0 })
  await uneImage()
  lecteur.commencerLeSuivi()
})

// Partie de la page de reprise, la personne lit : la ligne n'a plus rien à dire. Seul
// son geste compte — le texte qui se recompose au chargement de la police fait aussi
// passer des pages sous l'observateur.
const aDefile = ref(false)
const GESTES = ['wheel', 'touchmove', 'keydown'] as const
const noterLeGeste = () => (aDefile.value = true)
onMounted(() => GESTES.forEach((g) => window.addEventListener(g, noterLeGeste, { passive: true, once: true })))
onBeforeUnmount(() => GESTES.forEach((g) => window.removeEventListener(g, noterLeGeste)))

watch(pageEnCours, (page) => {
  if (aDefile.value && reprise.value && page !== reprise.value.index) repriseAffichee.value = false
})

function repartirDuDebut(): void {
  repriseAffichee.value = false
  lecteur.oublierLaReprise()
  if (modeAffiche.value === 'pages') void allerALaPage(1)
  else window.scrollTo({ top: 0 })
}

const texteDeReprise = computed(() => {
  const r = reprise.value
  if (!r) return ''
  const moment = momentLisible(r.a)
  return moment
    ? t('guide-nego.lecteur.reprise.texte', { page: r.label, moment })
    : t('guide-nego.lecteur.reprise.sans-moment', { page: r.label })
})

// --- La barre ---------------------------------------------------------------

const barreDepliee = ref(false)

type Action = 'sommaire' | 'rechercher' | 'reglages'
// Sans texte extrait, ni sommaire ni recherche (FR-018) ; les réglages portent au moins le thème.
const actions = computed<Action[]>(() => {
  const lue = lecture.value
  if (!lue?.has_text) return ['reglages']
  return [...(lue.outline.length ? (['sommaire'] as const) : []), 'rechercher', 'reglages']
})

const sommaireOuvert = ref(false)
const rechercheOuverte = ref(false)
const reglagesOuverts = ref(false)

function agir(action: Action): void {
  barreDepliee.value = false
  if (action === 'sommaire') sommaireOuvert.value = true
  else if (action === 'rechercher') rechercheOuverte.value = true
  else reglagesOuverts.value = true
}

async function allerAuChapitre(index: number): Promise<void> {
  sommaireOuvert.value = false
  repriseAffichee.value = false
  await nextTick()
  await allerALaPage(index)
  // Le focus suit le saut : sans lui, le prochain Tab repartirait du haut du document.
  elementDeLaPage(index)?.focus({ preventScroll: true })
}

// --- La recherche dans le document -------------------------------------------

const expression = ref('')
const rangCourant = ref<number | null>(null)
// Cherché à chaque frappe, sur l'index du document : pas de liste périmée qui se touche.
const passages = computed(() => (lecture.value ? chercherDansLeDocument(lecture.value, expression.value) : []))
watch(expression, () => (rangCourant.value = null))

const ici = computed(() => passageDeLaPage(passages.value, pageEnCours.value))
const passageCourant = computed(() => (rangCourant.value === null ? null : (passages.value[rangCourant.value] ?? null)))

// Construits une fois par recherche ; passer d'une occurrence à l'autre ne touche que deux pages.
const surlignagesParPage = computed(() => {
  const parPage = new Map<number, Occurrence[]>()
  for (const p of passages.value) {
    const liste = parPage.get(p.page) ?? []
    liste.push(p)
    parPage.set(p.page, liste)
  }
  return parPage
})

async function allerAuPassage(rang: number): Promise<void> {
  rechercheOuverte.value = false
  repriseAffichee.value = false
  rangCourant.value = rang
  const passage = passages.value[rang]
  if (!passage) return
  await nextTick()
  // Le passage marqué sur la page du PDF vient avec le récit 2 ; d'ici là, sa page s'ouvre.
  if (modeAffiche.value === 'pages') return allerALaPage(passage.page)
  const courante = window.document.querySelector<HTMLElement>('[data-occurrence-courante]')
  // Un tableau refermé à la main ne se rouvre pas par `:open` : sa valeur n'a pas changé.
  courante?.closest('details')?.setAttribute('open', '')
  await lecteur.sauter(passage.page, () =>
    courante
      ? courante.scrollIntoView({ block: 'start', behavior: 'instant' })
      : elementDeLaPage(passage.page)?.scrollIntoView({ block: 'start', behavior: 'instant' }),
  )
  courante?.focus({ preventScroll: true })
}

// D'un bout à l'autre, comme la recherche d'un navigateur.
function allerDe(pas: number): void {
  const total = passages.value.length
  if (rangCourant.value === null || !total) return
  void allerAuPassage((rangCourant.value + pas + total) % total)
}
const allerAuPrecedent = () => allerDe(-1)
const allerAuSuivant = () => allerDe(1)

// Échap referme la barre d'occurrence, quand aucune feuille ni panneau ne l'a pris.
function auClavier(evenement: KeyboardEvent): void {
  if (evenement.key === 'Escape' && !evenement.defaultPrevented && rangCourant.value !== null) rangCourant.value = null
}
onMounted(() => window.addEventListener('keydown', auClavier))
onBeforeUnmount(() => window.removeEventListener('keydown', auClavier))

// --- Le terme anglais touché -------------------------------------------------

const terme = ref<string | null>(null)
const termeOuvert = computed({
  get: () => terme.value !== null,
  set: (ouvert: boolean) => {
    if (!ouvert) terme.value = null
  },
})

/** Les pages où le terme paraît : un fait du document, jamais une définition inventée. */
const pagesDuTerme = computed(() => {
  if (!terme.value || !lecture.value) return ''
  const etiquettes = [...new Set(chercherDansLeDocument(lecture.value, terme.value).map((p) => p.etiquette))]
  return new Intl.ListFormat(locale.value, { type: 'conjunction' }).format(etiquettes)
})

/** Un toucher sur le texte bascule la barre ; un toucher sur un lien ou un bouton fait ce qu'il dit. */
function basculerLaBarre(evenement: MouseEvent): void {
  const cible = evenement.target as Element | null
  if (cible?.closest('a, button, summary, details, [role="button"]')) return
  barreDepliee.value = !barreDepliee.value
}

// --- Pas sur le téléphone ---------------------------------------------------

const enAttente = computed(() => copies.enAttente.value.includes(id.value))
const demandeEnCours = ref(false)

const lisibleMaintenant = computed<LibraryDocument | null>(() => {
  const gardees = new Set(copies.copies.value.map((c) => c.id))
  return documents.value.find((d) => d.id !== id.value && gardees.has(d.id)) ?? null
})

const texteDeLAbsence = computed(() => {
  const d = document.value
  if (!d) return t('guide-nego.lecteur.absent.texte-inconnu')
  const morceaux = [
    d.page_count === null ? null : t('guide-nego.lecteur.absent.pages', { count: d.page_count }, d.page_count),
    d.reading_bytes === null ? null : tailleLisible(d.reading_bytes, locale.value),
  ].filter((m): m is string => !!m)
  return morceaux.length
    ? t('guide-nego.lecteur.absent.texte', { titre: d.title, details: morceaux.join(', ') })
    : t('guide-nego.lecteur.absent.texte-sans-details', { titre: d.title })
})

const issue = ref<'place' | 'panne' | 'refus' | null>(null)

async function telechargerAuRetour(): Promise<void> {
  const d = document.value
  if (!d || demandeEnCours.value) return
  demandeEnCours.value = true
  issue.value = null
  try {
    const suite = await copies.telecharger(d)
    if (suite === 'place' || suite === 'panne' || suite === 'refus') issue.value = suite
    else if (suite === 'impossible') issue.value = 'refus'
  } finally {
    demandeEnCours.value = false
  }
}

useHead({ title: titre })
</script>

<template>
  <GnEcran
    :titre="titre"
    compact
    :retour="versLaFiche"
    :onglets="false"
    :ce-qui-se-lit="etat.etat === 'absent' ? t('guide-nego.lecteur.hors-connexion') : undefined"
  >
    <GnChargement
      v-if="etat.etat === 'chargement' || etat.etat === 'lien' || etat.etat === 'reserve'"
      forme="squelette"
      :lignes="8"
      :libelle="t('guide-nego.lecteur.chargement')"
      class="gn-lecteur__attente"
    />

    <div v-else-if="etat.etat === 'absent'" class="gn-lecteur__absent">
      <GnEtatVide picto="wifi-off" :titre="t('guide-nego.lecteur.absent.titre')" :texte="texteDeLAbsence" />
      <p v-if="issue" class="gn-lecteur__issue" role="status">{{ t(`guide-nego.lecteur.absent.issue.${issue}`) }}</p>
      <p v-if="lisibleMaintenant" class="gn-lecteur__lisible">
        {{ t('guide-nego.lecteur.absent.lisible', { titre: lisibleMaintenant.title }) }}
      </p>
      <div class="gn-lecteur__actions">
        <p v-if="enAttente" class="gn-lecteur__en-attente">
          <GnPicto nom="clock" :taille="20" />
          {{ t('guide-nego.lecteur.absent.en-attente') }}
        </p>
        <GnBouton
          v-else-if="document && document.source === 'file'"
          picto="download"
          :chargement="demandeEnCours"
          @clic="telechargerAuRetour"
        >
          {{ t(enLigne ? 'guide-nego.lecteur.absent.telecharger' : 'guide-nego.lecteur.absent.telecharger-au-retour') }}
        </GnBouton>
        <GnBouton
          v-if="lisibleMaintenant"
          variante="secondaire"
          :vers="`/guide-nego/ressources/documents/${lisibleMaintenant.id}/lire`"
        >
          {{ t('guide-nego.lecteur.absent.lire', { titre: lisibleMaintenant.title }) }}
        </GnBouton>
      </div>
    </div>

    <GnEtatVide
      v-else-if="etat.etat === 'introuvable'"
      picto="doc"
      :titre="t('guide-nego.lecteur.introuvable.titre')"
      :texte="t('guide-nego.lecteur.introuvable.texte')"
      :sortie="t('guide-nego.lecteur.introuvable.sortie')"
      sortie-vers="/guide-nego/ressources/documents"
    />

    <GnEtatErreur
      v-else-if="etat.etat === 'erreur'"
      :titre="t('guide-nego.lecteur.erreur.titre')"
      :texte="etat.message ?? t('guide-nego.lecteur.erreur.texte')"
      :sortie="t('guide-nego.lecteur.erreur.reessayer')"
      @sortie="lecteur.ouvrir()"
    />

    <GnEtatErreur
      v-else-if="lecture && impossible"
      :titre="t('guide-nego.lecteur.impossible.titre')"
      :texte="t('guide-nego.lecteur.impossible.texte')"
      :sortie="t('guide-nego.lecteur.impossible.sortie')"
      :sortie-vers="versLaFiche"
    />

    <template v-else-if="lecture">
      <GnAnnonce
        v-if="bascule !== null && !annonceFermee"
        :texte="t('guide-nego.lecteur.bascule')"
        @fermer="annonceFermee = true"
      />

      <GnAttentePages
        v-if="modeAffiche === 'attente'"
        :recu="plages.recu"
        :demande="plages.demande"
        :sorties="attente?.sorties"
        :texte-offert="lecture.large_text"
        @lire-le-texte="lireLeTexteEnAttendant"
        @telecharger="telechargerAuRetour"
      />

      <GnLecteurTexte
        v-if="modeAffiche === 'texte'"
        :lecture="lecture"
        :taille="taille"
        :surlignages-par-page="rangCourant === null ? null : surlignagesParPage"
        :passage-courant="passageCourant"
        :notes-par-page="notesParPage"
        :suivre-la-page="suivreLaPage"
        @terme="terme = $event"
        @basculer="basculerLaBarre"
        @redimension="allerALaPage(pageEnCours)"
      />

      <GnLecteurPages
        v-if="sourceDuPdf && bascule === null"
        ref="pages"
        :key="cleDesPages"
        :source="sourceDuPdf"
        :page-initiale="pageDeDepart"
        :cachee="modeAffiche !== 'pages'"
        @page="lecteur.poserLaPage"
        @premiere-page="surLaPremierePage"
        @plages="surLesPlages"
        @bascule="surLaBascule"
        @basculer-la-barre="barreDepliee = !barreDepliee"
        @reseau-perdu="reseauPerdu = true"
        @reseau-revenu="auRetourDuReseau"
      />

      <GnAttentePages
        v-if="modeAffiche === 'texte' && attente && !attente.pagePrete && !attente.resteSurLeTexte && !barreDepliee"
        variante="ligne"
        :recu="plages.recu"
        :demande="plages.demande"
        @rester="resterSurLeTexte"
      />

      <p v-if="reseauPerdu && modeAffiche === 'pages' && !barreDepliee" class="gn-lecteur__reprise" role="status">
        <GnPicto nom="wifi-off" :taille="20" />
        <span class="gn-lecteur__reprise-texte">{{ t('guide-nego.lecteur.sans-reseau.texte') }}</span>
        <button v-if="!enAttente" type="button" class="gn-lecteur__debut" @click="telechargerAuRetour">
          {{ t('guide-nego.lecteur.sans-reseau.telecharger') }}
        </button>
      </p>

      <p v-if="repriseAffichee && reprise && !barreDepliee" class="gn-lecteur__reprise" role="status">
        <GnPicto nom="bookmark" :taille="20" />
        <span class="gn-lecteur__reprise-texte">{{ texteDeReprise }}</span>
        <button type="button" class="gn-lecteur__debut" @click="repartirDuDebut">
          {{ t('guide-nego.lecteur.reprise.debut') }}
        </button>
      </p>
      <GnBarreLecture
        v-model:depliee="barreDepliee"
        :page="pageEnCours"
        :total="total"
        :etiquette="pageCourante?.label"
        :section="section"
        :actions="actions"
        @action="agir"
      />
      <GnOccurrence
        v-if="rangCourant !== null && passages.length && !barreDepliee"
        class="gn-lecteur__occurrence"
        :rang="rangCourant + 1"
        :total="passages.length"
        :expression="expression"
        @precedente="allerAuPrecedent"
        @suivante="allerAuSuivant"
        @fermer="rangCourant = null"
      />

      <GnPanneauLecteur
        v-model="sommaireOuvert"
        :titre="t('guide-nego.lecteur.sommaire.titre')"
        :sous-titre="t('guide-nego.lecteur.sommaire.sous-titre', { titre, pages: total })"
      >
        <GnLecteurSommaire
          :sommaire="lecture.outline"
          :pages="lecture.pages"
          :page-en-cours="pageEnCours"
          @aller="allerAuChapitre"
        />
      </GnPanneauLecteur>

      <GnPanneauLecteur v-model="rechercheOuverte" :titre="t('guide-nego.lecteur.recherche.titre')">
        <GnLecteurRecherche
          v-model:expression="expression"
          :passages="passages"
          :pages="pagesCherchees(lecture)"
          :ici="ici"
          @aller="allerAuPassage"
        />
      </GnPanneauLecteur>

      <GnReglagesLecture v-model="reglagesOuverts" />

      <GnFeuilleBasse v-model="termeOuvert" :titre="terme ?? ''" :fermeture="t('guide-nego.lecteur.terme.revenir')">
        <div class="gn-lecteur__terme">
          <p>{{ t('guide-nego.lecteur.terme.lexique') }}</p>
          <p v-if="pagesDuTerme" class="gn-lecteur__terme-pages">
            {{ t('guide-nego.lecteur.terme.pages', { pages: pagesDuTerme }) }}
          </p>
        </div>
      </GnFeuilleBasse>
    </template>
  </GnEcran>
</template>

<style>
/* Au-dessus de la barre repliée (32 + 6 px), comme la maquette 04 · 03. */
[data-app="guide-nego"] .gn-lecteur__reprise {
  position: fixed;
  bottom: calc(var(--gn-barre-lecture-repliee) + var(--gn-jauge) + var(--gn-espace-12) + env(safe-area-inset-bottom));
  left: 50%;
  transform: translateX(-50%);
  z-index: 5;
  width: calc(min(100%, var(--gn-colonne-largeur)) - 2 * var(--gn-marge-ecran));
  display: flex;
  align-items: center;
  gap: var(--gn-espace-8);
  min-height: var(--gn-cible);
  padding: 0 var(--gn-espace-8) 0 var(--gn-espace-16);
  background: var(--gn-titre);
  color: var(--gn-sur-titre);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-lecteur__reprise-texte {
  flex: 1;
  min-width: 0;
}

[data-app="guide-nego"] .gn-lecteur__debut {
  flex: none;
  min-height: var(--gn-cible);
  padding-inline: var(--gn-espace-8);
  border: none;
  background: none;
  color: var(--gn-action-sur-titre);
  font: inherit;
  font-weight: var(--gn-graisse-gras);
  text-decoration-line: var(--gn-action-sur-titre-trait);
  text-underline-offset: var(--gn-espace-4);
  cursor: pointer;
}

/* Au-dessus de la barre repliée, comme la ligne de reprise (04 · 06). */
[data-app="guide-nego"] .gn-lecteur__occurrence {
  position: fixed;
  bottom: calc(var(--gn-barre-lecture-repliee) + var(--gn-jauge) + env(safe-area-inset-bottom));
  left: 50%;
  transform: translateX(-50%);
  z-index: 5;
  width: min(100%, var(--gn-colonne-largeur));
}

[data-app="guide-nego"] .gn-lecteur__terme {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-bottom: var(--gn-espace-16);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
}

[data-app="guide-nego"] .gn-lecteur__terme-pages {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-lecteur__attente {
  margin-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-lecteur__absent {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-16);
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-lecteur__issue {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-danger);
  text-align: center;
}

[data-app="guide-nego"] .gn-lecteur__lisible,
[data-app="guide-nego"] .gn-lecteur__en-attente {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
  text-align: center;
}

[data-app="guide-nego"] .gn-lecteur__en-attente {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--gn-espace-8);
  min-height: var(--gn-cible);
}

[data-app="guide-nego"] .gn-lecteur__actions {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-espace-16);
  border-top: var(--gn-filet-1) solid var(--gn-filet);
}
</style>
