# 02 — Domaine

> Le métier de Guide Négo : ses mots, ses objets, ses règles. **La source de vérité reste `docs/database/*.sql`** : ce fichier dit ce qui existe et ce qui manque, il n'invente aucun nom. Ce qui manque s'ajoute au SQL d'abord.

## La négociation, pour qui n'en vient pas

- **COP** : la conférence annuelle des pays signataires de la convention climat (CCNUCC). Deux semaines, des centaines de réunions, jusqu'à dix-sept en parallèle.
- **Groupes de négociation** : les pays négocient en coalitions — G77 et Chine, Groupe africain, pays les moins avancés (PMA), petits États insulaires… Un pays appartient souvent à plusieurs. Chaque groupe tient ses **réunions de coordination**, fermées, et désigne par thématique un **coordonnateur** (le *lead*) qui parle pour lui en salle.
- **La Francophonie n'est pas un groupe de négociation** : elle réunit des pays de groupes opposés. L'IFDD forme et outille, elle ne coordonne aucune position.
- **Types de réunions** : la plénière (ouverte, interprétée dans les six langues), le groupe de contact, les consultations informelles, puis les *informal informals* et les apartés (*huddles*), fermés et rarement annoncés. Hors plénière, tout se passe en anglais.
- **Thématique** : une filière de négociation — adaptation, finance, genre, article 6… Une négociatrice en suit une ou plusieurs ; un document ou une formation en concerne zéro ou plusieurs.
- **Documents** : ceux de l'IFDD (guide des négociations, résumé pour les décideurs, notes techniques) et ceux de la session (projets de texte, notes informelles, documents L.), en anglais d'abord.

## Trois agendas, trois objets

| Agenda | Objet du modèle | Origine des données |
|---|---|---|
| Sessions de négociation | `negotiation.meetings`, nature `negotiation_session` | Import de la source officielle, signalements validés |
| Réunions de la Francophonie | `negotiation.meetings`, natures `francophone_consultation` et `preparatory_workshop` | Saisies par l'IFDD |
| Pavillon de la Francophonie | `programme.sessions` — un autre schéma, déjà servi par l'API | Appel à propositions de l'ePavillon |

Une réunion de la Francophonie peut aussi être une activité du Pavillon : c'est un **lien facultatif**, pas une fusion.

## Ce qui existe, ce qui manque

| Besoin | Existe déjà | À ajouter au SQL |
|---|---|---|
| Documents | `negotiation.documents` (version, `supersedes_id`, fichier ou lien, réservé ou non, marqueur « utilisable par l'IA »), `negotiation.document_bookmarks`, `media.assets` | Plusieurs thématiques par document, par `reference.entity_terms` ; la forme lisible, les pages et les notes de correction (fait à l'étape 1, [data-model](../../specs/011-guide-nego-documents/data-model.md)) |
| Thématiques | Taxonomie `negotiation_theme` dans `reference.taxonomy_terms` (0c) — `negotiation_track` est la filière d'un espace, pas une thématique | Le suivi d'une thématique par une personne (fait en 0c, `negotiation.theme_subscriptions`) |
| FAQ, lexique, parcours « Ma première COP » | Rien | Tout — avec date de vérification et état de publication |
| Sessions et réunions | `negotiation.meetings`, `negotiation.meeting_registrations` | Salle, point de l'ordre du jour, état « déplacée », origine (officielle ou réseau), heure de dernière lecture, accès ouvert ou limité, titre d'origine en anglais, lien vers une activité du Pavillon |
| Signalements | Rien | Tout — le signalement ne modifie jamais la ligne officielle |
| Accès | Rôle `negotiator`, permission `negotiation.space.access` portée par espace, `identity.negotiator_profiles` | Codes d'invitation et leur usage, demandes d'accès, mode d'admission, appartenance au réseau des négociatrices |
| Session et appareils | `identity.sessions` | Type de client, appareil ; appareils pour les notifications poussées |
| Échanges | `negotiation.channels`, `channel_members`, `channel_messages` (partitionnée par mois), `engagement.conversations`, `direct_messages` | Signalement d'un message, question à un expert |
| Assistant | `tool.knowledge_sources`, `knowledge_chunks`, `assistants`, `conversations`, `conversation_messages` (citations figées), `message_feedback` (motif « dépassé »), `usage_quotas` | État d'obsolescence complet, notes de correction, transcriptions horodatées, **dimension des vecteurs et nom du modèle** |
| Formations et quiz | `training.trainings`, `chapters`, `chapter_resources`, `quizzes`, `quiz_questions`, `quiz_options`, `quiz_attempts` | Quiz rattaché à un document, provenance IA, relecture par un expert, quiz personnel |
| Restitutions | Rien | Tout, cercles compris — après confirmation sur le terrain |
| Notifications | `engagement.notifications`, `notification_types` (canal poussé prévu), préférences | Les types propres à Guide Négo |

Fichiers à lire avant d'écrire : `100_negotiations.sql`, `120_tools.sql`, `125_training.sql`, `030_identity.sql`, `110_engagement.sql`, `075_programme_sessions.sql`, `020_reference.sql`.

## Règles

1. **La source officielle fait foi.** Une session importée porte son origine et son heure de lecture ; si la lecture échoue, l'application le dit et renvoie à la source — [ADR-009](adr/009-la-source-officielle-fait-foi.md).
2. **Un signalement se pose par-dessus**, une fois validé par un administrateur — [ADR-010](adr/010-un-signalement-se-pose-par-dessus.md).
3. **Les chevauchements se signalent, ils ne se bloquent pas** : la règle n° 2 de l'ePavillon vaut pour l'agenda personnel.
4. **L'assistant ne lit que la référence**, validée par un expert — [ADR-011](adr/011-un-corpus-a-deux-etages.md).
5. **Toute source porte un état**, et se corrige par une note — [ADR-012](adr/012-toute-source-porte-un-etat.md).
6. **Aucun quiz publié sans relecture** — [ADR-013](adr/013-aucun-quiz-publie-sans-relecture.md).
7. **Une restitution est privée par défaut** — [ADR-015](adr/015-restitutions-privees-partage-par-cercle.md).
8. **L'accès se teste par permission et par portée**, jamais par nom de rôle ; un administrateur d'un seul événement ne voit que le sien.
9. **Les libellés du métier sont des données** : thématiques, types de documents, rubriques de FAQ vivent en base, jamais dans les fichiers de traduction.

## États

| Objet | États |
|---|---|
| Session de négociation | Prévue · En cours · Déplacée · Annulée · Terminée — plus l'origine « non annoncée, signalée par le réseau » |
| Signalement | Envoyé · Validé · Non retenu |
| Admission | Par code · En attente d'approbation · Admise · Refusée · Révoquée |
| Source de l'assistant | À classer · Valide · À vérifier · Dépassée · Retirée |
| Entrée de FAQ, terme du lexique | Brouillon · Publié · À revoir |
| Quiz | Proposé par l'IA · En relecture · Publié · Rejeté — ou Personnel, non relu |
| Question à un expert | En attente · Répondue · Ajoutée à la FAQ |

## Rôles

| Rôle | Ce qu'il fait |
|---|---|
| Visiteur, compte | Lit ce qui est public ; garde ses favoris et ses thématiques |
| Négociatrice, négociateur | Accède aux modules réservés ; signale ; pose des questions |
| Membre du réseau des négociatrices | En plus : le canal réservé — [ADR-007](adr/007-le-reseau-distingue-pas-le-genre.md) |
| Expert | Valide le fond : FAQ, sources, quiz, notes de correction ; répond aux questions |
| Administrateur | Valide les signalements et les accès, publie les documents, tient les codes et l'import |
