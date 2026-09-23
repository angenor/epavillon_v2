/**
 * Le texte du Guide des négociations — CdP31, tel que la maquette le montre et
 * le cite (`docs/AppNego/design/ecrans/04-lecteur.html`, `design/donnees-lecteur.md`).
 * C'est le contenu du PDF : il ne se traduit pas, contrairement aux titres et
 * aux libellés que `negotiation-documents.ts` résout selon la langue.
 */

import type { Block, HeadingBlock, OutlineEntry, Span } from '~/types/negotiation-documents'

export const s = (text: string): Span => ({ text })
const terme = (text: string): Span => ({ text, italic: true, term: true })
export const para = (...spans: Span[]): Block => ({ kind: 'paragraph', spans })
const puce = (text: string): Block => ({ kind: 'list_item', depth: 0, marker: '•', spans: [s(text)] })

export function entree(title: string, level: 1 | 2 | 3, page: number, children: OutlineEntry[] = []): OutlineEntry {
  return { title, level, page_index: page, children }
}

/** Le sommaire de la maquette (`04-lecteur`, écran 04), titres compris. */
export const SOMMAIRE_DU_GUIDE: OutlineEntry[] = [
  entree('Résumé exécutif', 1, 8),
  entree('Introduction générale', 1, 11),
  entree('1. Décryptage des résultats de la CdP29', 1, 14, [
    entree('1.1 Nouvel objectif collectif quantifié', 2, 16),
    entree("1.2 Article 6 de l'Accord de Paris", 2, 17),
    entree('1.3 Adaptation', 2, 18),
    entree('1.4 Bilan mondial', 2, 19),
    entree('1.5 Pertes et préjudices', 2, 19),
    entree('1.6 Programme de travail sur la transition juste', 2, 20),
    entree('Tableau de synthèse des décisions prises à la CdP29', 2, 21),
  ]),
  entree('2. État des négociations aux intersessions de Bonn', 1, 26, [
    entree('2.1 Bilan mondial', 2, 26),
    entree("2.2 Questions relatives à l'adaptation", 2, 26),
    entree('2.3 Mécanisme international de Varsovie pour les pertes et préjudices', 2, 28),
    entree('2.4 Programme de travail sur la transition juste', 2, 30),
    entree("2.5 Programme de travail sur l'atténuation", 2, 31),
    entree('2.6 Mesures de riposte', 2, 33),
    entree('2.7 Recherche', 2, 35),
    entree('2.8 Agriculture', 2, 36),
    entree('2.9 Mécanisme pour un développement propre', 2, 38),
    entree('2.10 Article 6', 2, 39),
    entree('2.11 Questions méthodologiques', 2, 41),
    entree('2.12 Coopération', 2, 42),
    entree("2.13 Action pour l'autonomisation climatique", 2, 43),
    entree('2.14 Résumé des résultats clés des SB62', 2, 44),
  ]),
  entree('3. Enjeux et perspectives en route vers la CdP', 1, 47, [
    entree('3.1 Ordre du jour', 2, 47),
    entree('3.2 Bilan mondial', 2, 49),
    entree('3.3 Financement climatique', 2, 50),
    entree('3.4 Article 6', 2, 56),
    entree('3.5 Atténuation', 2, 57),
    entree('3.6 Adaptation', 2, 59, [
      entree("3.6.1 Objectif mondial en matière d'adaptation (GGA)", 3, 59),
      entree("3.6.2 Plans nationaux d'adaptation", 3, 60),
      entree('3.6.3 Programme de travail de Nairobi', 3, 61),
    ]),
    entree('3.7 Pertes et préjudices', 2, 62, [
      entree('3.7.1 Mécanisme de Varsovie', 3, 62),
      entree('3.7.2 Fonds de réponse', 3, 63),
      entree('3.7.3 Réseau de Santiago', 3, 64),
    ]),
  ]),
  entree('Annexes', 1, 65, [
    entree('A.1 Abréviations et acronymes', 2, 66),
    entree('A.2 Fiches thématiques', 2, 68),
    entree('A.3 Principes fondamentaux de la CCNUCC', 2, 72),
    entree('A.4 Science (GIEC)', 2, 77),
    entree('A.5 Principaux groupes de négociation', 2, 79),
    entree('A.6 Tournants clés récents', 2, 83),
  ]),
  entree('Bibliographie', 1, 88),
]

/**
 * Le texte des pages que la maquette montre ou cite. La page 59 est prise telle
 * quelle ; les pages 18, 26 et 68 portent les extraits de la recherche
 * « progrès collectifs », pour que ses cinq passages se retrouvent.
 */
export const PAGES_DU_GUIDE: Record<number, Block[]> = {
  1: [
    { kind: 'heading', level: 1, spans: [s('Guide des négociations — CdP31')] },
    para(s('Institut de la Francophonie pour le développement durable · Climate Analytics Africa')),
  ],
  8: [
    para(s('Ce guide retient trois priorités pour la CdP31 :')),
    puce("finaliser le suivi de l'objectif mondial en matière d'adaptation ;"),
    puce('rendre opérationnel le nouvel objectif collectif quantifié ;'),
    puce('ancrer la transition juste dans les contributions nationales.'),
  ],
  17: [
    para(s("Les règles des marchés du carbone de l'article 6 ont été complétées à Bakou¹.")),
    { kind: 'note', mark: '1', spans: [s('Article 6.2 : approches coopératives ; article 6.4 : mécanisme de crédit.')] },
  ],
  18: [
    para(
      s('La décision sur le '),
      terme('UAE Framework for Global Climate Resilience'),
      s(" donne un cadre pour mesurer les progrès collectifs vers l'objectif mondial en matière d'adaptation."),
    ),
  ],
  21: [{ kind: 'origin', reason: 'table', text: [s('Décision · Thématique · Résultat · Suite attendue à la CdP30')] }],
  26: [
    para(s("Sur l'adaptation, les Parties divergent sur la manière de rendre compte des progrès collectifs sans créer de nouvelles obligations.")),
  ],
  59: [
    {
      kind: 'heading',
      level: 3,
      spans: [s("3.6.1 Objectif mondial en matière d'adaptation ("), terme('global goal on adaptation'), s(', GGA)')],
    },
    para(
      s(
        "Les enjeux du GGA, à la CdP30, sont cruciaux, car la Conférence vise à élever l'adaptation au même niveau de centralité que l'atténuation dans la réponse climatique mondiale. La CdP30 représente une étape stratégique pour finaliser les éléments clés permettant un suivi robuste des progrès collectifs en matière d'adaptation et pour renforcer la résilience des sociétés et des écosystèmes. L'enjeu principal est la finalisation et l'adoption des 100 indicateurs du GGA, afin de permettre un suivi précis et mesurable des progrès collectifs. Les négociations portent également sur l'intégration des Plans Nationaux d'Adaptation (PNA), la mobilisation des financements nécessaires, et la prise en compte des dimensions humaines et sociales, en particulier pour les populations les plus vulnérables. L'objectif est de traduire les engagements politiques en actions concrètes, mesurables et soutenables, tout en garantissant la cohérence avec les CDN, les priorités nationales et les objectifs climatiques globaux.",
      ),
    ),
    para(
      s(
        "Les principaux mandats et enjeux de négociation incluent : la finalisation du cadre du GGA et l'adoption des 100 indicateurs, l'élévation de la centralité de l'adaptation pour qu'elle soit considérée au même niveau que l'atténuation, le renforcement des PNA comme outils stratégiques pour la mise en œuvre et la mobilisation des ressources, le financement de l'adaptation pour garantir un soutien adéquat et massif, et enfin l'intégration des dimensions humaines et sociales, incluant la justice climatique et le développement résilient face aux impacts du changement climatique.",
      ),
    ),
  ],
  68: [para(s("La fiche sur l'Accord de Paris rappelle les indicateurs retenus pour suivre les progrès collectifs à l'horizon 2030."))],
}

/** Les titres du sommaire qui commencent à une page, dans l'ordre de lecture. */
export function titresDeLaPage(sommaire: OutlineEntry[], page: number): HeadingBlock[] {
  return sommaire.flatMap((e): HeadingBlock[] => [
    ...(e.page_index === page ? [{ kind: 'heading' as const, level: e.level, spans: [s(e.title)] }] : []),
    ...titresDeLaPage(e.children, page),
  ])
}
