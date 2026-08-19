/**
 * BACK-OFFICE DE LA VITRINE (A15) — sa part de `useApi()`.
 *
 * Même motif qu'`api/admin-incidents.ts` : les pages appellent
 * `api.adminShowcase.list(…)`, n'importent aucun mock et n'appellent jamais
 * `$fetch`. Seule la place du code change, pour tenir `useApi.ts` sous le
 * garde-fou de mille lignes de `CLAUDE.md`.
 *
 * ── LE PÉRIMÈTRE SE REFUSE ICI, PAS DANS LA PAGE ────────────────────────────
 *
 * Règle métier n° 8, ADR-14 : une administratrice détachée sur la COP31 ne voit
 * que la COP31, « y compris quand l'utilisateur forge une URL ». Une page qui
 * porterait ce filtre le porterait UNE FOIS PAR PAGE, et `/admin/vitrine/<id>`
 * ouvert à la main serait le premier endroit où on l'oublierait. Il est donc ici,
 * sur le chemin des données, pour les trois pages à la fois.
 *
 * REFUSER, ET NON RENDRE VIDE. `list()` peut légitimement rendre `null` — la
 * personne n'administre rien —, mais une diapositive existante hors périmètre
 * lève `ForbiddenError` : une réponse vide se lirait « il n'y a rien ici » là où
 * il faut lire « ceci ne vous regarde pas ».
 *
 * ── UN CONTENU SANS ÉDITION EST UN CONTENU DE PLATEFORME ────────────────────
 *
 * `content.highlights.event_id` porte le périmètre (ADR-14), et il est
 * NULLABLE : une diapositive sans édition parle au nom de la plateforme
 * entière — elle s'affiche sur l'accueil quelle que soit la COP en cours. Elle
 * exige donc la portée globale, en lecture comme en écriture. C'est le seul
 * endroit du back-office où `null` est plus exigeant qu'une valeur, d'où le
 * garde `assertContentInScope` plutôt qu'un `assertEventInScope` direct.
 *
 * ── POURQUOI CERTAINS REFUS SE JOUENT DANS LA LECTURE SIMULÉE ───────────────
 *
 * `assertEventInScope` s'appelle AVANT la requête partout ailleurs, parce que
 * l'édition visée est dans l'URL. Ici, l'édition d'une diapositive n'est connue
 * qu'une fois la ligne lue : le garde vit donc DANS la lecture simulée, où il
 * rejoue ce que l'API répondra en 403. Sur API réelle la fonction n'est pas
 * appelée — le refus vient du serveur, et c'est la bonne place. Seules les
 * écritures dont la CIBLE est dans la charge utile (`save`) gardent le contrôle
 * avant l'appel.
 *
 * ── ENREGISTRER EST UN SEUL ACTE, MÊME S'IL A DEUX VERBES ───────────────────
 *
 * `ShowcaseSavePayload` porte `id: null` pour une création : une seule méthode
 * `save()`, qui choisit `POST` ou `PUT` d'après cet identifiant. Deux méthodes
 * pour un formulaire unique auraient dédoublé la validation — et c'est bien un
 * seul acte en base, `content.highlights` n'ayant pas de table de brouillon.
 *
 * ── ON N'EFFACE PAS UNE DIAPOSITIVE, ON L'ARCHIVE ───────────────────────────
 *
 * Aucune méthode de suppression : `content.highlight_status` vaut
 * `draft | published | archived`, et le modèle n'offre pas d'effacement. Retirer
 * de l'affichage, c'est `setStatus(…, 'archived')` — le contenu reste lisible,
 * duplicable, et l'historique de ce qui a été montré à une COP ne disparaît pas.
 *
 * ── L'ORDRE EST UNE ÉCRITURE À PART ─────────────────────────────────────────
 *
 * `move()` déplace d'un CRAN, pas d'une liste réordonnée : la spécification
 * impose des boutons monter/descendre de 44 px, utilisables au clavier, et non
 * un glisser-déposer seul. La réponse porte `placement_rows` — l'emplacement
 * entier renuméroté —, parce qu'un déplacement touche toujours deux lignes et
 * que l'écran doit les rafraîchir ensemble.
 */

import type {
  ShowcaseFormScreen,
  ShowcaseFormValues,
  ShowcaseListScreen,
  ShowcaseReorderPayload,
  ShowcaseSavePayload,
  ShowcaseSessionOption,
  ShowcaseStatusPayload,
  ShowcaseWriteResult,
} from '~/types/admin-showcase'
import type { HighlightId, HighlightPlacement } from '~/types/content'
import type { AdministeredEvents } from '~/types/identity'
import type { EventId, Uuid } from '~/types/shared'
/**
 * Import de VALEUR, et il ferme un cycle avec `useApi.ts` — qui importe cette
 * fabrique. Il est sans danger : la classe n'est référencée que dans un corps de
 * fonction, exécuté longtemps après l'évaluation des deux modules. C'est aussi
 * la seule façon de lever l'erreur que les écrans reconnaissent
 * (`instanceof ForbiddenError`, `error.name === 'ForbiddenError'`) — une erreur
 * maison ne serait pas rattrapée par `UiForbiddenState`.
 */
import { ForbiddenError } from '~/composables/useApi'
import type { ApiTransport } from './proposal-review'

interface ShowcaseApiDeps extends ApiTransport {
  assertEventInScope: (eventId: Uuid, scope: AdministeredEvents) => void
}

/** Le refus d'un contenu de plateforme, en français exploitable par l'écran. */
const PLATFORM_SCOPE_REFUSAL =
  "Ce contenu s'affiche sur toute la plateforme : sa modification demande la portée globale."

export function createAdminShowcaseApi({ call, send, assertEventInScope }: ShowcaseApiDeps) {
  /**
   * LE PÉRIMÈTRE D'UNE DIAPOSITIVE — son édition, ou la plateforme entière.
   *
   * `null` n'est pas « pas de contrainte » mais « la contrainte la plus
   * forte » : un contenu sans édition parle au nom de la plateforme et n'est
   * modifiable qu'en portée globale. Écrit une fois, appelé par les six
   * méthodes : c'est ce qui garantit que la lecture et l'écriture refusent la
   * même chose.
   */
  function assertContentInScope(eventId: EventId | null, scope: AdministeredEvents): void {
    if (eventId === null) {
      if (!scope.is_global) throw new ForbiddenError(PLATFORM_SCOPE_REFUSAL)
      return
    }
    assertEventInScope(eventId, scope)
  }

  return {
    /**
     * LA LISTE ET SES FACETTES — en une réponse.
     *
     * `null` quand la personne n'administre rien : l'écran affiche alors
     * `UiForbiddenState`, jamais un tableau vide. Les lignes arrivent triées par
     * emplacement puis `sort_order`, avec `is_first` / `is_last` déjà calculés —
     * c'est ce qui désactive les boutons d'ordre aux extrémités sans que la page
     * recompte.
     *
     * Le tri N'EST PAS refait par l'écran : `sort_order` est l'ordre de
     * défilement du bandeau public, et le voir autrement dans le back-office que
     * sur l'accueil rendrait les boutons monter/descendre incompréhensibles.
     */
    list: (scope: AdministeredEvents): Promise<ShowcaseListScreen | null> =>
      call('/admin/showcase', (m) => m.showcaseList(scope)),

    /**
     * L'ÉCRAN DE FORMULAIRE — création comme modification.
     *
     * `highlightId` nul : création, `options.placement` choisissant l'onglet
     * d'arrivée. Une administratrice détachée voit alors le formulaire s'ouvrir
     * sur SON édition : elle ne peut pas créer de contenu de plateforme.
     *
     * La réponse porte `preview`, un `ShowcaseRow` — le contrat EXACT du bandeau
     * public. C'est voulu : l'aperçu du formulaire est rendu par le même
     * composant que l'accueil, et non par une seconde mise en page qui
     * divergerait au premier changement de charte.
     *
     * DEUX ISSUES DIFFÉRENTES POUR DEUX CHOSES DIFFÉRENTES : une diapositive
     * inexistante rend `null` — l'écran dit « introuvable » — tandis qu'une
     * diapositive hors périmètre LÈVE. Les confondre ferait passer un contenu
     * interdit pour un contenu supprimé.
     */
    form: (
      highlightId: HighlightId | null,
      scope: AdministeredEvents,
      options: { placement?: HighlightPlacement } = {},
    ): Promise<ShowcaseFormScreen | null> =>
      call(
        highlightId === null ? '/admin/showcase/new' : `/admin/showcase/${highlightId}/form`,
        (m) => {
          if (highlightId !== null) {
            const found = m.showcaseById(highlightId)
            if (found !== null) assertContentInScope(found.event_id, scope)
          }
          return m.showcaseForm(highlightId, scope, options)
        },
        { placement: options.placement },
      ),

    /**
     * LES SEULES VALEURS DU FORMULAIRE, sans les listes de choix.
     *
     * Sert à recharger le fond du formulaire après une écriture, ou à relire une
     * diapositive sans repayer les référentiels (natures, éditions, pays,
     * personnes) que `form()` embarque. Même règle de refus.
     */
    byId: (highlightId: HighlightId, scope: AdministeredEvents): Promise<ShowcaseFormValues | null> =>
      call(`/admin/showcase/${highlightId}`, (m) => {
        const found = m.showcaseById(highlightId)
        if (found === null) return null
        assertContentInScope(found.event_id, scope)
        return found
      }),

    /**
     * LES SÉANCES D'UNE ÉDITION, pour la cascade « édition → séance ».
     *
     * `form()` rend les séances de l'édition CHARGÉE ; changer d'édition dans le
     * formulaire doit changer la liste sans recharger l'écran — sinon la saisie
     * en cours serait perdue. Le périmètre se vérifie avant l'appel : ici
     * l'édition est explicite, donc le refus n'a pas besoin d'attendre la
     * réponse.
     */
    sessionsFor: (eventId: EventId, scope: AdministeredEvents): Promise<ShowcaseSessionOption[]> => {
      assertEventInScope(eventId, scope)
      return call(
        '/admin/showcase/sessions',
        (m) =>
          m.allSessions
            .filter((session) => session.event_id === eventId)
            .map((session) => ({
              id: session.id,
              event_id: session.event_id,
              title: session.title,
              starts_at: session.starts_at,
              timezone: session.timezone,
            }))
            .sort((a, b) => a.starts_at.localeCompare(b.starts_at)),
        { event_id: eventId },
      )
    },

    /**
     * CRÉER OU MODIFIER — un seul acte, deux verbes.
     *
     * LE PÉRIMÈTRE SE VÉRIFIE SUR LA CIBLE, avant l'appel : on ne déplace pas une
     * diapositive vers une édition qu'on n'administre pas, et on n'en fait pas un
     * contenu de plateforme sans la portée globale. La source, elle, est vérifiée
     * par la lecture simulée — les deux bouts, comme partout.
     *
     * LES REFUS DE VALIDATION SONT DES RÉPONSES, pas des erreurs de réseau :
     * fenêtre inversée, organisation nommée ET référencée, libellé de lien sans
     * lien, français manquant. `ok: false` avec ses `errors`, que le formulaire
     * pose sur les champs concernés.
     *
     * Une création se place EN FIN de son emplacement. La placer en tête
     * déplacerait silencieusement tout le reste du bandeau.
     */
    save: (payload: ShowcaseSavePayload, scope: AdministeredEvents): Promise<ShowcaseWriteResult> => {
      assertContentInScope(payload.event_id, scope)
      return payload.id === null
        ? send('/admin/showcase', payload, (m) => m.saveShowcase(payload, scope))
        : send(`/admin/showcase/${payload.id}`, payload, (m) => m.saveShowcase(payload, scope), 'PUT')
    },

    /**
     * PUBLIER, RETIRER, ARCHIVER — depuis la liste, sans ouvrir le formulaire.
     *
     * Une route à part parce que ce sont trois actes de diffusion, pas une
     * modification de contenu : ils ne touchent ni les textes ni les médias, et
     * ils doivent rester possibles à une main depuis le tableau.
     *
     * `published_at` se pose au PREMIER passage en `published` et ne se rejoue
     * jamais — c'est le trigger de `115_content.sql` qui le dit, et le
     * back-office ne peut pas le contredire.
     */
    setStatus: (payload: ShowcaseStatusPayload, scope: AdministeredEvents): Promise<ShowcaseWriteResult> =>
      send(
        `/admin/showcase/${payload.id}/status`,
        payload,
        (m) => {
          const found = m.showcaseById(payload.id)
          if (found !== null) assertContentInScope(found.event_id, scope)
          return m.setShowcaseStatus(payload, scope)
        },
        'PUT',
      ),

    /**
     * MONTER OU DESCENDRE D'UN CRAN, dans son emplacement.
     *
     * L'ordre est la fonction principale de cet écran — son absence était le
     * défaut n° 6 de la v1. Aux extrémités, la réponse est `ok: true` sans
     * changement : les boutons y sont déjà désactivés, et un message d'erreur
     * pour une action que l'écran n'offrait pas serait du bruit.
     *
     * `placement_rows` rend l'emplacement entier renuméroté : deux lignes ont
     * bougé, et rafraîchir la seule ligne cliquée laisserait sa voisine mentir.
     */
    move: (payload: ShowcaseReorderPayload, scope: AdministeredEvents): Promise<ShowcaseWriteResult> =>
      send(
        `/admin/showcase/${payload.id}/order`,
        payload,
        (m) => {
          const found = m.showcaseById(payload.id)
          if (found !== null) assertContentInScope(found.event_id, scope)
          return m.moveShowcase(payload, scope)
        },
        'PUT',
      ),

    /**
     * DUPLIQUER — le geste qui remet un témoignage de la COP30 à la COP31.
     *
     * La copie part en BROUILLON, en fin d'emplacement : dupliquer un contenu
     * publié et le voir sortir aussitôt sur l'accueil serait une publication que
     * personne n'a demandée.
     */
    duplicate: (highlightId: HighlightId, scope: AdministeredEvents): Promise<ShowcaseWriteResult> =>
      send(`/admin/showcase/${highlightId}/duplicate`, {}, (m) => {
        const found = m.showcaseById(highlightId)
        if (found !== null) assertContentInScope(found.event_id, scope)
        return m.duplicateShowcase(highlightId, scope)
      }),
  }
}
