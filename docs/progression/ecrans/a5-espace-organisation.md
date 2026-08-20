# A5 — Espace organisation

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de cet écran est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md).

**État** : ✅

---

## Ce qui a été livré

Fait le 17/08, **repris le même jour** : « une personne doit pouvoir modifier son activité ». Le formulaire de dépôt s'ouvre désormais sur un dossier désigné (`?dossier=<id>`), la règle étant celle du commanditaire — tant que l'ÉDITION n'est pas terminée. Le SQL a été corrigé d'abord (écart n° 38 : le renvoi après correction était refusé passé l'échéance), et deux incohérences du jeu de données réparées. 1 utilitaire pur (`utils/proposal-edit.ts`), 1 fichier de mocks (`proposal-edit.ts`), 2 obligations d'API (n° 38, 39). **2 pages** — `/mon-organisation` et `/mon-organisation/dossiers/:id` (`/en/my-organization`, `/en/my-organization/submissions/:id`) —, 7 composants sous `app/components/workspace/`, 1 utilitaire pur (`utils/proposal-timeline.ts`), 2 fichiers de types (`types/engagement.ts` pour les rappels, `types/organization-workspace.ts` pour les contrats d'écran), 3 fichiers de mocks (`reminders.ts`, `organization-workspace.ts`, `proposals/history.ts`), 4 fichiers de traduction (2 × 2 locales). **Gardée par `requires-organization`** (A2), avec la raison `organization-space` écrite dès A2 pour cet écran. **Le modèle a été corrigé d'abord** : `org.memberships` ne distinguait pas une DEMANDE d'une INVITATION. **Le jeu de données gagne un dossier de la COP30** — sans séance déjà tenue, « rappels envoyés » n'avait aucune donnée. Le bouton « Suivre mon dossier » de la confirmation de dépôt (A4) mène désormais au dossier lui-même. Cinq obligations d'API relevées (n° 33 à 37) ; trois corrections apportées à `UiStatusTimeline` et à l'historique

---

## Complément du 20/08 — l'autre bout de l'invitation

L'espace organisation savait **émettre** une invitation depuis le 17/08 ; le lien du courriel, lui, ne menait nulle
part. `app/pages/invitation.vue` referme la boucle.

**Le chemin n'est pas traduit, et c'est volontaire.** `backend/crates/modules/org/src/mail.rs` compose `/invitation`
en français et `/en/invitation` en anglais — exactement ce que produit `prefix_except_default` sur un chemin sans
`defineI18nRoute`. Traduire ce chemin comme le font les écrans d'authentification casserait les liens déjà partis par
courriel, et ceux-là ne se redéploient pas.

**Aucune session n'est exigée.** Le jeton est la preuve d'adresse, comme pour la vérification d'adresse de B1 : la
personne qu'une invitation vise n'a le plus souvent pas encore de compte. Le middleware `guest` ne refuse rien ici, il
résout seulement la session avant le rendu.

**Quatre issues, quatre suites différentes** — adhésion active, lien périmé (à redemander à l'organisation), lien déjà
utilisé (proposer la connexion), lien invalide (l'accueil, et rien d'autre) —, plus l'état vide quand aucun jeton n'est
porté, et le refus nommé du cas où quelqu'un de connecté suit le lien d'un collègue (`ORG_INVITATION_NOT_YOURS`), qui
offre de se déconnecter et de reprendre.

**Un écart assumé** : hors session, l'écran propose **les deux** suites — créer son mot de passe, ou se connecter.
La réponse de l'API ne dit pas si un compte existe derrière l'adresse, et se tromper coûte dans les deux sens. La
création passe en premier : c'est le cas de la personne qu'une invitation vise le plus souvent.

---

## Écarts relevés en écrivant l'espace organisation (A5, 17/08)

Un défaut du modèle, corrigé dans le SQL avant d'écrire une ligne d'interface (voir « Modifications du modèle »). Quatre autres points sont des **règles ou des lectures que le modèle ne porte pas** et que l'écran compose seul : ce sont des obligations des prompts **B2**, **B4**, **B5** et **B6**, écrites ici pour ne pas être redécouvertes.

| N° | Écart | Où | Ce qu'il coûte à l'interface | Suite donnée |
|---|---|---|---|---|
| **33** | **RÉGLÉ le 17/08 — une adhésion « en attente » ne disait pas QUI attend.** `memberships.status = 'pending'` recouvrait la demande spontanée et l'invitation émise, qui se traitent à l'opposé | `040` § 4 | Tout, sur la section « Membres » : le référent voyait ses propres invitations dans sa file de modération et n'avait qu'un bouton « Accepter » à leur opposer — **il aurait fait entrer quelqu'un qui n'a rien répondu**. Rien non plus pour relancer une invitation, ni pour dire depuis quand elle est partie | **Corrigé dans le SQL** : `invited_by`, `invited_at`, une contrainte de cohérence et un index par file. L'écran affiche « Invitation envoyée le … » ou « Demande reçue le … », et n'offre « Accepter / Refuser » que sur les secondes |
| **34** | **Aucune lecture ne récapitule les rappels d'une séance.** `scheduled_reminders` porte une ligne PAR DESTINATAIRE, par canal et par décalage ; `schedule_session_reminders()` matérialise mais ne rend qu'un compte de créations | `110` § 6 | Une séance à quarante inscrits compte cent soixante lignes. L'organisation n'a besoin que de quatre : « J-2, parti, 40 destinataires ». Et elle **n'a pas à connaître QUI reçoit quoi** — ce sont les données personnelles d'inscrits, pas les siennes. Sans agrégation côté serveur, l'écran chargerait cent soixante lignes pour en afficher quatre, et se verrait remettre une liste nominative qu'il devrait s'interdire d'utiliser | **Reporté au prompt B6** : `GET /sessions/:id/reminders` rend une ligne par (décalage, canal) avec l'état consolidé et le NOMBRE de destinataires. Jamais la liste. Le type existe déjà côté front (`ReminderSlot`) |
| **35** | **Rien ne dit qui peut marquer une demande de correction « résolue ».** `proposal_comments.resolved_at` et `resolved_by` sont une date et une clé étrangère, sans règle | `070` § 6 | L'écran a tranché : le soumissionnaire pose la résolution — lui seul sait qu'il a corrigé — et peut la retirer, une case cochée trop vite ne devant pas exiger un courriel à l'IFDD. Le comité garde la main de son côté. C'est défendable et ce n'est écrit nulle part | **Reporté au prompt B4** : c'est une règle d'autorisation, elle appartient à `identity.has_permission()`. À confirmer aussi : une résolution posée par le déposant vaut-elle clôture pour le comité, ou seulement déclaration ? |
| **36** | **Le décompte des inscrits d'une séance n'est pas exposé séparément de la LISTE.** `registrations.forSession()` existe pour le back-office et exige un périmètre d'administration ; une organisation n'en a pas | `075` § 4 | L'organisation qui anime une séance a besoin de savoir combien de personnes viendront — pour la salle, les documents, l'interprétation. Elle n'a aucun titre à connaître leur identité. Sans décompte agrégé, l'écran n'a que deux options, toutes deux mauvaises : ne rien afficher, ou obtenir la liste nominative et se contenter de la compter | **Reporté au prompt B5** : le décompte (confirmés, liste d'attente, jauge) accompagne la séance dans la réponse de l'organisation ; la liste nominative reste derrière la permission du back-office |
| **37** | **Le journal des transitions ne couvre pas tous les dossiers, et l'écran s'y fiait trop.** `proposal_transitions` est écrit par trigger depuis la création de la ligne, mais les dossiers repris de la v1 n'en auront pas | `070` § 3, `910` | Constaté sur les données simulées, où six dossiers sur quarante ont un journal : la frise affichait « en évaluation — non concerné », BARRÉ, sur des dossiers retenus. Un dossier accepté a forcément été évalué ; c'est la trace qui manque, pas l'étape | **Corrigé côté écran** (`utils/proposal-timeline.ts`) : l'évaluation est franchie dès qu'une décision a été rendue, que le journal la date ou non. **Obligation d'API au prompt B4** : la reprise v1 doit semer les transitions déductibles (`created_at`, `submitted_at`, `decided_at`) plutôt que de laisser un journal vide, sinon chaque écran refera cette déduction à sa façon |

**Un point du prompt n'est pas tenu, et c'est assumé** : le dépôt du COMPTE RENDU d'une activité tenue. L'écran le RÉCLAME — c'est l'une des cinq actions du bloc « ce qui attend une action de votre part », et elle s'appuie sur `sessions.report` et `report_submitted_at`, qui existent — mais l'écriture elle-même suppose un formulaire et, pour les pièces jointes, le téléversement du prompt **B6**. Le prompt A5 ne le demandait pas ; le signaler sans permettre d'y répondre était le moindre mal, l'inverse — une activité tenue dont personne ne rappelle qu'elle attend son compte rendu — étant exactement ce que la v1 laissait faire.

**Un état ne se démontre pas sur les données figées** : un rappel `skipped`. Il suppose une séance annulée ayant des inscrits, ou une adresse en liste de suppression ; le composant le rend (pastille grise, « Non envoyé »), aucune donnée ne l'exerce. Les deux autres états — parti, à venir — le sont sur données réelles, respectivement à la COP30 et à la COP31.

---

## Écarts relevés en rendant un dossier modifiable (A5, retours du 17/08)

La règle du commanditaire — « tant que l'évènement n'est pas terminé, il peut modifier » — a fait apparaître un défaut du modèle, corrigé dans le SQL, et deux obligations d'API.

| N° | Écart | Où | Ce qu'il coûte à l'interface | Suite donnée |
|---|---|---|---|---|
| **38** | **RÉGLÉ le 17/08 — le renvoi d'un dossier corrigé était refusé après l'échéance.** `tg_check_submission_eligibility()` contrôlait la fenêtre de l'appel sur tout passage à `submitted`, sans distinguer le premier dépôt du renvoi | `070` § 3 | Tout, et de la pire façon : le comité demande ses corrections APRÈS la clôture, l'écran réclamait « 1 point à corriger », et la base refusait le renvoi. Un dossier bloqué, une organisation sans issue, et un défaut invisible tant qu'aucun écran ne permettait de corriger | **Corrigé dans le SQL**, base rechargée, deux comportements éprouvés sur un appel clos. **Obligation d'API** : le renvoi est une ROUTE distincte du dépôt (`POST /proposals/:id/resubmit`), sans quoi le contrôle de fenêtre revient par la porte du contrat |
| **39** | **La recomposition d'un dossier en brouillon n'existe nulle part.** Le formulaire travaille sur une structure d'écran ; la base range la même chose dans cinq tables (`proposals`, `proposal_organizations`, `proposal_speakers`, `proposal_documents`, `entity_terms`) | `070`, `020` § 4 | Sans elle, « modifier » n'existe pas : le formulaire ne sait ouvrir qu'un brouillon appartenant à la personne connectée. Recomposée dans la page, elle divergerait du dépôt au premier champ ajouté — et trois conversions y sont piégeuses (heure murale du fuseau de l'ÉDITION, français des textes multilingues, verrouillage de l'identité d'un intervenant qui a un compte) | **Reporté au prompt B4** : `GET /proposals/:id/draft` rend le dossier sous la forme que le formulaire attend. Rejoué en attendant dans `mocks/proposal-edit.ts`, à un seul endroit |

**Ce que l'écran assume, et qu'il DIT** : un dossier retenu reste modifiable — c'est la règle donnée —, mais la modification porte sur le DOSSIER et non sur l'activité déjà programmée. La séance garde son créneau, sa salle, ses inscrits et ses rappels ; le modèle sépare volontairement les deux (décision structurante n° 1 de `070`). L'écran l'annonce en tête plutôt que de laisser croire qu'on vient de déplacer une activité du programme publié. **Reste à trancher avec le commanditaire** : par quel chemin une organisation demande-t-elle un changement de créneau après acceptation ? Le fil d'échanges n'est ouvert que si le comité a écrit le premier.

**Une lecture de la règle, assumée** : un dossier **refusé, retiré ou annulé** n'est pas modifiable, même si l'édition est en cours. Il n'est plus en course, aucune transition ne l'y ramène à l'initiative de l'organisation, et le modifier n'aurait aucun effet. Reprendre un dossier refusé, c'est en déposer un nouveau — l'écran le dit en toutes lettres.

---

## Ce qui a été vérifié le 17/08 sur l'espace organisation, et comment

Un écran qui écrit — une invitation, une réponse, une résolution — ne se prouve pas au rendu statique. Tout ce qui suit a été exercé **dans un navigateur réel**, connecté comme Awa Sow Fall (ROAC) puis comme Fatoumata Sy (sans organisation).

| Contrôle | Résultat |
|---|---|
| **La correction du modèle tient-elle sur une base vierge ?** | `make check-db` (`down -v` puis rechargement complet) **vert**. Les deux colonnes, la contrainte et les deux index relus depuis `information_schema` et `pg_indexes`. **La contrainte a été éprouvée, pas supposée** : `invited_at` sans `invited_by` → refusé (`check_violation`) ; invitation complète → acceptée, avec `is_primary` à **faux** (le trigger ne pose la primauté que sur une adhésion ACTIVE) |
| Le bloc d'actions dit-il vrai ? | Trois lignes pour le ROAC, chacune vérifiable : compte rendu attendu (`COP30-00007`, séance tenue sans `report_submitted_at`), co-organisation à confirmer (`COP31-00020`, `confirmed_at` nul), demande de rattachement (Céline Lambert). Rien de ce que le comité doit faire |
| **Les quatre rappels, cumulés** | Sur la séance de la COP30 : « 2 jours avant », « 1 jour avant », « 1 heure avant », « 30 minutes avant », **tous marqués « Envoyé »**, chacun horodaté à l'heure de Belém (10/11 14:00, 11/11 14:00, 12/11 13:00, 12/11 13:30) et portant **6 destinataires** — les sept inscriptions moins l'annulation, qui ne reçoit rien. La phrase « ils sont cumulés : chaque inscrit les reçoit tous les quatre » est affichée au-dessus |
| Le fil de discussion | Demande de correction étiquetée, réponse en retrait sur fond creusé, marquage « Résolu le 12 juillet 2026 à 09:30 ». **Écriture exercée** : réponse envoyée et apparue dans le fil, résolution retirée puis reposée (« Résolu le 17 août 2026 à 20:08 ») |
| **L'invitation par adresse** | Adresse inconnue → « Invitation envoyée à … », la personne apparaît dans la liste avec « Invitation envoyée le … » et le bouton « Relancer l'invitation » — **jamais « Accepter »**, puisqu'elle n'a pas répondu |
| **La demande d'adhésion tranchée** | « Accepter » sur Céline Lambert → elle quitte la file, et le bloc d'actions passe de **3 à 2**. Ces deux boutons ne faisaient rien avant d'être branchés |
| L'historique champ par champ | Dix entrées sur `COP31-00001`, du dossier créé au dossier retenu. **Trois défauts d'affichage trouvés là** : les états sortaient bruts (`under_review`), les dates aussi (`2027-11-09T14:00:00-03:00`), et « Durée modifié » ne s'accordait pas. Corrigés : « En évaluation », « 9 novembre 2027 à 14:00 », « Champ modifié : Durée » |
| **L'état vide, sur données réelles** | Organisation créée en séance (écran A2), puis espace ouvert : « Rien ne vous attend », « Vous n'avez déposé aucun dossier », et **l'appel en cours mis en avant** avec son échéance, son rebours à 44 jours et le bouton de dépôt. La navigation doit rester CLIENT pour cela — un rechargement complet vide le journal des mocks, comme partout depuis A2 |
| La garde | `/mon-organisation` sans rattachement → renvoi vers `/rattachement-organisation?redirect=…&reason=organization-space`, avec le message écrit dès A2 pour cet écran |
| **Défilement horizontal à 375 px** | `document.body.scrollWidth == 375` sur la vue d'ensemble comme sur la fiche. La frise horizontale retombe en colonne, le calendrier des rappels tient sans troncature |
| Thème clair, anglais, clés brutes | Thème clair vérifié sur l'historique ; `/en/my-organization` et `/en/my-organization/submissions/:id` complets, rappels compris (« 2 days before ») ; **zéro clé brute** relevée par balayage du texte des deux pages |
| `make check-db-safe` et `make check-front` | Verts — `npm run typecheck` à 0 erreur et build de production complet |

**Un piège d'outillage, noté pour la prochaine session** : la barre flottante des Nuxt DevTools se pose au centre-bas de la fenêtre et **intercepte les clics** d'un pilote de navigateur. Trois boutons ont paru inertes avant qu'un clic programmatique ne prouve le contraire. Ce n'est pas un défaut de l'écran, mais cela coûte du temps si on ne le sait pas.
