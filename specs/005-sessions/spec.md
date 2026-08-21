# Feature Specification: Sessions (B5)

**Feature Branch**: `main` (aucun crochet de branche n'est enregistré ; le dossier de la fonctionnalité est `specs/005-sessions`)

**Created**: 2026-08-21

**Status**: Draft

**Input**: User description: "Module SESSIONS de l'API ePavillon v2 (Rust + Actix Web + SQLx). Création depuis une proposition acceptée, planification, détection de conflits, rattachement aux journées spéciales, publication, inscriptions avec formulaire configurable. Plus les exigences issues des écarts n° 6, 7 et 36. Livrable : extension de `backend/crates/modules/programme`, monté par `api` et `worker`."

---

## Contexte

**Ce module est celui qui transforme une décision en programme.** B4 a livré les dossiers et leur cycle de vie ; à l'acceptation, il ne se passe aujourd'hui **rien** — `programme.sessions` n'apparaît que si quelqu'un l'insère, et personne ne l'insère (écart n° 57, relevé le 18/08 en écrivant le planificateur). Le planificateur du back-office existe et n'a rien à placer ; la programmation publique existe et reste vide ; l'espace organisation annonce des séances et sert une liste vide (écart n° 108, assumé en B4). Ce module referme les trois.

**Aucun crate n'est créé.** `backend/crates/modules/programme` existe depuis B4 et porte déjà tout le schéma `programme` : la décision est écrite en tête de son `lib.rs` et dans `specs/004-propositions/research.md` § R1 — **un seul crate pour tout le schéma**, B4 y pose la partie « propositions » (fichier `070`), B5 y ajoute la partie « séances » (fichier `075`). Les dossiers internes sont nommés par agrégat, jamais par « proposition » : c'est fait pour recevoir ce jalon sans réorganisation.

Le modèle fait autorité et **n'est pas modifié**. `docs/database/075_programme_sessions.sql` a été relu en entier : ses neuf sections portent la séance et ses trois valeurs dérivées, ses intervenants, sa co-organisation, son rattachement aux journées spéciales, les formulaires d'inscription configurables, les inscriptions et leur liste d'attente, les questions du public, la vue de la programmation publique, la détection des conflits, le contrôle avant publication, l'historique des reports et les statistiques d'édition. S'y ajoutent `060_events.sql` § 3, 3 bis, 4, 4 bis (les jours, les journées spéciales, les salles, les canaux de diffusion), § 5 (les bornes de l'appel, qui donnent la durée par défaut et la plage d'accueil du pavillon), `070_programme_proposals.sql` § 2, 5, 6 (le dossier retenu, ses intervenants, ses co-organisations), `030_identity.sql` § 3 et § 5 (les deux permissions `programme.session.schedule` et `programme.registration.manage`, le périmètre d'administration, et les consentements RGPD), `020_reference.sql` § 4 (les thématiques), `050_media.sql` § 8 (le rôle de couverture d'une séance, semé par `075` § 8) et `010_platform.sql` (audit, historique d'entité, outbox, garde de rejeu).

**Trois décisions du modèle gouvernent tout ce qui suit**, et elles sont écrites en tête du fichier SQL :

1. **Les chevauchements de créneaux ne sont JAMAIS bloqués.** Aucune contrainte d'exclusion n'existe sur les créneaux. Une première version du modèle en posait une : elle transformait un outil d'arbitrage en mur, parce qu'un planificateur travaille par déplacements successifs et passe par des états incohérents. `programme.detect_conflicts()` recense, l'interface montre, l'équipe arbitre. **Le seul garde-fou dur se situe à la publication**, et il appartient à B3, qui l'a livré.
2. **Le formulaire d'inscription est une donnée, les réponses un document.** La v1 a vu sa table d'inscriptions grossir au fil des besoins — six colonnes `guest_*`, un canal d'acquisition, une table annexe de données démographiques : chaque question posée coûtait une migration et un déploiement. Ici, ajouter une question est une insertion, et **la validation est dynamique**, contre le formulaire applicable.
3. **La proposition est un DOSSIER, la séance est une OCCURRENCE PROGRAMMÉE.** B4 a inscrit l'interdiction de propager l'un vers l'autre (FR-091, décision du 21/08). Ce module la lit **dans l'autre sens** : il naît d'un dossier retenu, en recopie ce qu'il faut **une fois**, et cesse ensuite d'en dépendre. Une séance a un créneau arbitré, une salle attribuée, des inscrits prévenus et des rappels programmés ; rien de tout cela ne se rejoue parce qu'un dossier a été corrigé.

Le front existe depuis le 18/08 et consomme des données simulées. Ses contrats — `frontend/app/types/programme/{session,registration}.ts`, `types/admin-planner.ts`, `types/event-programme.ts`, `types/views.ts`, `types/organization-workspace.ts` — et les chemins déclarés dans `composables/api/planner.ts` et dans les blocs `sessions` et `registrations` de `composables/useApi.ts` **sont le contrat de cette API**. Ils ne se renégocient pas.

Le socle existe depuis B1, les frontières depuis B2, l'édition et sa publication depuis B3, le dossier depuis B4 : `kernel` (contexte de requête, erreurs à code stable, unique porte d'écriture, garde d'autorisation testant permission **et** portée, file de travaux, garde de rejeu d'outbox), `contracts`, `api`, `worker`. **Ce module ne les réinvente pas et ne dépend d'aucun autre crate de module.** La règle de frontière posée en B2 s'applique telle quelle : *un module lit hors de son schéma quand la question porte sur ses propres entités, il n'y écrit jamais, et il n'appelle jamais un autre module.*

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Une activité retenue devient une séance à placer (Priority: P1)

Le comité retient un dossier. Sans que personne n'ait à ressaisir quoi que ce soit, l'activité apparaît dans le panneau « à placer » du planificateur : au créneau que l'organisation avait souhaité, sans salle, avec ses intervenants, ses co-organisateurs et ses thématiques. Un dossier qui demandait trois occurrences en produit trois, numérotées.

**Why this priority**: sans elle, le planificateur n'a rien à placer et tout le reste du module est sans objet. C'est l'écart n° 57, ouvert depuis le 18/08 et explicitement renvoyé à ce jalon.

**Independent Test**: retenir un dossier demandant une occurrence, puis un dossier demandant trois, et constater dans le panneau « à placer » une séance puis trois — sans salle, au créneau souhaité, avec le même nombre d'intervenants que le dossier ; retenir une seconde fois le même dossier et constater qu'aucune séance de plus n'est créée.

**Acceptance Scenarios**:

1. **Given** un dossier en évaluation demandant **une** occurrence, **When** il est retenu, **Then** une séance est créée dans la même transaction, à l'état « créneau pressenti », **sans salle**, non publiée, au créneau souhaité par l'organisation, dans le fuseau de l'édition.
2. **Given** un dossier demandant **trois** occurrences, **When** il est retenu, **Then** trois séances sont créées, portant les rangs 1, 2 et 3, chacune avec sa propre adresse d'URL.
3. **Given** un dossier retenu, **When** ses séances sont créées, **Then** elles portent son titre, son résumé, son format, sa durée, ses **intervenants** (personne, rôle, fonction et organisation telles que déclarées, notice, ordre), ses **co-organisations** hors porteur, et ses **thématiques**.
4. **Given** un dossier dont le créneau souhaité n'a pas été renseigné, **When** il est retenu, **Then** la séance est créée quand même : elle est datée du **premier jour de l'édition, à l'heure d'ouverture du pavillon déclarée par l'appel**, et sa durée est celle du dossier, à défaut la durée par défaut de l'appel.
5. **Given** un dossier dont la durée n'est pas renseignée, **When** il est retenu, **Then** la fin de la séance est calculée depuis la durée par défaut de l'appel, jamais laissée à zéro : le modèle exige une fin strictement postérieure au début.
6. **Given** un dossier déjà retenu et porteur de séances, **When** la même transition est rejouée, ou une action groupée le retient une seconde fois, **Then** **aucune séance supplémentaire n'est créée** — le rang d'occurrence est unique par dossier, et le service ne crée que ce qui manque.
7. **Given** l'acceptation d'un dossier, **When** les séances sont créées, **Then** le service **n'émet aucun événement de création de séance** : le déclencheur du modèle l'émet déjà, une fois par séance.
8. **Given** douze dossiers retenus par une action groupée, **When** elle aboutit, **Then** chaque dossier produit ses séances, et un échec sur l'un **n'annule pas** les autres selon la règle déjà posée par l'action groupée de B4.
9. **Given** un dossier retenu puis corrigé par son organisation, **When** la correction est enregistrée, **Then** la séance reste **strictement inchangée** — c'est la garantie livrée par B4, et elle n'est pas affaiblie par ce module.

---

### User Story 2 — Le planificateur voit tout son écran en une réponse (Priority: P1)

Une chargée de programmation ouvre le planificateur d'une édition. En une seule réponse arrivent : les jours du calendrier, les salles, les journées spéciales, les canaux de diffusion, les séances installées, les séances qui restent à placer, et **les chevauchements**. Elle voit l'édition qu'on lui a confiée, et rien d'autre.

**Why this priority**: c'est l'écran d'arbitrage. Les conflits ne sont pas un second appel — une grille affichée avant de savoir ce qui s'y chevauche montre, pendant une seconde, une programmation qui a l'air saine.

**Independent Test**: charger l'écran avec un compte détaché sur une seule édition, vérifier que les six listes et les conflits arrivent ensemble ; recharger avec l'identifiant d'une autre édition et constater le même refus qu'avec un identifiant inexistant.

**Acceptance Scenarios**:

1. **Given** une administratrice détenant le droit de planifier sur une édition, **When** elle demande l'écran, **Then** la réponse porte l'édition, son fuseau et le libellé de sa ville, la date de publication du programme, les jours, les salles, les journées spéciales, les canaux, les séances **placées**, les séances **à placer** et les conflits.
2. **Given** cette réponse, **When** on la lit, **Then** une séance sans salle figure dans « à placer » et jamais dans « placées », et réciproquement.
3. **Given** une séance née d'un dossier, **When** elle est rendue, **Then** elle porte le numéro du dossier, sa note consolidée, la durée souhaitée, le créneau souhaité et les contraintes déclarées au dépôt — de quoi trier le panneau sans une requête de plus.
4. **Given** une séance, **When** elle est rendue, **Then** elle porte le nom de sa salle, le nom, le sigle et le pays de son organisation porteuse, ses journées spéciales, ses thématiques **avec leur libellé et leur couleur**, et son nombre d'intervenants.
5. **Given** un compte sans aucun droit d'administration, **When** il demande l'écran, **Then** il reçoit un **refus explicite**, jamais une grille vide.
6. **Given** un compte détaché sur une autre édition, **When** il forge l'identifiant d'une édition qu'il n'administre pas, **Then** le refus est **indiscernable** de celui d'une édition inexistante.
7. **Given** une édition dont aucune séance n'est encore née, **When** l'écran est demandé, **Then** il répond avec ses listes **vides**, jamais par une erreur.
8. **Given** l'écran, **When** on demande séparément la liste des séances de l'édition ou ses seuls conflits, **Then** les deux lectures existent et rendent exactement ce que l'écran porte.

---

### User Story 3 — Placer, déplacer, redimensionner, retirer : une seule écriture, jamais refusée pour chevauchement (Priority: P1)

L'équipe fait glisser une activité sur la grille. Elle la déplace, l'allonge, la renvoie au panneau. Rien n'est refusé pour cause de créneau occupé : les blocs qui se recouvrent virent au rouge, le bandeau compte, et l'équipe continue jusqu'à ce que la liste soit vide.

**Why this priority**: c'est la règle métier n° 2, la plus facile à trahir et la plus coûteuse à trahir. Une base qui refuse l'écriture oblige à contourner l'outil.

**Independent Test**: poser une séance sur un créneau déjà occupé par une autre séance de la même édition en salle physique, constater que l'écriture **réussit** et que la réponse porte le conflit en gravité haute ; retirer sa salle et constater qu'elle revient au panneau sans rien perdre.

**Acceptance Scenarios**:

1. **Given** deux séances d'une même édition installées en salle physique sur des créneaux qui se recouvrent, **When** la seconde est enregistrée, **Then** l'écriture **aboutit**, et la réponse porte le conflit de **stand unique** en gravité bloquante.
2. **Given** une séance installée en salle **virtuelle** et une séance en salle physique au même moment, **When** on lit les conflits, **Then** **aucun conflit de stand** n'est remonté : le lieu dit l'occupation, jamais le mode de participation.
3. **Given** une séance **sans salle**, **When** on lit les conflits, **Then** elle n'en produit aucun : c'est l'état normal d'une activité retenue et pas encore installée.
4. **Given** une séance placée, **When** on lui reprend sa salle, **Then** elle revient au panneau « à placer » : ce n'est pas une suppression, la séance existe et son créneau souhaité reste celui du dossier.
5. **Given** n'importe laquelle de ces quatre écritures, **When** elle aboutit, **Then** la réponse porte la séance **et les conflits recalculés pour toute l'édition** — déplacer un bloc peut résoudre le conflit d'un bloc situé à l'autre bout de la semaine.
6. **Given** une écriture qui envoie l'intervalle dérivé ou l'exclusivité de salle, **When** elle arrive, **Then** elle est **refusée en nommant le champ**, et non acceptée en silence pour être écrasée par la base.
7. **Given** une séance déplacée d'un jour à un autre sans que la journée de rattachement soit fournie, **When** l'écriture aboutit, **Then** la séance est rattachée à la **journée du nouveau créneau**, dans le fuseau de l'édition.
8. **Given** une séance déplacée avec une journée de rattachement **explicitement fournie**, **When** l'écriture aboutit, **Then** c'est cette journée qui est retenue : elle reste saisissable.
9. **Given** une écriture dont la fin est antérieure ou égale au début, **When** elle arrive, **Then** elle est refusée sur son champ, avec un message français.
10. **Given** une écriture visant une séance d'une édition hors du périmètre de l'appelante, **When** elle arrive, **Then** le refus est indiscernable de celui d'une séance inexistante.
11. **Given** un intervenant attendu à deux endroits en même temps, ou une organisation programmée deux fois, **When** on lit les conflits, **Then** ils sont remontés en **avertissement**, jamais en gravité bloquante.

---

### User Story 4 — Les journées spéciales sont composées à la main (Priority: P2)

L'équipe compose la « Journée finance durable » en choisissant des activités parmi celles qui ont été retenues. Ce ne sont pas les activités du 12 novembre : c'est une sélection éditoriale, et il arrive d'avoir à l'expliquer à une organisation qui s'étonne de ne pas y figurer.

**Why this priority**: c'est la règle métier n° 7. Elle est déjà tenue par le modèle ; l'API doit se garder de la « simplifier » en la déduisant des dates.

**Independent Test**: rattacher une séance à deux fils, en retirer un, constater que la base retient qui a rattaché quoi et quand ; tenter de rattacher une séance de la COP31 à un fil d'une autre édition et constater le refus.

**Acceptance Scenarios**:

1. **Given** une séance et une liste de journées spéciales, **When** la liste est envoyée, **Then** elle **remplace** la précédente : ce qui n'y figure plus est détaché.
2. **Given** un rattachement, **When** il est écrit, **Then** la base retient **qui** l'a posé et **quand**.
3. **Given** une séance et un fil appartenant à une **autre** édition, **When** le rattachement est tenté, **Then** il est refusé, et le refus de la base est **traduit** en message français exploitable, non rendu brut.
4. **Given** une liste envoyée deux fois à l'identique, **When** la seconde arrive, **Then** l'état final est le même et aucun doublon n'est créé.
5. **Given** un rattachement, **When** il aboutit, **Then** la réponse porte la séance **et** les conflits de l'édition, comme les autres écritures de l'écran.
6. **Given** une séance rattachée à un fil **non publié**, **When** la programmation publique est lue, **Then** le fil n'y apparaît pas : c'est la vue du modèle qui le filtre, et le service ne le refait pas.

---

### User Story 5 — La diffusion, et la règle « un seul direct » (Priority: P2)

L'équipe marque une activité comme diffusée. Le canal se pose de lui-même quand l'édition n'en a qu'un ; quand elle en a plusieurs, l'écran laisse le choix. Deux directs simultanés restent écrivables — et remontent aussitôt en gravité haute.

**Why this priority**: c'est la règle métier n° 4, et le modèle la tient par une affectation automatique précisément pour qu'elle ne dépende pas d'une saisie. Le contrat du front porte pourtant le canal ; les deux se concilient, et il faut dire comment.

**Independent Test**: marquer deux séances comme diffusées sur le même créneau sans choisir de canal, constater que le canal par défaut est posé sur les deux et que le conflit de diffusion remonte en gravité bloquante ; retirer la diffusion de l'une et constater que son canal est effacé et le conflit résolu.

**Acceptance Scenarios**:

1. **Given** une séance non diffusée et une édition ayant un canal par défaut, **When** la diffusion est activée **sans** choisir de canal, **Then** le canal par défaut de l'édition est posé, à défaut le canal général de la plateforme.
2. **Given** une édition ayant plusieurs canaux, **When** la diffusion est activée **avec** un canal choisi, **Then** c'est ce canal qui est retenu — il n'est pas écrasé.
3. **Given** une séance diffusée, **When** la diffusion est retirée, **Then** le canal est **effacé**, et la séance cesse d'occuper une ressource qu'elle n'utilise pas.
4. **Given** une écriture qui retire la diffusion **et** désigne un canal, **When** elle arrive, **Then** elle est **refusée en nommant le champ** : accepter reviendrait à laisser la base effacer en silence une valeur que la personne vient de choisir.
5. **Given** deux séances diffusées sur le même canal et des créneaux qui se recouvrent, **When** on lit les conflits, **Then** le conflit de diffusion est remonté en gravité **bloquante**, y compris quand les deux séances appartiennent à **deux éditions différentes**.
6. **Given** un canal désigné qui n'appartient ni à l'édition ni à la plateforme, ou qui est désactivé, **When** l'écriture arrive, **Then** elle est refusée en le disant.

---

### User Story 6 — La programmation devient réellement publique (Priority: P1)

L'équipe publie le programme. B3 contrôle, estampille l'édition et **annonce** ; ce module reçoit l'annonce et rend publiques exactement les séances désignées. Republier ne publie rien de plus.

**Why this priority**: sans cette moitié, le bouton « Publier » du planificateur estampille une édition dont le programme public reste vide. C'est l'obligation inscrite aux points bloqués le 20/08, et son contrat est déjà fixé.

**Independent Test**: publier une édition portant trois séances éligibles et une séance annulée, comparer le nombre annoncé au nombre de séances devenues publiques, puis rejouer la même annonce et constater qu'aucune séance de plus n'est publiée.

**Acceptance Scenarios**:

1. **Given** l'annonce de publication d'une édition, **When** elle est reçue, **Then** les séances **de cette édition**, aux **états portés par l'annonce**, et **non encore publiques**, reçoivent la date de publication portée par l'annonce — et **pas un autre prédicat**.
2. **Given** cette même annonce livrée une seconde fois, **When** elle est reçue, **Then** **aucune séance de plus n'est publiée** : la garde de rejeu du noyau l'écarte.
3. **Given** cette annonce, **When** elle est traitée, **Then** le module **n'écrit pas** la date de publication de l'édition : elle est déjà posée par l'émetteur.
4. **Given** une publication qui aboutit, **When** on compare le nombre annoncé au nombre de séances effectivement devenues publiques, **Then** ils sont **égaux** en l'absence d'écriture concurrente, et tout écart est **mesuré et consigné**, jamais supposé nul.
5. **Given** une édition sans aucune séance, **When** elle est publiée, **Then** le traitement aboutit sans rien publier et sans erreur.
6. **Given** une séance publiée puis annulée, **When** la programmation publique est lue, **Then** elle y figure avec l'état temporel « annulée » : c'est la vue du modèle qui le décide, et ce module ne dépublie rien.

---

### User Story 7 — Le public lit le programme (Priority: P1)

Un visiteur ouvre la programmation d'une édition. Il voit les activités publiées, leur créneau à l'heure du pavillon, leur salle, l'organisation qui les porte et son pays, leurs thématiques et leurs journées spéciales, leur image, et combien de personnes sont déjà inscrites — en une requête.

**Why this priority**: c'est la page que la COP31 montrera au monde, et trois écrans du front l'appellent déjà.

**Independent Test**: lire la programmation d'une édition sans aucune session ouverte et vérifier que seules les séances publiées apparaissent, que l'état temporel est calculé, et que les libellés de thématiques viennent de la base.

**Acceptance Scenarios**:

1. **Given** une édition dont le programme est publié, **When** la programmation est demandée **sans session**, **Then** elle répond : c'est une lecture publique.
2. **Given** cette réponse, **When** on la lit, **Then** chaque ligne porte son créneau et son fuseau, sa salle, son organisation avec son sigle et son pays, ses journées spéciales, ses thématiques **avec libellé et couleur**, son image de couverture — celle de la séance, **à défaut celle du dossier d'origine** —, son état temporel et son nombre d'inscrits.
3. **Given** une séance non publiée, **When** la programmation est demandée, **Then** elle **n'y figure pas**.
4. **Given** une séance publiée, **When** on demande son détail par son adresse d'URL dans son édition, **Then** elle est rendue avec ses intervenants et ses organisations.
5. **Given** une adresse d'URL inconnue dans une édition connue, **When** le détail est demandé, **Then** la réponse est un refus ordinaire, sans divulguer l'existence d'une séance non publiée portant cette adresse.
6. **Given** une édition dont le programme n'est **pas** publié, **When** la programmation est demandée, **Then** la réponse est **vide**, jamais une erreur : l'écran annonce que le programme n'est pas encore paru.

---

### User Story 8 — S'inscrire à une activité, avec un formulaire qui n'a pas été écrit en dur (Priority: P2)

Une personne s'inscrit à une activité. Les questions qu'on lui pose viennent de la base : l'IFDD en ajoute une sans déploiement. Ses réponses sont validées contre **ce** formulaire — type, obligation, options, bornes —, jamais contre une liste figée dans le code. Quand la salle est pleine, elle passe en liste d'attente si l'activité en tient une, et remonte d'un rang dès qu'une place se libère.

**Why this priority**: c'est la seconde décision structurante du fichier SQL, et c'est ce que la v1 payait au prix d'une migration par question posée.

**Independent Test**: charger le formulaire applicable à une séance, s'inscrire en omettant une réponse obligatoire, puis en donnant une valeur hors options, puis correctement ; remplir la jauge et constater le passage en liste d'attente ; annuler une inscription confirmée et constater la promotion du premier de la file.

**Acceptance Scenarios**:

1. **Given** une séance, **When** son formulaire est demandé, **Then** le formulaire **applicable** est rendu avec ses seuls champs **actifs**, triés : celui de la séance, à défaut celui de son édition, à défaut celui de la plateforme.
2. **Given** un formulaire dont un champ renvoie à une taxonomie, **When** il est rendu, **Then** les options viennent de la base avec leur libellé traduit, **jamais** d'une liste écrite dans l'API.
3. **Given** une inscription omettant une réponse **obligatoire**, **When** elle arrive, **Then** elle est refusée en **nommant le champ**.
4. **Given** une réponse dont le **type** ne correspond pas au champ — un texte pour un nombre, une date mal formée, une adresse invalide —, **When** elle arrive, **Then** elle est refusée en nommant le champ.
5. **Given** une réponse hors des **options** d'un champ à choix, ou hors des **bornes** déclarées par le champ, **When** elle arrive, **Then** elle est refusée en nommant le champ et la règle enfreinte.
6. **Given** une réponse portant une clé qui ne correspond à **aucun champ actif** du formulaire applicable, **When** elle arrive, **Then** elle est refusée : une clé mal orthographiée qui disparaît en silence est une réponse perdue.
7. **Given** un champ marqué **sensible** auquel une réponse est donnée, **When** l'inscription arrive **sans consentement explicite**, **Then** elle est refusée en nommant le champ ; **When** le consentement accompagne la demande, **Then** l'inscription aboutit et le consentement est **conservé comme preuve**, avec sa finalité, sa version de politique, son origine et l'adresse d'appel.
8. **Given** une séance dont la jauge est atteinte et qui tient une liste d'attente, **When** une inscription arrive, **Then** elle est enregistrée **en liste d'attente**, à la position suivante, et la réponse le dit.
9. **Given** une séance dont la jauge est atteinte et **sans** liste d'attente, **When** une inscription arrive, **Then** elle est refusée en portant le nombre de places.
10. **Given** une inscription confirmée annulée, **When** l'annulation aboutit, **Then** **exactement une** personne de la liste d'attente est promue, dans la même transaction, et jamais plus que le nombre de places libérées.
11. **Given** une personne déjà inscrite à une séance, **When** elle s'inscrit à nouveau, **Then** la demande est refusée en le disant ; **Given** une personne dont l'inscription a été **annulée**, **When** elle se réinscrit, **Then** elle le peut.
12. **Given** une séance dont les inscriptions ne sont pas encore **ouvertes**, ou qui n'en prend pas, ou qui est **annulée**, ou dont les inscriptions sont **closes**, **When** une inscription arrive, **Then** chacun de ces quatre refus porte son propre motif.
13. **Given** un formulaire admettant l'inscription **sans compte**, **When** une personne sans session s'inscrit, **Then** son identité est prise dans des **champs dédiés** — adresse, prénom, nom — et jamais dans les réponses au formulaire ; la personne est retrouvée par son adresse ou créée sans compte.
14. **Given** un formulaire **n'admettant pas** l'inscription sans compte, **When** une personne sans session s'inscrit, **Then** elle est refusée en le disant.
15. **Given** une inscription, **When** elle est écrite ou change d'état, **Then** le service **n'émet aucun événement** : le déclencheur du modèle l'émet déjà.
16. **Given** une personne inscrite qui rejoint l'activité, **When** elle clique une première puis une seconde fois, **Then** l'instant de première présence est **écrit une seule fois** et jamais écrasé.
17. **Given** une personne connectée, **When** elle demande ce à quoi elle est inscrite, **Then** elle reçoit **ses** inscriptions, annulations comprises, et jamais celles d'un autre.

---

### User Story 9 — L'organisation sait combien de personnes viendront, jamais qui (Priority: P2)

Une organisation ouvre le dossier qu'elle a fait retenir. Elle y voit ses séances : le créneau arbitré, la salle, et **trois nombres** — inscriptions confirmées, liste d'attente, jauge. Aucun nom.

**Why this priority**: c'est l'écart n° 36, et il ferme l'écart n° 108 laissé ouvert par B4. Une organisation a besoin de ces nombres pour la salle, les documents et l'interprétation ; elle n'a aucun titre à connaître l'identité des inscrits.

**Independent Test**: charger le dossier d'une organisation portant une séance à quarante inscrits et onze en attente, et **balayer la charge utile entière** à la recherche d'un nom, d'une adresse ou d'une réponse au formulaire d'un inscrit — n'en trouver aucun.

**Acceptance Scenarios**:

1. **Given** un dossier retenu portant des séances, **When** son suivi est demandé par son organisation, **Then** la liste des séances est **remplie** : la séance, sa salle, les inscriptions confirmées, la liste d'attente et la jauge.
2. **Given** cette réponse, **When** on la balaie en entier, **Then** elle ne contient **ni nom d'inscrit, ni adresse, ni réponse au formulaire**, quel que soit le champ.
3. **Given** une séance **terminée sans compte rendu**, **When** l'espace de l'organisation est chargé, **Then** une action « compte rendu manquant » y figure, nommant la séance.
4. **Given** un dossier non retenu, **When** son suivi est demandé, **Then** la liste des séances est **vide**, jamais absente.
5. **Given** une organisation, **When** elle demande le suivi d'un dossier qu'elle ne porte pas, **Then** elle est refusée comme aujourd'hui.
6. **Given** une séance, **When** la liste **nominative** de ses inscrits est demandée, **Then** elle exige la permission de gérer les inscriptions **sur l'édition visée**, et un compte hors périmètre reçoit le même refus qu'avec une séance inexistante.

---

### Edge Cases

- **Un dossier retenu dont l'organisation porteuse a été fusionnée** entre l'acceptation et la programmation : le registre des références de fusion déclare déjà les trois colonnes d'organisation de ce module ; les séances suivent la fiche survivante sans intervention.
- **Une séance dont la salle est supprimée** : la clé est « mise à nul », la séance revient donc d'elle-même au panneau « à placer » et cesse d'occuper le stand. Rien ne doit la faire disparaître.
- **Une séance dont la journée de rattachement est supprimée** : même régime. La séance garde son créneau ; c'est le rattachement qui disparaît.
- **Un fil de programmation supprimé** : le rattachement disparaît en cascade ; la séance reste.
- **Une édition sans aucune journée déclarée** — un cycle de webinaires : la journée de rattachement reste nulle, et la programmation publique se groupe alors sur la date du créneau.
- **Deux séances de la même édition dans la même salle physique** : elles remontent **une seule fois**, par le conflit de salle qui nomme la salle, et non deux fois comme avant la correction du 18/08.
- **Une séance diffusée sans canal** : elle échappe à la détection du conflit de diffusion. C'est pour cela que le modèle pose le canal d'office, et pour cela que le contrôle avant publication la signale en avertissement.
- **Une inscription qui arrive à la seconde où la jauge se remplit** : deux demandes concurrentes ne doivent pas dépasser la jauge ni attribuer deux fois la même position d'attente.
- **Une annulation et une inscription concurrentes** : la promotion depuis la liste d'attente ne doit pas faire dépasser la jauge.
- **Une personne anonymisée au titre du RGPD** qui était inscrite : l'inscription subsiste, les décomptes ne s'effondrent pas, et le nom rendu à la liste nominative est celui que l'anonymisation a posé.
- **Un formulaire dont un champ a été désactivé** après des inscriptions : les réponses déjà collectées restent lisibles, et le champ n'est plus proposé ni accepté.
- **Une réponse à un champ de type « pays »** : sa forme doit être fixée et validée, faute de quoi un export mêlant deux formes serait irrécupérable.
- **Une séance déplacée d'un jour à l'autre** : sa journée de rattachement doit suivre, sans quoi le calendrier et la programmation publique la rangent au mauvais jour.
- **Une annonce de publication reçue deux fois**, ou reçue après qu'une séance a été annulée entre-temps : le prédicat de l'annonce est appliqué tel quel, et rien de plus.

---

## Requirements *(mandatory)*

### Naissance d'une séance (écart n° 57)

- **FR-001**: À l'acceptation d'un dossier, le système DOIT créer ses séances **dans la même transaction** que la transition.
- **FR-002**: Il DOIT en créer autant que le dossier en demande d'occurrences, numérotées de 1 à N.
- **FR-003**: Chaque séance DOIT être créée à l'état « créneau pressenti », **sans salle**, **non publiée**.
- **FR-004**: Elle DOIT être datée du créneau souhaité par l'organisation, dans le fuseau de l'édition.
- **FR-005**: À défaut de créneau souhaité, elle DOIT être datée du **premier jour de l'édition** à l'heure d'ouverture quotidienne déclarée par l'appel, dans le fuseau de l'édition.
- **FR-006**: Sa fin DOIT être calculée depuis la durée du dossier, à défaut depuis la durée par défaut de l'appel, et DOIT toujours être strictement postérieure à son début.
- **FR-007**: Elle DOIT reprendre le titre, le résumé et le format du dossier, et porter une adresse d'URL **unique dans l'édition**, dérivée par le service.
- **FR-008**: Le système DOIT recopier les **intervenants** du dossier — personne, rôle, fonction et organisation déclarées, notice, confirmation, ordre.
- **FR-009**: Il DOIT recopier les **co-organisations** du dossier autres que le porteur ; la ligne du porteur est posée par le modèle et NE DOIT PAS être écrite par le service.
- **FR-010**: Il DOIT recopier les **thématiques** du dossier sur la séance, faute de quoi la programmation publique n'en afficherait aucune.
- **FR-011**: Une acceptation rejouée NE DOIT créer aucune séance supplémentaire : le système ne crée que les rangs qui manquent.
- **FR-012**: Le service NE DOIT émettre **aucun** événement de création ou de changement d'état de séance : le déclencheur du modèle les émet déjà.
- **FR-013**: Une action groupée retenant plusieurs dossiers DOIT créer les séances de chacun, en conservant la règle de compte rendu par dossier déjà posée par B4.
- **FR-014**: La création de séances NE DOIT PAS modifier le dossier, et la correction ultérieure d'un dossier NE DOIT modifier aucune séance.

### L'écran du planificateur

- **FR-015**: Le système DOIT rendre **en une réponse** l'édition, son fuseau, le libellé de sa ville, la date de publication de son programme, ses jours, ses salles, ses journées spéciales, ses canaux, ses séances placées, ses séances à placer et ses conflits.
- **FR-016**: Les conflits NE DOIVENT PAS faire l'objet d'un second appel pour composer cet écran.
- **FR-017**: Une séance sans salle DOIT figurer parmi les séances « à placer », et une séance avec salle parmi les « placées ».
- **FR-018**: Chaque séance rendue DOIT porter, déjà joints : le nom de sa salle, l'exclusivité de salle, la précision de lieu, le nom, le sigle et le code pays de son organisation porteuse, ses journées spéciales, ses thématiques avec libellé et couleur, son nombre d'intervenants, sa diffusion et son canal.
- **FR-019**: Une séance née d'un dossier DOIT porter le numéro du dossier, sa note consolidée, la durée souhaitée, le créneau souhaité et les contraintes de programmation déclarées au dépôt.
- **FR-020**: Cet écran DOIT être gardé par la permission de **planifier**, testée sur l'édition visée.
- **FR-021**: Il DOIT être borné par le périmètre d'administration, et les trois cas du périmètre DOIVENT rester distincts — global, éditions listées, **aucun droit → refus explicite**.
- **FR-022**: Un identifiant d'édition hors périmètre DOIT produire un refus **indiscernable** de celui d'un identifiant inexistant.
- **FR-023**: Le système DOIT exposer séparément la **liste des séances** d'une édition et ses **conflits**, gardées et bornées de la même façon.
- **FR-024**: Le système NE DOIT PAS servir de seconde route rendant le contrôle avant publication : elle existe depuis B3, sous un autre chemin.

### Placer, déplacer, redimensionner, retirer

- **FR-025**: Le système DOIT servir **une seule écriture** pour placer, déplacer, redimensionner et retirer une séance — salle, début et fin.
- **FR-026**: Cette écriture NE DOIT JAMAIS être refusée pour cause de chevauchement, quelle qu'en soit la gravité.
- **FR-027**: Une salle nulle DOIT renvoyer la séance au panneau, sans rien supprimer ni effacer son créneau souhaité.
- **FR-028**: Toute écriture de cet écran DOIT rendre la séance **et** les conflits recalculés **pour toute l'édition**.
- **FR-029**: Le contrat d'écriture NE DOIT PAS accepter l'**intervalle dérivé** ni l'**exclusivité de salle** : envoyés, ils DOIVENT produire un refus **nommant le champ**.
- **FR-030**: La **journée de rattachement** DOIT rester saisissable ; fournie, elle est retenue telle quelle.
- **FR-031**: Non fournie, elle DOIT être **déduite du créneau** dans le fuseau de l'édition — y compris lorsqu'une séance déjà rattachée change de jour.
- **FR-032**: Un créneau dont la fin n'est pas strictement postérieure au début DOIT être refusé sur son champ, en français.
- **FR-033**: Une salle désignée qui n'appartient pas à l'édition visée DOIT être refusée.
- **FR-034**: Toute écriture DOIT vérifier le périmètre d'administration **de l'édition de la séance**, résolue en base et jamais annoncée par le client.
- **FR-035**: Toute écriture DOIT positionner le contexte d'écriture — acteur et identifiant de requête — avant la première écriture, pour que l'historique de la séance reste lisible.

### Conflits

- **FR-036**: Le système DOIT rendre les conflits par la fonction du modèle, sans recomposer sa logique.
- **FR-037**: Il NE DOIT PAS filtrer ni requalifier les gravités qu'elle rend.
- **FR-038**: Il NE DOIT PAS empêcher une écriture au motif d'un conflit, ni offrir de moyen de le faire.
- **FR-039**: La lecture des conflits DOIT être gardée et bornée comme l'écran du planificateur.

### Journées spéciales

- **FR-040**: Le système DOIT accepter une liste de journées spéciales pour une séance, qui **remplace** la précédente.
- **FR-041**: Il DOIT laisser le modèle retenir qui a posé le rattachement et quand ; l'acteur DOIT donc être celui de la session appelante.
- **FR-042**: Il NE DOIT JAMAIS déduire un rattachement de la date d'une séance.
- **FR-043**: Le refus du modèle pour un fil d'une autre édition DOIT être **traduit** en message français portant un code stable, et non rendu brut.
- **FR-044**: Une liste envoyée deux fois DOIT laisser le même état, sans doublon.
- **FR-045**: Cette écriture DOIT rendre la séance et les conflits, comme les autres écritures de l'écran.

### Diffusion

- **FR-046**: Le système DOIT accepter d'activer ou de retirer la diffusion d'une séance, avec ou sans canal désigné.
- **FR-047**: Un canal désigné alors que la diffusion est activée DOIT être retenu tel quel.
- **FR-048**: Aucun canal désigné et diffusion activée : le canal par défaut posé par le modèle DOIT être laissé faire, et non recalculé par le service.
- **FR-049**: Une écriture retirant la diffusion **et** désignant un canal DOIT être refusée en **nommant le champ** — la base l'effacerait en silence.
- **FR-050**: Un canal inexistant, désactivé, ou n'appartenant ni à l'édition ni à la plateforme DOIT être refusé en le disant.
- **FR-051**: Deux directs simultanés DOIVENT rester **écrivables** et remonter en gravité bloquante.

### Publication

- **FR-052**: Le système DOIT consommer l'annonce de publication émise par le module Événements.
- **FR-053**: Il DOIT se garder du rejeu par le registre d'entrée du noyau, sur le couple consommateur / événement.
- **FR-054**: Il DOIT publier **exactement le prédicat porté par l'annonce** — édition, états retenus, séances non encore publiques — et aucun autre.
- **FR-055**: Il DOIT poser la **date de publication portée par l'annonce**, et non l'instant du traitement.
- **FR-056**: Il NE DOIT PAS écrire la date de publication de l'édition, déjà posée par l'émetteur.
- **FR-057**: Une seconde livraison NE DOIT publier aucune séance de plus.
- **FR-058**: Le système NE DOIT PAS dépublier une séance annulée : la vue publique porte déjà son état.
- **FR-059**: Un test de bout en bout DOIT comparer le nombre annoncé au nombre de séances effectivement publiées, et tout écart DOIT être **mesuré et consigné**.

### Programmation publique

- **FR-060**: Le système DOIT servir la programmation publique d'une édition **sans exiger de session**.
- **FR-061**: Il DOIT la lire par la vue du modèle, sans recomposer sa jointure ni recalculer son état temporel.
- **FR-062**: Il NE DOIT rendre que les séances publiées, la vue s'en chargeant.
- **FR-063**: Il DOIT servir le détail d'une séance publiée par son adresse d'URL dans son édition, avec ses intervenants et ses organisations.
- **FR-064**: Une adresse d'URL inconnue ou désignant une séance non publiée DOIT produire le **même** refus.
- **FR-065**: Une édition dont le programme n'est pas publié DOIT produire une réponse **vide**, non une erreur.

### Le formulaire d'inscription

- **FR-066**: Le système DOIT résoudre le formulaire **applicable** à une séance : celui de la séance, à défaut celui de son édition, à défaut celui de la plateforme.
- **FR-067**: Il NE DOIT rendre que les champs **actifs**, dans leur ordre déclaré.
- **FR-068**: Les options d'un champ adossé à une taxonomie DOIVENT être résolues **depuis la base**, avec leur libellé traduit ; elles NE DOIVENT JAMAIS être écrites dans l'API.
- **FR-069**: Cette lecture DOIT être publique : elle précède l'inscription, qui peut être ouverte sans compte.

### Les réponses, validées dynamiquement (écart n° 6)

- **FR-070**: Le système DOIT valider les réponses **contre le formulaire applicable**, et jamais contre une structure figée.
- **FR-071**: Il DOIT valider la **présence** de chaque réponse obligatoire, **y compris quand la séance ne porte pas de formulaire attaché** — cas où le modèle ne vérifie rien.
- **FR-072**: Il DOIT valider le **type** de chaque réponse selon le type du champ.
- **FR-073**: Il DOIT valider l'appartenance aux **options** d'un champ à choix, qu'elles soient explicites ou tirées d'une taxonomie.
- **FR-074**: Il DOIT appliquer les **règles de saisie** déclarées par le champ — minimum, maximum, longueur, motif.
- **FR-075**: Il DOIT **refuser** toute clé de réponse ne correspondant à aucun champ actif du formulaire applicable.
- **FR-076**: Chaque refus DOIT **nommer le champ** fautif et la règle enfreinte.
- **FR-077**: La forme de la réponse à un champ de type **pays** DOIT être fixée par le contrat et validée contre le référentiel des pays.
- **FR-078**: Le système NE DOIT PAS remplacer le document de réponses par une structure figée ni par une table annexe.

### Données sensibles

- **FR-079**: Une réponse à un champ marqué **sensible** DOIT exiger un **consentement explicite** accompagnant la demande.
- **FR-080**: Sans ce consentement, l'inscription DOIT être refusée en **nommant le ou les champs** concernés.
- **FR-081**: Le consentement DOIT être **conservé comme preuve**, avec sa finalité, sa version de politique, son origine et l'adresse d'appel.
- **FR-082**: Les réponses sensibles NE DOIVENT figurer dans **aucune** réponse servie à l'organisation qui anime la séance, ni dans la programmation publique.
- **FR-083**: Tout export **nominatif** livré par ce module NE DOIT PAS porter de réponse sensible ; ce jalon n'en livre aucun, et la règle est inscrite pour le jour où il en livrera un.

### Inscription, jauge, liste d'attente

- **FR-084**: Le système DOIT refuser une inscription à une séance **annulée**, à une séance **ne prenant pas d'inscription**, **avant** l'ouverture des inscriptions et **après** leur clôture — chacun avec son propre motif.
- **FR-085**: Il DOIT laisser le modèle basculer en **liste d'attente** quand la jauge est atteinte et qu'une liste est tenue, et la réponse DOIT porter l'état obtenu et la position.
- **FR-086**: Il DOIT traduire le refus de jauge sans liste d'attente en message portant le **nombre de places**.
- **FR-087**: Il DOIT refuser une seconde inscription vivante d'une même personne à une même séance, et **admettre** une réinscription après annulation.
- **FR-088**: L'annulation DOIT promouvoir depuis la liste d'attente **exactement** le nombre de places libérées, dans la même transaction, sans jamais dépasser la jauge.
- **FR-089**: Deux inscriptions concurrentes NE DOIVENT ni dépasser la jauge ni recevoir la même position d'attente.
- **FR-090**: Le service NE DOIT émettre **aucun** événement d'inscription : le déclencheur du modèle les émet déjà.
- **FR-091**: La langue de l'inscription DOIT être celle de la requête, retenue pour les envois ultérieurs.
- **FR-092**: L'origine technique de l'inscription DOIT être renseignée et distinguer une inscription faite par une personne d'une inscription posée par l'administration.

### Inscription sans compte

- **FR-093**: Le système DOIT admettre l'inscription sans compte **quand et seulement quand** le formulaire applicable l'autorise.
- **FR-094**: L'identité de la personne DOIT être prise dans des **champs dédiés** — adresse, prénom, nom — et **jamais** dans les réponses au formulaire.
- **FR-095**: La personne DOIT être retrouvée par son adresse, insensiblement à la casse, et créée **sans compte** si elle est inconnue.
- **FR-096**: Cette création NE DOIT poser ni compte, ni mot de passe, ni rôle, ni visibilité d'annuaire — la borne posée en B4 pour l'intervenant inconnu.
- **FR-097**: Un formulaire n'admettant pas l'inscription sans compte DOIT refuser une demande sans session en le disant.

### Présence et lectures d'inscription

- **FR-098**: Le système DOIT écrire l'instant de **première présence** une seule fois, sans jamais l'écraser.
- **FR-099**: Il DOIT servir à une personne connectée **ses** inscriptions, annulations comprises, et jamais celles d'un autre.
- **FR-100**: La liste **nominative** des inscrits d'une séance DOIT exiger la permission de **gérer les inscriptions**, testée sur l'édition de la séance.
- **FR-101**: Un identifiant de séance hors périmètre DOIT produire un refus indiscernable de celui d'une séance inexistante.

### Ce que voit l'organisation (écart n° 36)

- **FR-102**: Le suivi d'un dossier DOIT porter ses séances programmées, chacune avec sa salle.
- **FR-103**: Chaque séance DOIT porter **trois nombres** : inscriptions confirmées, liste d'attente, jauge.
- **FR-104**: Elle NE DOIT porter **aucun nom**, aucune adresse, aucune réponse au formulaire d'un inscrit — vérifié en balayant la charge utile entière, jamais champ par champ.
- **FR-105**: La liste des rappels DOIT rester **vide** jusqu'à B6, jamais absente.
- **FR-106**: L'espace de l'organisation DOIT produire l'action « **compte rendu manquant** » pour chaque séance terminée sans compte rendu.
- **FR-107**: Un dossier non retenu DOIT porter une liste de séances **vide**, jamais absente.

### Frontières, erreurs, traces

- **FR-108**: Le module NE DOIT dépendre d'aucun autre crate de module, ce qui DOIT rester **vérifiable mécaniquement**.
- **FR-109**: Il NE DOIT PAS appeler un autre module ; ses effets inter-modules passent par l'outbox transactionnel.
- **FR-110**: Toutes ses lectures hors de son schéma DOIVENT être **réunies au même endroit**, comme en B4, pour qu'un ajout se discute.
- **FR-111**: Toute erreur rendue DOIT porter un **code stable** et un **message français**, et une erreur de validation DOIT désigner le champ fautif.
- **FR-112**: Aucune erreur NE DOIT divulguer l'existence d'une donnée hors du périmètre de l'appelant.
- **FR-113**: Le module NE DOIT PAS réimplémenter un invariant déjà porté par la base : il traduit les refus.
- **FR-114**: Le module NE DOIT modifier aucun fichier du modèle de données.

### Key Entities

- **Séance** — une occurrence programmée : l'édition, le dossier d'origine s'il y en a un, le rang de l'occurrence, le titre, le créneau et son fuseau, la salle et la précision de lieu, l'état, le format, la diffusion et son canal, les réglages d'inscription, le compte rendu, la date de publication. Trois de ses valeurs sont **dérivées** et n'appartiennent pas au contrat d'écriture.
- **Intervenant de séance** — la personne annoncée pour le jour, avec sa fonction et son organisation telles qu'au moment de l'activité, et sa présence constatée après coup. Recopié du dossier, puis **indépendant** de lui.
- **Co-organisation de séance** — les organisations d'une séance publiée, porteur compris ; le porteur est posé par le modèle.
- **Rattachement à une journée spéciale** — un choix éditorial, avec son ordre d'affichage, sa mise en avant, et **qui l'a posé**.
- **Formulaire d'inscription** — un jeu de questions rattaché à une édition ou à la plateforme, ouvert ou non aux personnes sans compte.
- **Champ de formulaire** — une question : son code, son libellé, son type, son obligation, ses options, ses règles de saisie, son caractère sensible, son activité.
- **Inscription** — une personne, une séance, un état, ses **réponses en document libre**, sa langue, sa position d'attente, sa présence, son origine.
- **Conflit** — un chevauchement recensé : sa gravité, sa nature, la ressource en cause, les deux séances et l'intersection de leurs créneaux. Il n'est **jamais** un refus.

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Retenir un dossier demandant trois occurrences fait apparaître **trois** séances à placer, sans aucune saisie supplémentaire.
- **SC-002**: Retenir deux fois le même dossier laisse le **même** nombre de séances.
- **SC-003**: Une séance née d'un dossier porte autant d'intervenants, de co-organisations et de thématiques que lui.
- **SC-004**: L'écran du planificateur répond en **une** requête HTTP, conflits compris.
- **SC-005**: Poser une séance sur un créneau déjà occupé **aboutit**, et le conflit apparaît dans la même réponse.
- **SC-006**: Une séance en salle virtuelle ne produit **aucun** conflit de stand face à une séance physique simultanée.
- **SC-007**: Une séance sans salle ne produit **aucun** conflit.
- **SC-008**: Deux séances de la même édition dans la même salle remontent **une seule fois**.
- **SC-009**: Envoyer l'intervalle dérivé ou l'exclusivité de salle produit un refus **nommant le champ**, et jamais un succès silencieux.
- **SC-010**: Déplacer une séance du 12 au 14 novembre la rattache à la **journée du 14**.
- **SC-011**: Retirer la salle d'une séance la fait passer de « placées » à « à placer », sans perte.
- **SC-012**: Une liste de journées spéciales envoyée deux fois laisse le même état.
- **SC-013**: Rattacher une séance à un fil d'une autre édition produit un message français portant un code stable.
- **SC-014**: Retirer la diffusion efface le canal ; retirer la diffusion **en désignant un canal** est refusé sur ce champ.
- **SC-015**: Deux directs simultanés s'écrivent et remontent en gravité bloquante, y compris entre deux éditions.
- **SC-016**: Le nombre de séances annoncé par la publication et le nombre de séances devenues publiques sont **égaux**, et l'écart mesuré est consigné s'il ne l'est pas.
- **SC-017**: Rejouer l'annonce de publication publie **zéro** séance de plus.
- **SC-018**: La programmation publique d'une édition publiée répond **sans session** et ne porte que des séances publiées.
- **SC-019**: Chaque ligne de la programmation publique porte sa salle, son organisation, son pays, ses thématiques avec couleur, sa couverture et son état temporel — **sans requête supplémentaire**.
- **SC-020**: Omettre une réponse obligatoire, donner une valeur hors options, dépasser une borne ou employer une clé inconnue produisent **quatre** refus distincts, chacun nommant son champ.
- **SC-021**: Une réponse sensible sans consentement est refusée ; avec consentement, elle aboutit et le consentement est retrouvable.
- **SC-022**: Remplir la jauge d'une séance à liste d'attente fait basculer la demande suivante en attente, à la position suivante, sans trou.
- **SC-023**: Annuler une inscription confirmée promeut **exactement une** personne, et ne dépasse jamais la jauge.
- **SC-024**: Cent inscriptions concurrentes sur une séance de dix places produisent **dix** confirmées et **quatre-vingt-dix** en attente ou refusées, sans jamais dépasser dix.
- **SC-025**: Une personne sans compte s'inscrit quand le formulaire l'autorise, et se voit refusée quand il ne l'autorise pas.
- **SC-026**: Le suivi d'un dossier retenu porte trois nombres par séance et **aucun nom** — prouvé par un balayage de la charge utile entière.
- **SC-027**: La liste nominative des inscrits refuse un compte hors périmètre de la même façon qu'une séance inexistante.
- **SC-028**: Le contrôle des frontières de modules reste sans écart, et le graphe de dépendances du crate ne porte **aucune arête** vers un autre crate de module.
- **SC-029**: Aucun fichier de code de l’API ne dépasse mille lignes après ce jalon.

---

## Assumptions

- **Une réponse à un champ de type « pays » est le code ISO 3166-1 alpha-2** (écart n° 11, laissé ouvert le 16/08). C'est ce que les données simulées retiennent, c'est lisible dans un export, et cela reste stable si la fiche d'un pays est refaite. La forme est validée contre le référentiel des pays.
- **Les séances naissent dans la transaction d'acceptation**, et non par un travail différé consommant l'événement du dossier. Le planificateur doit avoir quelque chose à placer au moment où l'équipe regarde son écran, et un décalage de quelques secondes se lit comme une panne. Les deux voies sont écrivables ; celle-ci est retenue, et le plan la justifiera.
- **Une séance ne se crée pas à la main dans ce jalon.** Le modèle admet une séance sans dossier — l'IFDD programmant directement une activité —, mais aucun écran ne le demande et le prompt ne le nomme pas. La colonne reste facultative ; la route viendra le jour où l'écran existe.
- **Le compte rendu d'activité ne s'écrit pas dans ce jalon.** L'espace de l'organisation produit l'action « compte rendu manquant » parce que l'écran l'affiche déjà, mais aucun formulaire ne l'écrit — ni dans le front, ni dans le prompt.
- **Les questions du public ne sont pas livrées.** Trois tables du fichier `075` les portent, aucun écran ne les consomme, et le prompt ne les nomme pas.
- **L'annulation d'une séance et son report ne sont pas livrés** : aucun écran ne les demande. Les états existent dans le modèle et la programmation publique les rend déjà.
- **Les rappels appartiennent à B6** : la liste part vide, jamais absente, comme B4 l'a fait pour les séances.
- **Le libellé de la ville qui accompagne le fuseau** est celui que B3 rend déjà pour l'édition ; ce module ne le recompose pas.
- **Le module ne modifie aucun fichier de `docs/database/`.** Les écarts relevés ci-dessous se traitent dans le service ou se consignent.

---

## Dépendances et frontières

| Ce que ce module lit hors de son schéma | Pourquoi c'est admis |
|---|---|
| `event.events` — fuseau, ville, période, publication du programme | « à quelle heure ce créneau se lit-il, et à quelle date commence l'édition ? » — déjà autorisé en B4 |
| `event.event_days` | La journée à laquelle rattacher une séance |
| `event.rooms` | La salle d'une séance, et si elle occupe le stand |
| `event.programme_tracks` | Les journées spéciales offertes au rattachement |
| `event.broadcast_channels` | Le canal qu'occupe une séance diffusée |
| `event.calls_for_proposals` | La durée par défaut et l'heure d'ouverture du pavillon, à la naissance d'une séance — déjà autorisé en B4 |
| `org.organizations`, `reference.countries` | L'organisation porteuse et son pays, pour le planificateur |
| `identity.people` | Les intervenants, les inscrits nommés au back-office |
| `identity.has_permission()`, `identity.administered_events()` | L'autorisation et le périmètre, par le garde du noyau |
| `reference.taxonomy_terms`, `terms_of()`, `term_badges()`, `reference.locales` | Les thématiques, les options de formulaire adossées à une taxonomie, la langue d'une inscription |
| `media.attached_image()` par la vue publique | La couverture d'une séance, à défaut celle du dossier |

| Ce que ce module écrit hors de son schéma | Statut |
|---|---|
| `reference.entity_terms` — les thématiques d'une séance | **Dérogation déjà assumée et bornée** en B3 et B4, isolée dans un seul fichier du crate. La table est polymorphe et sans clé étrangère : aucun autre module ne peut poser les thématiques d'une séance |
| `identity.people` — l'inscrit sans compte | **Précédent établi** en B2 (l'invité d'une organisation) et en B4 (l'intervenant inconnu), avec la même borne : adresse, prénom, nom, et rien d'autre |
| `identity.consents` — la preuve du consentement à une réponse sensible | **À trancher au plan.** Le modèle prévoit l'origine « formulaire d'inscription » sur cette table, et aucun autre module ne peut poser ce consentement au moment où il est donné. Deux voies : la dérogation bornée, ou un contrat d'événement — la seconde interdirait de refuser l'inscription faute de consentement, puisque la preuve serait écrite après coup |

---

## Écarts relevés en écrivant la spécification de B5 (21/08)

Numérotation à la suite de B4, qui s'arrêtait à 110. **Aucune modification du modèle n'est proposée** : les points ci-dessous se traitent dans le service ou se consignent.

| N° | Écart | Où | Ce qu'il coûte | Suite donnée |
|---|---|---|---|---|
| **111** | **L'ÉCART N° 7 EST PARTIELLEMENT INEXACT, ET LE FRONT A RAISON.** Le canal de diffusion **est** saisissable : le déclencheur ne le pose que lorsqu'il est **nul** et que la diffusion est activée, et il n'écrase jamais un canal choisi. Le contrat du front le porte, et c'est juste — une édition à plusieurs canaux doit pouvoir en désigner un | `075` § 1, `tg_sessions_derive_fields()` | Refuser le canal comme l'écart le demandait aurait **cassé une fonctionnalité livrée** du planificateur : sur une édition à deux canaux, l'écran ne pourrait plus en choisir un | **Le contrat d'écriture refuse l'intervalle dérivé et l'exclusivité de salle** (FR-029) **et accepte le canal** (FR-047). Il refuse en revanche un canal désigné alors que la diffusion est **retirée** (FR-049) : là, et là seulement, la base efface en silence une valeur choisie — exactement ce que l'écart n° 7 veut empêcher |
| **112** | **L'EXCLUSIVITÉ DE SALLE N'EST PAS TOUJOURS RECALCULÉE.** Le déclencheur ne se déclenche que sur la salle, le début, l'édition, la diffusion et le canal : une écriture qui ne toucherait **que** l'exclusivité de salle passerait **sans être corrigée** | `075` § 1, `CREATE TRIGGER tg_sessions_derive` | La valeur resterait celle qu'un client a envoyée, et le calendrier colorerait un chevauchement matériel sur une salle virtuelle, ou l'inverse | **Le refus du champ (FR-029) est donc plus nécessaire encore que l'écart n° 7 ne le disait** : il ne protège pas d'un écrasement, il protège d'une valeur fausse qui **tiendrait** |
| **113** | **LA JOURNÉE DE RATTACHEMENT NE SE RECALCULE PAS QUAND ON DÉPLACE UNE SÉANCE.** Le déclencheur ne la déduit que lorsqu'elle est **nulle**. Une séance déjà rattachée qu'on déplace du 12 au 14 novembre **reste rattachée au 12** | `075` § 1 | Le calendrier du back-office et le regroupement par jour de la programmation publique rangeraient la séance au mauvais jour, sans qu'aucune erreur ne le signale — et c'est le geste le plus fréquent de tout l'écran | **Le service met la journée à nul quand le créneau change et qu'aucune journée n'est fournie** (FR-031), pour que le déclencheur la redéduise. Aucune modification du SQL : la déduction reste là où elle est |
| **114** | **LE DÉCLENCHEUR D'INSCRIPTION NE VALIDE QUE CONTRE LE FORMULAIRE ATTACHÉ À LA SÉANCE.** Il ne fait rien lorsque la séance n'en porte pas — alors que le formulaire **applicable** peut être celui de l'édition ou celui de la plateforme | `075` § 4, `tg_validate_registration()` | Une séance sans formulaire attaché accepterait une inscription **sans aucune réponse obligatoire**, alors que l'écran en aura posé quatre. La garantie de la base est un **plancher**, pas la règle entière | **La validation du service porte sur le formulaire résolu** (FR-066, FR-071). Le déclencheur reste le dernier filet, il n'est pas le premier |
| **115** | **LE DÉCLENCHEUR IGNORE L'OUVERTURE DES INSCRIPTIONS ET L'ABSENCE D'INSCRIPTION.** Il vérifie la clôture, l'annulation et la jauge — jamais la date d'**ouverture**, jamais le fait qu'une séance **ne prenne pas** d'inscription | `075` § 1 et § 4 | On pourrait s'inscrire trois mois avant l'ouverture annoncée, et à une activité qui ne demande aucune inscription | **Refusé par le service, chacun avec son motif** (FR-084) |
| **116** | **LA PROMOTION DEPUIS LA LISTE D'ATTENTE NE VÉRIFIE PAS LA JAUGE.** Le contrôle de capacité du déclencheur ne porte que sur l'**insertion** ; une promotion est une mise à jour | `075` § 4, `promote_from_waitlist()` | Promouvoir plus de personnes que de places libérées ferait dépasser la jauge **sans un mot**, et personne ne le verrait avant le jour de l'activité | **Le service promeut exactement le nombre de places libérées** (FR-088), dans la transaction de l'annulation |
| **117** | **LES DEUX DÉCLENCHEURS DU FICHIER ÉMETTENT DÉJÀ.** La séance émet à la création, à chaque changement d'état et à chaque report ; l'inscription émet à la création et à chaque changement d'état. C'est le piège de B1, B2 et B4, à l'identique | `075` § 1 et § 4 | Un service qui émettrait à son tour produirait **deux** événements par séance créée et par inscription — donc deux courriels, deux rappels planifiés —, et le doublon ne se verrait qu'en production | **Inscrit à la spécification** (FR-012, FR-090) : le service n'émet aucun de ces événements. Vérifié en lisant le corps des deux déclencheurs |
| **118** | **UN CHANGEMENT DE SALLE N'ÉMET RIEN.** Le déclencheur d'événements ne se déclenche que sur l'état, le début et la fin, et son corps sort immédiatement si l'état et le début n'ont pas changé | `075` § 1, `tg_sessions_emit_events()` | Une séance déplacée du stand vers une salle virtuelle — ou l'inverse — ne prévient **aucun** inscrit, alors que c'est un changement de lieu | **Consigné, non corrigé.** Le corriger demanderait de modifier le SQL, et rien du jalon en cours ne le consomme : les rappels appartiennent à B6, qui décidera s'il lui faut cet événement |
| **119** | **LE RÔLE DE PROGRAMMATION NE PEUT PAS VOIR LES INSCRITS.** Il détient la permission de planifier, pas celle de gérer les inscriptions | `030` § 3 | Une chargée de programmation compose la grille mais ne peut pas savoir combien de personnes viendront, ni ouvrir la liste. Elle devra passer par un compte d'administration | **Le modèle est suivi.** C'est une attribution de permission, pas une règle d'API : elle se corrige par le SQL ou par le back-office des rôles. Consigné à côté de l'écart n° 56, qui pose la même question sur la publication |
| **120** | **LA SÉANCE PERD LE LIEN VERS L'ORGANISATION DE SES INTERVENANTS.** Le dossier porte, pour chaque intervenant, une organisation **liée** en plus du libellé figé ; la séance ne porte que le libellé | `070` § 6, `075` § 2 | Un intervenant d'une séance ne peut plus être rattaché à une fiche d'organisation : le comptage par organisation d'un programme se fait sur les co-organisations, jamais sur les intervenants | **Consigné, non corrigé.** Le libellé figé est ce dont l'archive a besoin — « l'archive de la COP28 ne doit pas être réécrite » —, et aucun écran ne demande le lien |
| **121** | **DEUX CHEMINS DU FRONT DÉSIGNENT LA MÊME LECTURE.** Le contrat du front déclare un contrôle avant publication sous les séances, alors que B3 l'a livré sous le planificateur, et aucun écran n'appelle le premier | `composables/useApi.ts`, `crates/modules/event/src/routes/planner.rs` | Livrer les deux donnerait **deux chemins pour une même réponse**, appartenant à deux modules : ils divergeraient au premier changement | **Un seul est servi**, celui de B3 (FR-024). Le second est à retirer du front au raccordement (B7) |
| **122** | **AUCUN ÉCRAN N'ÉCRIT LE COMPTE RENDU D'UNE SÉANCE**, alors que l'espace de l'organisation produit l'action « compte rendu manquant » et affiche son bandeau | `075` § 1, `frontend/app/components/workspace/` | L'action mène à un écran qui constate le manque sans offrir de le combler | **L'action est servie** (FR-106), l'écriture n'appartient pas à ce jalon. À rouvrir avec l'écran du bilan d'activité, où le rôle média `gallery` attend déjà |
| **123** | **QUE DEVIENT LA SÉANCE D'UN DOSSIER ANNULÉ APRÈS ACCEPTATION ?** Le seul chemin qui sort de l'état « retenu » est l'annulation, avec motif. Rien ne dit ce qu'il advient de la séance, de ses inscrits et de sa place au programme | `070` § 1, `075` § 1 | Une activité annulée qui reste au programme public envoie des gens devant une salle vide ; une séance annulée d'office prive l'IFDD de la possibilité de la reprogrammer | **Question au commanditaire, inscrite aux points bloqués.** L'option tenue provisoirement est la première : l'annulation d'un dossier retenu **annule ses séances** avec le même motif, ce qui prévient les inscrits par le déclencheur existant |

---

## Points à arbitrer avec le commanditaire

Une seule question naît de cette spécification ; elle est posée en mots simples aux points bloqués, avec ses options et leur coût.

**Quand l'IFDD annule une activité qu'elle avait retenue, faut-il retirer d'office la séance du programme et prévenir les inscrits, ou laisser l'équipe décider séance par séance ?**

- **A. La séance est annulée d'office, avec le même motif.** Les personnes inscrites sont prévenues par le mécanisme existant, et l'activité s'affiche « annulée » au programme public plutôt que d'y rester comme si de rien n'était. En revanche, si l'IFDD comptait la reprogrammer plus tard, il faudra la recréer.
- **B. Rien ne bouge ; l'équipe annule la séance à la main.** Rien à développer, mais tant que personne n'y pense, l'activité reste annoncée, avec ses inscrits et ses rappels.

**Recommandation : A.** Le symptôme de B est le silence — une salle vide un matin de COP —, celui de A se voit tout de suite et se rattrape d'un geste.
