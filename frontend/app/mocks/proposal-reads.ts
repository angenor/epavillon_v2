/**
 * `programme.proposal_reads` — qui a déjà OUVERT quel dossier.
 *
 * POURQUOI CETTE TABLE MÉRITE DES DONNÉES SIMULÉES. La liste du back-office (A7)
 * porte un indicateur discret : les dossiers que la personne connectée n'a pas
 * encore consultés. Sans accusés de lecture écrits ici, l'indicateur serait vrai
 * partout — quarante lignes signalées comme neuves —, c'est-à-dire un signal qui
 * ne distingue rien, et le seul cas qu'il faut voir (« il en reste six que je
 * n'ai jamais ouverts ») ne se produirait jamais à l'écran.
 *
 * CE QUE LE JEU RACONTE, et il est écrit pour cela :
 *
 *  · Mme Perret, administratrice de la SEULE COP31 — le compte qui éprouve le
 *    périmètre d'administration —, a parcouru l'essentiel des dossiers déposés
 *    mais pas les derniers arrivés ni les brouillons. C'est elle que l'écran
 *    montre par défaut, et elle voit donc un indicateur qui a du sens.
 *  · Les membres du comité n'ont ouvert QUE ce qui leur est confié : leur
 *    lecture suit leurs affectations, ce que fait un révisionniste réel.
 *  · M. Bakayoko, qui répartit la charge, a tout ouvert une fois — c'est le cas
 *    « aucun dossier neuf », qui doit rendre l'indicateur totalement absent.
 *
 * OÙ CE FICHIER VIT, et pourquoi pas dans `mocks/proposals/` : il lit les
 * affectations de revue, et `mocks/reviews.ts` lit lui-même les propositions.
 * Rangé sous `proposals/`, il aurait bouclé — `proposals/index` → `reads` →
 * `reviews` → `proposals/index` — et un cycle d'imports ne se manifeste pas par
 * une erreur claire mais par un tableau vide au chargement d'un module. Il se
 * range donc à côté de `reviews.ts`, comme lui à la racine.
 *
 * `read_count` et `last_read_at` ne sont pas décoratifs : la vue expose
 * `read_count` (« lu par 3 membres du comité »), et une relecture est un
 * incrément, pas une seconde ligne — la clé primaire est (dossier, personne).
 */

import type { ProposalRead } from '~/types/programme/proposal'
import { PERSON } from './ids'
import { reviewAssignments } from './reviews'
import { acceptedProposals, reviewedProposals, submittedProposals } from './proposals'

/** Une lecture : première ouverture, dernière ouverture, nombre de passages. */
function read(
  proposal_id: string,
  person_id: string,
  first_read_at: string,
  options: { lastReadAt?: string; count?: number } = {},
): ProposalRead {
  return {
    proposal_id,
    person_id,
    first_read_at,
    last_read_at: options.lastReadAt ?? first_read_at,
    read_count: options.count ?? 1,
  }
}

/**
 * LES DEUX DERNIERS DÉPOSÉS RESTENT NEUFS pour l'administratrice de la COP31 :
 * c'est ce qui donne à l'indicateur quelque chose à signaler. Ils sont désignés
 * par leur rang dans le fichier des dossiers déposés, et non par un identifiant
 * recopié — ajouter un dossier au jeu ne demande alors rien ici.
 */
const NEUFS_POUR_PERRET = 2

const lusParPerret = [
  ...acceptedProposals,
  ...reviewedProposals,
  ...submittedProposals.slice(0, Math.max(0, submittedProposals.length - NEUFS_POUR_PERRET)),
]

export const proposalReads: ProposalRead[] = [
  // L'administratrice de l'édition : tout sauf les derniers arrivés et les
  // brouillons — un brouillon n'est pas un dossier reçu, personne ne l'ouvre.
  ...lusParPerret.map((p, index) =>
    read(p.id, PERSON.perretAdmin, '2026-08-04T08:30:00Z', {
      lastReadAt: '2026-08-14T09:15:00Z',
      count: 1 + (index % 3),
    }),
  ),

  // Le comité lit ce qui lui est confié, et rien d'autre. Les déports sont
  // exclus : une personne qui s'est retirée n'a pas à figurer parmi les
  // lecteurs d'un dossier dont elle a déclaré ne pas pouvoir juger.
  ...reviewAssignments
    .filter((a) => a.recused_at === null)
    .map((a) =>
      read(a.proposal_id, a.reviewer_id, a.assigned_at, {
        lastReadAt: '2026-08-12T16:40:00Z',
        count: 2,
      }),
    ),

  // Celui qui répartit la charge a tout vu passer : le cas « aucun dossier neuf ».
  ...[...acceptedProposals, ...reviewedProposals, ...submittedProposals].map((p) =>
    read(p.id, PERSON.bakayoko, '2026-08-01T07:00:00Z', { lastReadAt: '2026-08-15T18:00:00Z', count: 4 }),
  ),
]
