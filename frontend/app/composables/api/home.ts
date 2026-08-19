/**
 * ACCUEIL PUBLIC (A15) — sa part de `useApi()`.
 *
 * Même motif qu'`api/organization-workspace.ts` : la page appelle
 * `api.home.screen()`, n'importe aucun mock et n'appelle jamais `$fetch`. Seule
 * la place du code change, pour tenir `useApi.ts` sous le garde-fou de mille
 * lignes de `CLAUDE.md`.
 *
 * ── DEUX LECTURES, AUCUNE ÉCRITURE ──────────────────────────────────────────
 *
 * La fabrique ne reçoit que `call`. L'accueil est public et ne modifie rien ; ce
 * qui compose la vitrine, lui, écrit et exige `content.highlight.manage` — il
 * vit dans `api/admin-showcase.ts`. La séparation n'est pas cosmétique : aucune
 * route servie à un visiteur anonyme ne doit voisiner avec une route
 * d'administration, ne serait-ce que dans ce fichier.
 *
 * ── UNE REQUÊTE, PAS QUATRE ─────────────────────────────────────────────────
 *
 * `screen()` rend le bandeau, le panneau « À venir », les prochaines séances,
 * l'appel de l'édition en cours et l'historique des éditions — en une fois.
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
 */

import type { EditionHistory, EditionPeriod, HomeScreen } from '~/types/home'
import type { ApiTransport } from './proposal-review'

/** L'accueil ne fait que lire : `send` ne lui est pas passé. */
type HomeApiDeps = Pick<ApiTransport, 'call'>

export function createHomeApi({ call }: HomeApiDeps) {
  return {
    /**
     * TOUTE LA PAGE D'ACCUEIL EN UNE RÉPONSE.
     *
     * `hero` et `aside` sortent de `content.v_showcase`, qui a DÉJÀ appliqué le
     * statut et la fenêtre de diffusion ; `editions` sort de
     * `event.v_public_editions`, qui a déjà écarté brouillons et annulations ;
     * `stats` sort de `programme.v_edition_stats`. La page ne rejoue aucun de
     * ces filtres — un écran qui les rejoue finit par en oublier un, et c'est
     * exactement ainsi que les annonces périmées survivaient en v1.
     *
     * `stats` est INDEXÉE par `event_id` et une édition sans programme publié
     * n'y a AUCUNE CLÉ : l'absence vaut zéro. La forme est choisie pour que la
     * règle soit difficile à oublier.
     *
     * En phase B, un seul `GET /home`.
     */
    screen: (): Promise<HomeScreen> => call('/home', (m) => m.homeScreen()),

    /**
     * L'HISTORIQUE GROUPÉ PAR MILLÉSIME — le segment `?periode=` de l'URL.
     *
     * CE QUI EST ICI EST LE GROUPEMENT, PAS LE FILTRE. `screen()` porte déjà la
     * liste entière et non filtrée : changer d'onglet ne coûte donc aucune
     * requête, et les onglets peuvent annoncer leurs décomptes — calculés sur
     * l'ensemble, pour que « Passés (0) » se voie avant d'y aller. Ce que cette
     * méthode ajoute, c'est la COMPOSITION que la section attend : années
     * décroissantes, éditions triées à l'intérieur, décomptes, volumes de
     * programme. Écrite dans la page, elle y serait réécrite une seconde fois
     * le jour où l'historique aura sa propre adresse.
     *
     * Elle sert donc l'arrivée directe sur `/?periode=passes` rendue côté
     * serveur, et elle deviendra la pagination le jour où il y aura cinquante
     * éditions et non cinq.
     */
    editions: (period: EditionPeriod = 'all'): Promise<EditionHistory> =>
      call('/home/editions', (m) => m.editionHistory(period), { period }),
  }
}
