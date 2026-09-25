# Contrat — ouvrir le lexique sur un terme

Pour le lecteur (1b), l'agenda des sessions (3a) et tout écran à venir. L'appelant ne connaît que **le texte anglais** d'un terme (« contact group », « GGA », « Contact Group »).

## La désignation stable

Chaque entrée porte un `slug` : le terme anglais normalisé, espaces changées en tirets — `contact-group`, `bracketed-text`, `global-goal-on-adaptation`. Posé à la création, **jamais recalculé** quand le terme est corrigé. C'est son adresse.

## La normalisation, identique des deux côtés

Minuscules → sans accents → tout ce qui n'est ni lettre ni chiffre devient une espace → espaces réduites et bordures ôtées.

- Client : `normaliserTerme(texte: string): string` — `utils/guide-nego/lexique.ts`, sur `replier()`.
- Base : `platform.normalize_label(text)`, déjà en place.
- Un test passe les mêmes chaînes aux deux et exige le même résultat.

## Trois façons d'y aller

| Moyen | Pour qui | Comportement |
|---|---|---|
| **Adresse** `/guide-nego/lexique?terme=<texte>` | tout écran qui veut simplement ouvrir | Résout sur le téléphone, sans réseau. Trouvée : remplace l'adresse par `/guide-nego/lexique/<slug>`. Sinon : le lexique, champ prérempli, « aucun résultat » et « Proposer « … » » |
| **Fonction** `resoudreLeTerme(texte, lexique): EntreeLexique \| null` — `utils/guide-nego/lexique.ts` | un écran qui veut savoir avant d'ouvrir (la feuille du lecteur) | Pure, synchrone ; `lexique` vient de `useGnSavoir()` |
| **Base** `negotiation.glossary_resolve(text) RETURNS uuid` | le serveur, s'il doit rattacher une ligne à une entrée | `STABLE` ; entrées publiées ou « À revoir » seulement |

## L'ordre de résolution

Sur les formes normalisées, la première correspondance gagne :

1. le terme ;
2. le sigle ;
3. une variante d'écriture ;
4. le `slug` (un texte qui est déjà une désignation).

**Aucun rapprochement approché** : un terme mal reconnu ouvrirait une mauvaise définition. La tolérance aux fautes est celle de la recherche, pas de la résolution.

## Garanties

- Hors connexion : les trois moyens côté téléphone marchent dès que le savoir a été lu une fois.
- Un `slug` publié reste valide tant que l'entrée existe ; une entrée dépubliée rend `null` et l'adresse tombe sur « aucun résultat ».
