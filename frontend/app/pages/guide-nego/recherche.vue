<script setup lang="ts">
import type { DocumentReading, LibraryDocument } from '~/types/negotiation-documents'
import type { FaqEntry, GlossaryEntry } from '~/types/negotiation-savoir'
import { dayKeyInZone } from '~/utils/datetime'
import { morceauxSurlignes } from '~/utils/guide-nego/lexique'
import { chercherDansLeDocument, type Passage } from '~/utils/guide-nego/lecteur'
import { cheminDeLActivite, etatDeLActivite } from '~/utils/guide-nego/pavillon'
import {
  activitesTrouvees,
  documentsTrouves,
  rechercheLancee,
  reunionsTrouvees,
  sessionsTrouvees,
} from '~/utils/guide-nego/recherche-globale'
import { etatAffiche, jourAOuvrir, joursDeLaBande } from '~/utils/guide-nego/sessions'

/**
 * La recherche globale — maquette 02-socle, écran « 09 Recherche ». Lexique, FAQ,
 * documents, les sessions de négociation du jour que la liste ouvrirait, puis les
 * réunions de la Francophonie et les activités du Pavillon gardées — les trois agendas,
 * chacun son groupe.
 *
 * Tout se cherche sur le téléphone, sauf le texte des documents : l'API le cherche dans
 * tous ceux qu'on peut lire ; sans elle, seules les copies gardées se lisent, et l'écran
 * le dit. La saisie vit dans l'adresse : revenir d'un résultat retrouve la recherche.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const MONTRES = 5
const RACINES = ['/guide-nego', '/guide-nego/', '/guide-nego/ressources']

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const { dayLong } = useDateTime()
const connexion = useGnConnexion()
const compte = useGnSession()
const savoir = useGnSavoir()
const docs = useGnDocuments()
const copies = useGnCopies()
const sessions = useGnSessions()
const reunions = useGnReunions()
const inscriptions = useGnInscriptionsReunions()
const edition = useGnEdition()
const pavillon = useGnPavillon()
const inscriptionsPavillon = useGnInscriptionsPavillon()
const thematiques = useGnThematiques()

const origine = useState<string>('gn-recherche-origine', () => '/guide-nego')

const lirePremier = (valeur: unknown): string => {
  const premier = Array.isArray(valeur) ? valeur[0] : valeur
  return typeof premier === 'string' ? premier : ''
}

const saisie = ref(lirePremier(route.query.q))
const cherche = computed(() => saisie.value.trim())
const lancee = computed(() => rechercheLancee(cherche.value))
const zone = useTemplateRef<HTMLElement>('zone')

watch(saisie, (q) => {
  if (q !== lirePremier(route.query.q)) void router.replace({ query: q.trim() ? { q } : {} })
})

onMounted(async () => {
  const avant: unknown = window.history.state?.back
  if (typeof avant === 'string' && RACINES.includes(avant)) origine.value = avant
  zone.value?.querySelector('input')?.focus()
  void docs.rafraichir()
  void sessions.rafraichir()
  void reunions.rafraichir()
  void pavillon.rafraichir()
  void copies.recharger()
  await compte.assurer()
  inscriptions.assurer()
  inscriptionsPavillon.assurer()
  void (compte.connectee.value ? thematiques.assurer() : thematiques.assurerLeVocabulaire())
  await savoir.assurer()
})

// --- Le texte des documents : l'API en ligne, les copies gardées sinon ------------

const enLigne = computed(() => connexion.etat.value.enLigne)
const attenteServeur = ref(false)
let minuterie: ReturnType<typeof setTimeout> | undefined

watch(
  [cherche, enLigne],
  ([q]) => {
    clearTimeout(minuterie)
    attenteServeur.value = enLigne.value && rechercheLancee(q)
    minuterie = setTimeout(async () => {
      await docs.chercherDansLeTexte(q)
      if (q === cherche.value) attenteServeur.value = false
    }, 250)
  },
  { immediate: true },
)
onBeforeUnmount(() => clearTimeout(minuterie))

const surLeServeur = computed(() => docs.pagesTrouvees.value !== null)
const telephoneSeul = computed(() => lancee.value && !attenteServeur.value && !surLeServeur.value)

// Lues une fois par version : chaque frappe ne cherche plus que dans leur index.
const formes = shallowRef(new Map<string, { version: string; lecture: DocumentReading }>())

watch(
  [telephoneSeul, () => copies.copies.value],
  async ([seul, gardees]) => {
    if (!seul) return
    const suivantes = new Map<string, { version: string; lecture: DocumentReading }>()
    for (const copie of gardees) {
      const connue = formes.value.get(copie.id)
      if (connue?.version === copie.version) {
        suivantes.set(copie.id, connue)
        continue
      }
      const lecture = await copies.lireLaFormeGardee(copie.id)
      if (lecture) suivantes.set(copie.id, { version: copie.version, lecture })
    }
    formes.value = suivantes
  },
  { immediate: true },
)

const passagesGardes = computed(() => {
  const passages = new Map<string, Passage[]>()
  if (!telephoneSeul.value) return passages
  for (const [id, { lecture }] of formes.value) passages.set(id, chercherDansLeDocument(lecture, cherche.value))
  return passages
})

// --- Les groupes ---------------------------------------------------------------

const lexique = computed(() => (lancee.value ? savoir.chercherDansLeLexique(cherche.value).trouves.map((r) => r.valeur) : []))
const faq = computed(() => (lancee.value ? savoir.chercherDansLaFaq(cherche.value).trouves.map((r) => r.valeur) : []))
const documents = computed(() =>
  documentsTrouves(docs.documents.value, cherche.value, docs.pagesTrouvees.value, passagesGardes.value),
)

const servies = computed(() => (sessions.affichage.value.etat === 'sert' ? sessions.affichage.value : null))
const fuseau = computed(() => servies.value?.fuseau ?? sessions.fuseau.value ?? 'UTC')
const maintenant = new Date()
const jour = computed(() =>
  servies.value ? jourAOuvrir(joursDeLaBande(servies.value.sessions, fuseau.value), maintenant, fuseau.value) : null,
)
const sessionsDuJour = computed(() =>
  servies.value ? sessionsTrouvees(servies.value.sessions, cherche.value, jour.value, fuseau.value) : [],
)

const reunionsDeLaRecherche = computed(() =>
  reunionsTrouvees(reunions.reunions.value, cherche.value).map((reunion) => ({
    reunion,
    etat: inscriptions.etat(reunion.id, maintenant) ?? 'prevue',
  })),
)
const fuseauDesReunions = computed(() => reunions.fuseau.value ?? 'UTC')

const activitesDeLaRecherche = computed(() =>
  activitesTrouvees(pavillon.activites.value, cherche.value).map((activite) => ({
    activite,
    etat: etatDeLActivite(activite, maintenant),
    marque: inscriptionsPavillon.marque(activite.id, maintenant),
  })),
)
const fuseauDuPavillon = computed(() => pavillon.fuseau.value ?? 'UTC')

const lignesDeDocuments = computed(() =>
  documents.value.flatMap(({ document, passages }) =>
    passages.length
      ? passages.map((p) => ({
          cle: `${document.id}-${p.page}-${p.extrait}`,
          document,
          vers: `/guide-nego/ressources/documents/${document.id}/lire?page=${p.page}`,
          texte: morceauxSurlignes(t('guide-nego.recherche.documents.page', { page: p.etiquette, extrait: p.extrait }), cherche.value),
        }))
      : [{
          cle: document.id,
          document,
          vers: `/guide-nego/ressources/documents/${document.id}`,
          texte: morceauxSurlignes(texteDeFiche(document), cherche.value),
        }],
  ),
)

function texteDeFiche(document: LibraryDocument): string {
  if (document.restricted && !document.accessible && !(document.summary ?? '').trim()) return t('guide-nego.recherche.documents.reserve')
  return [document.publisher, document.summary].filter(Boolean).join(' · ')
}

const documentsMontres = computed(() => {
  const gardes = new Set(documents.value.slice(0, MONTRES).map((d) => d.document.id))
  return lignesDeDocuments.value.filter((l) => gardes.has(l.document.id))
})

const compteDesDocuments = computed(() => {
  const passages = documents.value.reduce((n, d) => n + d.passages.length, 0)
  const n = documents.value.length
  const textePassages = t('guide-nego.recherche.documents.passages', { count: passages }, passages)
  const texteDocuments = t('guide-nego.recherche.documents.documents', { count: n }, n)
  if (passages === 0) return texteDocuments
  if (documents.value.every((d) => d.passages.length)) return textePassages
  return t('guide-nego.recherche.documents.les-deux', { documents: texteDocuments, passages: textePassages })
})

const compteDesSessions = computed(() => {
  const n = sessionsDuJour.value.length
  if (jour.value === dayKeyInZone(maintenant, fuseau.value)) return t('guide-nego.recherche.sessions.aujourdhui', { count: n }, n)
  return t('guide-nego.recherche.sessions.le-jour', { count: n, jour: dayLong(`${jour.value}T12:00:00Z`, 'UTC') }, n)
})

const proposition = ref(false)

const total = computed(
  () =>
    lexique.value.length +
    faq.value.length +
    lignesDeDocuments.value.length +
    sessionsDuJour.value.length +
    reunionsDeLaRecherche.value.length +
    activitesDeLaRecherche.value.length,
)

const chargement = computed(() => lancee.value && !savoir.etat.value.pret)
const savoirAbsent = computed(() => savoir.etat.value.pret && !savoir.etat.value.valeur)

const ligneDuTotal = computed(() =>
  t(`guide-nego.recherche.total.${telephoneSeul.value ? 'telephone' : 'serveur'}`, { count: total.value }, total.value),
)

const suite = (cle: 'lexique' | 'faq' | 'documents', n: number, vers: string) =>
  n > MONTRES ? { libelle: t(`guide-nego.recherche.${cle}.suite`, { count: n }, n), vers: `${vers}?q=${encodeURIComponent(cherche.value)}` } : null

const versTerme = (e: GlossaryEntry) => `/guide-nego/lexique/${e.slug}`
const versQuestion = (e: FaqEntry) => `/guide-nego/ressources/faq/${e.id}`

useHead({ title: t('guide-nego.recherche.titre') })
</script>

<template>
  <GnEcran :titre="t('guide-nego.recherche.titre')" :retour="origine">
    <div ref="zone" class="gn-recherche">
      <GnChampRecherche
        v-model="saisie"
        :libelle="t('guide-nego.recherche.champ')"
        :indication="t('guide-nego.recherche.indication')"
        :chargement="attenteServeur"
      />

      <p v-if="!lancee" class="gn-recherche__ligne gn-recherche__ligne--filet">{{ t('guide-nego.recherche.invite') }}</p>

      <GnChargement v-else-if="chargement" forme="squelette" :lignes="3" :libelle="t('guide-nego.recherche.chargement')" />

      <template v-else>
        <p class="gn-recherche__ligne gn-recherche__ligne--filet" role="status">{{ ligneDuTotal }}</p>
        <p v-if="savoirAbsent" class="gn-recherche__ligne">{{ t('guide-nego.recherche.savoir-absent') }}</p>

        <GnGroupeResultats
          v-if="lexique.length"
          :titre="t('guide-nego.recherche.lexique.titre')"
          :compte="t('guide-nego.recherche.lexique.compte', { count: lexique.length }, lexique.length)"
          :suite="suite('lexique', lexique.length, '/guide-nego/lexique')"
        >
          <li v-for="e in lexique.slice(0, MONTRES)" :key="e.id">
            <GnLigneTerme :entree="e" :vers="versTerme(e)" extrait :surligne="cherche" />
          </li>
        </GnGroupeResultats>

        <GnGroupeResultats
          v-if="faq.length"
          :titre="t('guide-nego.recherche.faq.titre')"
          :compte="t('guide-nego.recherche.faq.compte', { count: faq.length }, faq.length)"
          :suite="suite('faq', faq.length, '/guide-nego/ressources/faq')"
        >
          <li v-for="e in faq.slice(0, MONTRES)" :key="e.id">
            <GnLigneQuestion :entree="e" :vers="versQuestion(e)" :surligne="cherche" />
          </li>
        </GnGroupeResultats>

        <GnGroupeResultats
          v-if="documents.length"
          :titre="t('guide-nego.recherche.documents.titre')"
          :compte="compteDesDocuments"
          :suite="suite('documents', documents.length, '/guide-nego/ressources/documents')"
        >
          <li v-for="l in documentsMontres" :key="l.cle">
            <NuxtLink :to="l.vers" class="gn-recherche__document">
              <GnPicto :nom="l.document.source === 'link' ? 'external' : 'doc'" :taille="20" class="gn-recherche__picto" />
              <span class="gn-recherche__corps">
                <span class="gn-recherche__titre">{{ l.document.title }}</span>
                <span v-if="l.texte.length" class="gn-recherche__extrait">
                  <template v-for="(m, i) in l.texte" :key="i"><mark v-if="m.marque">{{ m.texte }}</mark><template v-else>{{ m.texte }}</template></template>
                </span>
              </span>
              <GnPicto nom="chevron" :taille="20" class="gn-recherche__chevron" />
            </NuxtLink>
          </li>
        </GnGroupeResultats>

        <GnGroupeResultats
          v-if="sessionsDuJour.length"
          :titre="t('guide-nego.recherche.sessions.titre')"
          :compte="compteDesSessions"
        >
          <li v-for="s in sessionsDuJour" :key="s.id">
            <GnLigneSession
              :session="s"
              :etat="etatAffiche(s, maintenant, fuseau)"
              :fuseau="fuseau"
              :ville="servies?.ville ?? null"
              :thematique="s.theme ? thematiques.nomDe(s.theme) : null"
              :vers="`/guide-nego/negociations/${s.id}`"
              :surligne="cherche"
            />
          </li>
        </GnGroupeResultats>

        <GnGroupeResultats
          v-if="reunionsDeLaRecherche.length"
          :titre="t('guide-nego.recherche.reunions.titre')"
          :compte="t('guide-nego.recherche.reunions.compte', { count: reunionsDeLaRecherche.length }, reunionsDeLaRecherche.length)"
        >
          <li v-for="l in reunionsDeLaRecherche" :key="l.reunion.id">
            <GnLigneReunion
              :reunion="l.reunion"
              :etat="l.etat"
              :fuseau="fuseauDesReunions"
              :ville="reunions.ville.value"
              :vers="`/guide-nego/francophonie/reunions/${l.reunion.id}`"
            />
          </li>
        </GnGroupeResultats>

        <GnGroupeResultats
          v-if="activitesDeLaRecherche.length"
          :titre="t('guide-nego.recherche.pavillon.titre')"
          :compte="t('guide-nego.recherche.pavillon.compte', { count: activitesDeLaRecherche.length }, activitesDeLaRecherche.length)"
        >
          <li v-for="l in activitesDeLaRecherche" :key="l.activite.id">
            <GnLigneActivite
              v-bind="l"
              jour
              :fuseau="fuseauDuPavillon"
              :ville="edition.edition.value?.city ?? null"
              :vers="cheminDeLActivite(l.activite.slug)"
            />
          </li>
        </GnGroupeResultats>

        <div v-if="total === 0" class="gn-recherche__aucun">
          <h2 class="gn-recherche__aucun-titre">{{ t('guide-nego.recherche.aucun.titre', { terme: cherche }) }}</h2>
          <p class="gn-recherche__aucun-texte">{{ t('guide-nego.recherche.aucun.texte') }}</p>
          <GnBouton picto="plus" @clic="proposition = true">
            {{ t('guide-nego.recherche.aucun.proposer', { terme: cherche }) }}
          </GnBouton>
        </div>

        <p v-if="telephoneSeul" class="gn-recherche__ligne">
          <GnPicto nom="wifi-off" :taille="16" />
          {{ t('guide-nego.recherche.non-telecharges') }}
        </p>
      </template>
    </div>

    <GnFeuilleProposerTerme v-model="proposition" :terme="cherche" />
  </GnEcran>
</template>

<style>
/* 20 entre les blocs, comme au lexique ; le compte se colle au champ. */
[data-app="guide-nego"] .gn-recherche {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-20);
}

[data-app="guide-nego"] .gn-recherche__ligne {
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-8);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
}

[data-app="guide-nego"] .gn-recherche__ligne .gn-picto {
  flex: none;
}

[data-app="guide-nego"] .gn-recherche__ligne--filet {
  margin-top: calc(-1 * var(--gn-espace-8));
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
}

[data-app="guide-nego"] .gn-recherche .gn-groupe-resultats {
  display: flex;
  flex-direction: column;
}

/* Un groupe de résultats est un bloc de la page, pas une section : titre Sora 16 en retrait. */
[data-app="guide-nego"] .gn-recherche .gn-groupe {
  margin-bottom: var(--gn-espace-4);
  padding: 0 0 var(--gn-espace-4);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-16);
  line-height: var(--gn-interligne-16);
}

[data-app="guide-nego"] .gn-recherche .gn-groupe-resultats__liste {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

/* Chaque résultat devient une ligne-carte : la ligne d'agenda garde son dessin, sans filet ni débord au toucher. */
[data-app="guide-nego"] .gn-recherche .gn-groupe-resultats__liste > li {
  padding-inline: var(--gn-espace-16);
  border-radius: var(--gn-rayon-16);
  background: var(--gn-fond-2);
}

[data-app="guide-nego"] .gn-recherche .gn-groupe-resultats__liste > li:has(> .gn-ligne-terme) {
  padding-inline: 0;
  background: none;
}

[data-app="guide-nego"] .gn-recherche .gn-groupe-resultats__liste > li + li > .gn-ligne-terme {
  margin-top: 0;
}

[data-app="guide-nego"] .gn-recherche .gn-groupe-resultats__liste > li > * {
  border-bottom: none;
}

[data-app="guide-nego"] .gn-recherche .gn-groupe-resultats__liste > li:not(:has(> .gn-ligne-terme)) > *:is(:active, :has(:active)) {
  width: auto;
  margin-inline: 0;
  padding-inline: 0;
  background: none;
}

[data-app="guide-nego"] .gn-recherche .gn-groupe-resultats__liste > li:not(:has(> .gn-ligne-terme)):has(:active) {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-recherche .gn-groupe-resultats__suite {
  margin-top: 6px;
  border-top: none;
}

[data-app="guide-nego"] .gn-recherche__document {
  min-height: var(--gn-ligne-reglage);
  padding-block: var(--gn-espace-12);
  display: flex;
  align-items: flex-start;
  gap: 14px;
  color: var(--gn-texte);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-recherche__picto {
  flex: none;
  color: var(--gn-accent);
}

[data-app="guide-nego"] .gn-recherche__corps {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

[data-app="guide-nego"] .gn-recherche__titre {
  font-size: var(--gn-taille-16);
  line-height: var(--gn-interligne-16);
  font-weight: var(--gn-graisse-gras);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-recherche__extrait {
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
  color: var(--gn-texte-2);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-recherche__chevron {
  flex: none;
  align-self: center;
  color: var(--gn-picto-secondaire);
}

[data-app="guide-nego"] .gn-recherche__aucun {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--gn-espace-12);
  padding: 18px var(--gn-espace-16);
  border-radius: var(--gn-rayon-24);
  background: var(--gn-fond-2);
  text-align: center;
}

[data-app="guide-nego"] .gn-recherche__aucun-titre {
  color: var(--gn-titre);
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-18);
  line-height: var(--gn-interligne-18);
  font-weight: var(--gn-graisse-gras);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-recherche__aucun-texte {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
}

[data-app="guide-nego"] .gn-recherche__aucun .gn-bouton {
  align-self: stretch;
}
</style>
