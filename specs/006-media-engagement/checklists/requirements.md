# Specification Quality Checklist: Média + Engagement (B6)

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-08-21
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

**9 histoires, 15 cas limites, 113 exigences, 29 critères, aucun marqueur de clarification.**

**Sur le premier point de la première section, une réserve nommée.** Les 113 exigences ne citent aucune
table, colonne, contrainte ni fonction — vérifié mécaniquement : les seuls identifiants techniques des
sections d'exigences et de critères sont **quatre chemins de crates et deux noms de binaires**, tous dans
les quatre premières exigences. Ils y sont parce que le livrable lui-même est une contrainte de
structure : le prompt impose **deux** crates et interdit qu'ils se connaissent, et le principe II de la
constitution en fait une vérification mécanique bloquante. L'écrire ailleurs qu'en exigence reviendrait à
ne pas le tenir. Partout ailleurs, les exigences énoncent des règles — « le même contenu déposé deux fois
n'écrit pas un second objet », « une organisation lit un nombre de destinataires, jamais un nom ».

**Une seule question au commanditaire, posée en mots simples, et elle ne bloque pas la planification.**
Faut-il obliger à décrire une image au moment où on la dépose (écart n° 129) ? L'option tenue
provisoirement — obliger, et refuser clairement en nommant le champ — est la seule dont le symptôme est
visible : ne pas obliger produit une image qui reste indéfiniment « en traitement », c'est-à-dire un
emplacement vide que personne ne sait expliquer. En changer ne toucherait qu'un embranchement de la
validation du dépôt.

**Le prompt demande une chose déjà faite, et la spécification le dit plutôt que de la refaire.** L'écart
n° 32 — assainir le HTML de la présentation détaillée d'un dossier — a été livré par B4 le 21/08 :
liste blanche relevée sur la barre d'outils de l'éditeur, assainissement **à l'écriture**, sur l'unique
chemin d'écriture de la colonne. Le refaire ici produirait un second filtre, et il ne pourrait pas y
vivre : la colonne appartient à un schéma qu'aucun de ces deux crates n'a le droit de toucher.
**L'intention de l'écart est tenue autrement** : la même règle est appliquée là où elle ne l'est pas
encore — le corps HTML des modèles de courriel, saisi par un administrateur et envoyé à des milliers de
personnes (FR-082, SC-022).

**Douze écarts nouveaux, n° 126 à 137, tous vérifiés dans le SQL et dans le code livré, jamais supposés.**
Deux produiraient un défaut entièrement silencieux :

- **n° 126** — `programme.registration.confirmed`, l'événement que le modèle nomme lui-même comme
  déclencheur de la matérialisation des rappels, **n'existe pas**. L'énumération des statuts d'inscription
  ne le contient pas, et le déclencheur émet une *création* portant le statut. Un consommateur écrit
  d'après ce commentaire ne serait jamais réveillé : **aucun rappel ne partirait**, sans erreur, sans
  trace, sans que personne ne s'en aperçoive avant le jour de la séance.
- **n° 129** — le texte alternatif manque au contrat des trois écrans qui téléversent, alors que le
  modèle interdit à une image d'être servie sans lui. L'objet resterait « en traitement » pour toujours.

Deux autres engagent des choix d'architecture que le plan devra trancher :

- **n° 127** — **aucune permission propre au média n'existe** dans le modèle, pour dix modules qui en ont.
  La spécification pose la règle qui en découle — le droit de rattacher est le droit d'écrire sur
  l'entité porteuse — et exige qu'aucune ligne de la table blanche ne reste sans garde, vérifié par un
  test qui la parcourt.
- **n° 133** — les courriels de B1 et B2 ne passent ni par le journal d'expédition ni par la liste de
  suppression. Une adresse en rebond dur continue de recevoir des invitations, et c'est la réputation de
  **tous** les envois qui en pâtit. La spécification exige que la garde s'applique à eux **sans qu'aucun
  module livré ne change d'une ligne** (FR-099, FR-100) ; le plan dira comment.

**Trois réserves de périmètre, dites plutôt que laissées à deviner.**

- **Six parties du schéma `engagement` ne sont pas livrées** : commentaires, réactions, messagerie
  directe, mise en relation, blocages et infolettres. Aucune n'a d'écran, la messagerie est fermée par
  drapeau, et le modèle déclare lui-même les infolettres hors phase 1.
- **Le rappel d'échéance d'un appel à propositions, relevé comme obligation de B6 en livrant B3, reste
  hors de ce jalon** (H10) : il suppose un périmètre de destinataires — quelles organisations, à quel
  titre — que rien ne définit aujourd'hui. Le mécanisme livré ici le rendra mécanique le jour où ce
  périmètre sera arbitré. Le dire maintenant évite qu'on le croie oublié.
- **Le canal de notification par poussée est hors périmètre** : aucun client ne s'y abonne.

**Une dépendance que la spécification rend explicite.** L'agrégation des rappels a **deux** lecteurs — la
lecture par séance et la composition de l'espace organisation, servie par un autre module. FR-052 exige
qu'elle soit écrite une seule fois. C'est la seule exigence de cette spécification qui pourrait justifier
un ajout au modèle : une **fonction de lecture**, jamais une table ni une colonne, sur le précédent exact
de l'image rattachée que trois modules appellent déjà. FR-004 borne d'avance ce qui est admissible.

> **Tranché le 21/08 par le plan** (`research.md` § R17) : `engagement.session_reminder_schedule()` est
> ajoutée à `110_engagement.sql` § 6. Ni table, ni colonne, ni type. C'est **la seule** modification du
> modèle demandée par ce jalon, et le SQL n'a pas encore été écrit — l'implémentation le fera, et il
> faudra alors détruire la base.

> **Le plan a aussi amendé cette spécification** sur deux points, datés dans sa section « Amendements » :
> le téléversement passe de trois temps à un seul geste (§ R6), et les déclinaisons d'image sont trois
> tailles plutôt que deux formats (§ R12). Le premier amendement était annoncé comme repli par H1.
