/**
 * ACCUEIL PUBLIC (A15) — sa part de `useApi()`.
 *
 * Même motif qu'`api/organization-workspace.ts` : la page appelle
 * `api.home.screen()`, n'importe aucun mock et n'appelle jamais `$fetch`. Seule
 * la place du code change, pour tenir `useApi.ts` sous le garde-fou de mille
 * lignes de `CLAUDE.md`.
 *
 * ── UNE LECTURE, AUCUNE ÉCRITURE, ET PAS ENCORE D'API ───────────────────────
 *
 * La fabrique ne reçoit que `call`. L'accueil est public et ne modifie rien ;
 * ce qui compose la vitrine, lui, écrit et exige `content.highlight.manage` — il
 * vit dans `api/admin-showcase.ts`. La séparation n'est pas cosmétique : aucune
 * route servie à un visiteur anonyme ne doit voisiner avec une route
 * d'administration, ne serait-ce que dans ce fichier.
 *
 * **`GET /home` EST SERVIE DEPUIS LE 24/08** : le crate `content` existe, et
 * `pending` est redevenu `call` — le chemin et la forme étaient déjà écrits.
 * L'accueil ne lit plus aucune donnée d'exemple lorsque l'API est configurée.
 *
 * ── UNE REQUÊTE, PAS QUATRE ─────────────────────────────────────────────────
 *
 * `screen()` rend le bandeau, les prochaines séances, l'appel de l'édition en
 * cours et l'historique des éditions — en une fois.
 * C'est le principe tenu par tous les écrans composés du projet (`review.desk`,
 * `admin.dashboard`, `proposals.list`) : quatre vues du modèle assemblées par le
 * client, ce sont quatre états de chargement à composer et surtout quatre
 * instants de mesure différents dans une même page. Un bandeau qui annonce un
 * appel encore ouvert pendant que la section d'appel affiche « clos » n'est pas
 * un défaut d'affichage : c'est une réponse assemblée en quatre fois.
 *
 * ── L'ÉDITION EN COURS SE CHOISIT PAR LES DONNÉES ───────────────────────────
 *
 * `HomeScreen.currentEdition` arrive DÉJÀ choisie : la première édition à
 * pavillon non terminée par `starts_at`, à défaut la plus récente. C'était la
 * logique de l'ancien `pages/index.vue`, qui redirigeait vers elle ; la
 * redirection est révoquée, le choix survit. Aucune page ne le refait, et
 * surtout aucune constante ne le remplace — une COP écrite en dur, c'est la
 * page d'accueil qui pointe encore sur la précédente six mois après.
 *
 * ── AUCUN PARAMÈTRE D'INSTANT DANS LE CONTRAT ───────────────────────────────
 *
 * Les fonctions simulées acceptent un `at` pour rejouer la page à une date
 * choisie ; le contrat ne l'expose pas. « À venir », « en cours » et la fenêtre
 * de diffusion d'une épingle se décident avec l'horloge qui fait autorité,
 * celle du serveur, et la réponse porte son instant de composition dans
 * `generated_at`. Un client qui choisirait son propre « maintenant » ferait
 * réapparaître une annonce périmée en changeant l'heure de sa machine.
 *
 * ── CE QUI N'EST PAS ICI, ET POURQUOI ───────────────────────────────────────
 *
 * Ni `currentEdition()`, ni `publicEditions()`, ni `editionStats()` : les trois
 * sont DANS `screen()`. Les exposer séparément inviterait la page à les
 * redemander, c'est-à-dire à payer trois allers-retours pour des données déjà
 * en main. `api.events.publicList()` garde sa place — il sert le sélecteur
 * d'année de la page d'une édition (A3), qui est un autre écran.
 *
 * Ni l'historique groupé par millésime : `screen()` porte la liste entière et
 * non filtrée, et `utils/edition-history.ts` en compose les groupes, les
 * décomptes et les volumes dans le navigateur — changer d'onglet ne coûte donc
 * aucune requête. Une adresse `/home/editions` que personne n'appelle serait une
 * dette de plus au contrat ; elle se réintroduira le jour où il y aura cinquante
 * éditions et non cinq, avec un appelant.
 */

import type { HomeScreen } from '~/types/home'
import type {
  EditionStatsRow,
  PublicEditionRow,
  PublicScheduleRow,
  ShowcaseRow,
} from '~/types/views'
import type { EventId } from '~/types/shared'
import type { ApiTransport } from './proposal-review'

/** Ce que l'accueil montre des prochaines séances. Le reste défilerait sous le pli. */
const SEANCES_ANNONCEES = 6

/**
 * L'accueil est composé de TROIS APPELS, tous servis par la plateforme.
 *
 * Il devait n'en faire qu'un — une composition en base, un seul instant. Les
 * trois lectures appartiennent à trois modules, et la composition en base
 * exigerait qu'un module lise les tables d'un autre : c'est précisément ce que
 * la constitution interdit. Le prix est connu et borné — trois instants de
 * mesure séparés de quelques millisecondes, sur des données qui changent à la
 * journée.
 *
 * LES CHIFFRES DU PROGRAMME NE COÛTENT AUCUN APPEL : chaque ligne d'édition les
 * porte déjà (`published_session_count` et ses voisins). Les redemander serait
 * un aller-retour pour une donnée qu'on tient en main.
 */
export function createHomeApi({ call }: Pick<ApiTransport, 'call'>) {
  return {
    screen: async (): Promise<HomeScreen> => {
      const [editions, upcomingSessions, showcase] = await Promise.all([
        call<PublicEditionRow[]>('/events/public', (m) => m.publicEditions()),
        call<PublicScheduleRow[]>('/schedule', (m) => m.upcomingSessions()),
        call<Pick<HomeScreen, 'hero'>>('/home', (m) => ({
          hero: m.currentShowcase().filter((row: ShowcaseRow) => row.placement === 'home_hero'),
        })),
      ])

      // Décroissant : l'accueil ouvre sur ce qui vient, pas sur 2024.
      const parDateDecroissante = [...editions].sort((a, b) =>
        b.starts_at.localeCompare(a.starts_at),
      )

      return {
        ...showcase,
        // Les six premières dans l'ordre du temps. L'API les rend déjà triées
        // et bornées aux séances à venir : l'écran ne retrie ni ne refiltre.
        upcomingSessions: upcomingSessions.slice(0, SEANCES_ANNONCEES),
        editions: parDateDecroissante,
        stats: chiffresParEdition(editions),
        currentEdition: editionDuPavillon(parDateDecroissante),
        generated_at: new Date().toISOString(),
      }
    },
  }
}

/**
 * Les chiffres du programme, indexés par édition.
 *
 * INDEXÉS, et non listés : une édition sans programme publié n'a AUCUNE CLÉ, et
 * l'absence vaut zéro. Une liste obligerait à chercher, et « pas trouvé » se
 * serait vite transformé en tiret à l'écran.
 */
function chiffresParEdition(editions: PublicEditionRow[]): Record<EventId, EditionStatsRow> {
  const parEdition: Record<EventId, EditionStatsRow> = {}
  for (const edition of editions) {
    if (edition.published_session_count === 0) continue
    parEdition[edition.id] = {
      event_id: edition.id,
      published_session_count: edition.published_session_count,
      streamed_session_count: edition.streamed_session_count,
      organization_count: edition.organization_count,
      programme_starts_at: edition.programme_starts_at,
      programme_ends_at: edition.programme_ends_at,
    }
  }
  return parEdition
}

/**
 * L'édition dont l'accueil présente l'appel.
 *
 * La première qui tient un pavillon et n'est pas terminée ; à défaut, la
 * dernière qui en a tenu un. `null` quand aucune n'en tient — l'ancre de la
 * section reste, mais elle s'efface (règle métier n° 5 : sans pavillon, pas
 * d'appel à propositions).
 */
function editionDuPavillon(parDateDecroissante: PublicEditionRow[]): PublicEditionRow | null {
  const avecPavillon = parDateDecroissante.filter((edition) => edition.has_pavilion)
  if (avecPavillon.length === 0) return null
  return (
    avecPavillon.find((edition) => edition.temporal_state !== 'past') ??
    avecPavillon[0] ??
    null
  )
}
