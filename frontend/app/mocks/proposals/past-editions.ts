/**
 * Les dossiers des ÉDITIONS PASSÉES.
 *
 * POURQUOI CE FICHIER EXISTE. Les quarante dossiers du jeu visent tous la COP31,
 * qui se tient en novembre 2027 : tout y est à venir. Or l'espace organisation
 * (A5) doit rendre ce qui est DERRIÈRE autant que ce qui est devant — une séance
 * tenue, ses inscrits présents, ses rappels déjà partis, son compte rendu
 * attendu. Aucune de ces quatre choses n'a de données sur une édition future, et
 * les inventer sur la COP31 aurait produit un programme qui se serait tenu
 * avant d'avoir commencé.
 *
 * Le dossier ci-dessous est celui du ROAC à la COP30 de Belém (2025). Il n'est
 * pas ajouté pour faire nombre : la séance existait déjà dans
 * `mocks/sessions/other-editions.ts` (`cop30Adaptation`), portée par le ROAC et
 * marquée `completed`, mais SANS DOSSIER D'ORIGINE — `proposal_id` était nul,
 * comme si l'IFDD l'avait programmée d'office. Ce fichier lui rend le dossier
 * qu'elle a forcément eu, et l'espace organisation sait alors remonter de la
 * séance tenue jusqu'à la proposition déposée l'année d'avant.
 */

import type { Proposal } from '~/types/programme/proposal'
import { CALL, COUNTRY, EVENT, ORG, PERSON, PROPOSAL } from '../ids'
import { proposal } from './_shared'

export const pastEditionProposals = [
  proposal({
    id: PROPOSAL.cop30Littoraux,
    // Numéro de la COP30 : la séquence est GLOBALE en base et ne repart pas à
    // zéro d'une édition à l'autre, mais le sigle change — `COP30-00007`.
    ref: 7,
    event: EVENT.cop30,
    call: CALL.cop30,
    organization: ORG.roac,
    submittedBy: PERSON.sowFall,
    contact: PERSON.mbayeNdiaye,
    title: {
      fr: "Littoraux d'Afrique de l'Ouest : trois ans de suivi",
      en: 'West African coastlines: three years of monitoring',
    },
    slug: 'cop30-littoraux-afrique-ouest',
    summary: {
      fr: "Restitution du réseau d'observation côtière : ce que trois campagnes de mesure ont changé dans les plans nationaux d'adaptation.",
      en: 'Findings from the coastal observation network and their effect on national adaptation plans.',
    },
    objectives: {
      fr: "Rendre publiques les mesures de recul du trait de côte relevées entre 2022 et 2025, et confronter leur usage réel dans les plans nationaux d'adaptation de quatre pays.",
    },
    presentation: {
      fr: "<p>Le réseau d'observation côtière ouest-africain a mené trois campagnes de mesure sur onze sites habités. Les données existent, elles sont publiques, et elles ne sont presque jamais reprises dans les documents de planification nationale — qui continuent de citer des relevés antérieurs à 2015.</p><p>La séance présente les résultats, puis donne la parole à deux administrations qui ont, elles, révisé leur plan sur cette base : ce qu'il a fallu pour y parvenir, et ce qui a bloqué ailleurs.</p>",
      en: '<p>The West African coastal observation network completed three measurement campaigns across eleven inhabited sites.</p>',
    },
    outcomes: {
      fr: "Un protocole de transmission des relevés aux points focaux nationaux, adopté par les quatre pays représentés.",
    },
    audience: [
      { fr: "Points focaux nationaux" },
      { fr: "Agences d'aménagement du littoral" },
      { fr: "Chercheurs en géomorphologie" },
    ],
    format: 'hybrid',
    category: 'results_sharing',
    languages: ['fr', 'en'],
    country: COUNTRY.sn,
    preferredStart: '2025-11-12T14:00:00-03:00',
    preferredEnd: '2025-11-12T15:30:00-03:00',
    duration: 90,
    status: 'accepted',
    submittedAt: '2025-06-18T10:20:00Z',
    decidedAt: '2025-08-04T13:00:00Z',
    decisionReason: "Restitution attendue, données inédites, deux administrations engagées à venir en parler.",
    decidedBy: PERSON.bakayoko,
    createdAt: '2025-06-09T08:40:00Z',
    updatedAt: '2025-11-12T18:30:00Z',
    averageScore: 17.2,
    weightedScore: 34.4,
    reviewCount: 3,
    viewCount: 612,
  }),
] satisfies Proposal[]
