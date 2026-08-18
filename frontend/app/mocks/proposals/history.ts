/**
 * Données simulées de `programme.proposal_history()` — l'onglet « Historique »
 * de l'espace organisation (A5).
 *
 * CE N'EST PAS UNE TABLE, et c'est tout l'intérêt. L'historique est un
 * SOUS-PRODUIT du journal d'audit : `platform.tg_audit()` écrit dans
 * `platform.audit_log` à chaque écriture, et `platform.entity_history()` dépile
 * `changed_fields` en une ligne par champ modifié. Toute table portant le
 * trigger obtient donc son historique sans rien coder — y compris pour les
 * corrections faites en console. La v1 maintenait une table
 * `activity_modifications` alimentée à la main par le code applicatif : elle ne
 * couvrait que les activités, et seulement les écritures qui passaient par le
 * bon chemin. « L'ancienne plateforme ne le permettait pas », dit le prompt.
 *
 * SEPT CHAMPS SONT EXCLUS EN BASE et ne doivent donc jamais apparaître ici :
 * `updated_at`, `search_vector`, `view_count`, `average_score`,
 * `weighted_score`, `review_count`, `is_knocked_out`. Les quatre derniers sont
 * recalculés par `refresh_proposal_score()` à chaque note saisie : les laisser
 * passer noierait les modifications réelles du dossier sous le bruit du comité,
 * et montrerait au soumissionnaire des notes qu'il n'a pas à voir.
 *
 * `actor_label` EST DÉNORMALISÉ DANS L'AUDIT, volontairement : il reste lisible
 * après anonymisation RGPD, quand `actor_id` ne pointe plus vers personne.
 *
 * LES VALEURS SONT DES DOCUMENTS JSON, pas des chaînes — `old_data -> field`.
 * Un titre est un `i18n_text`, une durée un nombre, un statut une chaîne :
 * l'écran les rend selon leur forme, il ne les concatène pas.
 */

import type { ProposalHistoryEntry } from '~/types/programme/proposal'
import { PERSON, PROPOSAL } from '../ids'

/** Raccourci d'écriture : une modification de champ, datée et signée. */
function change(
  occurred_at: string,
  actor_id: string | null,
  actor_label: string | null,
  field: string,
  old_value: unknown,
  new_value: unknown,
): ProposalHistoryEntry {
  return { occurred_at, actor_id, actor_label, action: 'update', field, old_value, new_value }
}

/** La ligne d'ouverture : une création n'a pas de champ modifié. */
function creation(occurred_at: string, actor_id: string, actor_label: string): ProposalHistoryEntry {
  return {
    occurred_at,
    actor_id,
    actor_label,
    action: 'insert',
    field: null,
    old_value: null,
    new_value: null,
  }
}

/**
 * L'historique, par dossier.
 *
 * Il ne couvre PAS les quarante et un dossiers : trois suffisent à éprouver
 * l'écran, et quarante historiques écrits à la main seraient illisibles sans
 * lecteur. Ceux qui sont couverts sont ceux dont un écran du jalon montre le
 * parcours — c'est le même choix que pour les revues détaillées, documenté en
 * tête de `mocks/reviews.ts`.
 */
export const proposalHistories: Record<string, ProposalHistoryEntry[]> = {
  // Dossier retenu, corrigé une fois en cours d'évaluation : le cas complet.
  [PROPOSAL.adaptationCotiere]: [
    // Le plus récent en tête, comme le rend `entity_history()` (ORDER BY DESC).
    change(
      '2026-07-28T14:00:00Z',
      PERSON.bakayoko,
      'Aminata Bakayoko',
      'status',
      'under_review',
      'accepted',
    ),
    change(
      '2026-07-11T16:25:00Z',
      PERSON.sowFall,
      'Awa Sow Fall',
      'status',
      'changes_requested',
      'under_review',
    ),
    change(
      '2026-07-11T16:20:00Z',
      PERSON.sowFall,
      'Awa Sow Fall',
      'detailed_presentation',
      {
        fr: "L'érosion côtière ouest-africaine progresse de plusieurs mètres par an sur des secteurs habités. Les réponses techniques sont connues ; leur financement ne l'est pas.",
      },
      {
        fr: "L'érosion côtière ouest-africaine progresse de plusieurs mètres par an sur des secteurs habités. Les réponses techniques sont connues ; leur financement ne l'est pas. Les guichets internationaux instruisent des projets nationaux, alors que la dynamique sédimentaire est régionale : un ouvrage construit dans un pays déplace l'érosion chez le voisin.",
      },
    ),
    change(
      '2026-07-08T11:00:00Z',
      PERSON.nkoDiop,
      'Fatou Nko Diop',
      'status',
      'under_review',
      'changes_requested',
    ),
    change('2026-06-24T08:00:00Z', PERSON.bakayoko, 'Aminata Bakayoko', 'status', 'submitted', 'under_review'),
    change('2026-06-10T09:00:00Z', PERSON.sowFall, 'Awa Sow Fall', 'status', 'draft', 'submitted'),
    // Le créneau souhaité a bougé pendant la rédaction : c'est exactement ce que
    // la v1 ne savait pas restituer, ses dates proposées et retenues vivant dans
    // la même ligne, sans historique des arbitrages.
    change(
      '2026-06-08T15:42:00Z',
      PERSON.sowFall,
      'Awa Sow Fall',
      'preferred_start_at',
      '2027-11-09T14:00:00-03:00',
      '2027-11-10T14:00:00-03:00',
    ),
    change('2026-06-08T15:42:00Z', PERSON.sowFall, 'Awa Sow Fall', 'duration_minutes', 60, 90),
    change(
      '2026-06-05T09:15:00Z',
      PERSON.mbayeNdiaye,
      'Ousmane Mbaye Ndiaye',
      'title',
      { fr: "Adaptation côtière en Afrique de l'Ouest" },
      {
        fr: "Financer l'adaptation côtière en Afrique de l'Ouest",
        en: 'Financing coastal adaptation in West Africa',
      },
    ),
    creation('2026-06-02T10:30:00Z', PERSON.sowFall, 'Awa Sow Fall'),
  ],

  // Dossier en corrections demandées : l'historique s'arrête au renvoi, et c'est
  // ce que l'organisation vient vérifier — « qu'ai-je changé depuis ? ».
  [PROPOSAL.numeriqueResponsable]: [
    change(
      '2026-08-10T08:30:00Z',
      PERSON.bakayoko,
      'Aminata Bakayoko',
      'status',
      'submitted',
      'changes_requested',
    ),
    change('2026-07-30T13:05:00Z', PERSON.moreau, 'Julien Moreau', 'status', 'draft', 'submitted'),
    change(
      '2026-07-30T12:58:00Z',
      PERSON.moreau,
      'Julien Moreau',
      'expected_outcomes',
      null,
      { fr: "Un référentiel d'écoconception partagé par les trois administrations pilotes." },
    ),
    change('2026-07-29T09:20:00Z', PERSON.moreau, 'Julien Moreau', 'format', 'in_person', 'hybrid'),
    creation('2026-07-21T14:10:00Z', PERSON.moreau, 'Julien Moreau'),
  ],

  // Dossier EN COURS D'ÉVALUATION — celui que la fiche d'évaluation (A8) montre.
  // Ajouté au prompt A8 : l'onglet « Historique des modifications » du
  // back-office ne peut pas se démontrer sur un dossier déjà décidé, et aucun
  // des trois historiques écrits jusqu'ici ne portait sur un dossier que le
  // comité est en train de lire. Ce qu'il raconte est ce qu'un membre du comité
  // vient y chercher : le dossier a-t-il bougé depuis que je l'ai lu ?
  [PROPOSAL.budgetsGenre]: [
    // La dernière modification porte la date de `updated_at` du dossier : les
    // deux se lisent côte à côte dans l'écran, et diverger ici passerait pour un
    // historique incomplet.
    change(
      '2026-08-05T16:20:00Z',
      PERSON.ngoBassong,
      'Estelle Ngo Bassong',
      'summary',
      {
        fr: "Un marqueur budgétaire ne suffit pas : encore faut-il que la direction du budget sache s'en servir.",
      },
      {
        fr: "Un marqueur budgétaire ne suffit pas : encore faut-il que la direction du budget sache s'en servir.",
        en: 'A budget marker is not enough: the budget department still has to know how to use it.',
      },
    ),
    change('2026-07-27T10:00:00Z', PERSON.bakayoko, 'Aminata Bakayoko', 'status', 'submitted', 'under_review'),
    change('2026-07-02T09:30:00Z', PERSON.ngoBassong, 'Estelle Ngo Bassong', 'status', 'draft', 'submitted'),
    // Le créneau souhaité a reculé de trois jours pendant la rédaction : c'est
    // exactement le genre d'arbitrage que la v1 perdait, ses dates proposées et
    // retenues vivant dans la même colonne.
    change(
      '2026-06-30T14:12:00Z',
      PERSON.ngoBassong,
      'Estelle Ngo Bassong',
      'preferred_start_at',
      '2027-11-15T11:00:00-03:00',
      '2027-11-12T14:00:00-03:00',
    ),
    change('2026-06-28T09:40:00Z', PERSON.ngoBassong, 'Estelle Ngo Bassong', 'format', 'in_person', 'hybrid'),
    creation('2026-06-24T11:00:00Z', PERSON.ngoBassong, 'Estelle Ngo Bassong'),
  ],

  // Dossier d'une édition passée : l'historique ne s'efface pas avec l'édition.
  [PROPOSAL.cop30Littoraux]: [
    change(
      '2025-11-12T18:30:00Z',
      PERSON.mbayeNdiaye,
      'Ousmane Mbaye Ndiaye',
      'expected_outcomes',
      { fr: "Un protocole de transmission des relevés aux points focaux nationaux." },
      {
        fr: "Un protocole de transmission des relevés aux points focaux nationaux, adopté par les quatre pays représentés.",
      },
    ),
    change('2025-08-04T13:00:00Z', PERSON.bakayoko, 'Aminata Bakayoko', 'status', 'under_review', 'accepted'),
    change('2025-07-02T08:00:00Z', PERSON.bakayoko, 'Aminata Bakayoko', 'status', 'submitted', 'under_review'),
    change('2025-06-18T10:20:00Z', PERSON.sowFall, 'Awa Sow Fall', 'status', 'draft', 'submitted'),
    creation('2025-06-09T08:40:00Z', PERSON.sowFall, 'Awa Sow Fall'),
  ],
}

/**
 * `programme.proposal_history(proposal_id)`.
 *
 * Rend un tableau VIDE — et non `null` — pour un dossier sans historique écrit :
 * la fonction SQL ne renvoie jamais de ligne nulle, et un écran qui distinguerait
 * « pas d'historique » de « historique vide » inventerait un état de plus.
 */
export function proposalHistory(proposalId: string): ProposalHistoryEntry[] {
  return proposalHistories[proposalId] ?? []
}
