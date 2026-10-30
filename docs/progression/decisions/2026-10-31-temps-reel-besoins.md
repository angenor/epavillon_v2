# Décisions : 2026-10-31, le temps réel, ce qui en vaut la peine

> Extrait de la [progression](../../PROGRESSION.md). Ce qui a été tranché en cours de route, et pourquoi.

**Statut : en construction.** Arbitré le 31/10 (§ 7) ; le socle est écrit, voir l'étude du mécanisme § 5. Complète l'étude du mécanisme, [2026-10-31-temps-reel.md](2026-10-31-temps-reel.md), qui ne traitait que les deux premiers usages demandés.

**La demande (30/10).** « Évalue toutes les données qui, si elles sont en temps réel, augmentent considérablement l'expérience utilisateur, et nous allons voir ce qu'on retient pour construire un socle solide. »

---

## En bref

- Tout le site, le back-office et Guide Négo ont été parcourus : une quarantaine d'écrans. **Huit usages comptent vraiment pour la COP31**, neuf seront utiles ensuite, le reste ne vaut pas une connexion ouverte.
- Le plus gros gain **le jour J** n'est pas où on l'attendait : c'est le **direct**, les **messages d'incident** et les **changements de salle des négociations**, devant un public qui garde la page ouverte des heures.
- **Le repère « en direct » ne s'allume aujourd'hui jamais**, rechargement ou pas : le démarrage de la réunion Zoom n'est pas reporté sur l'activité. À corriger d'abord, sinon il n'y a rien à mettre à jour.
- Le public anonyme est trop nombreux pour garder chacun une ligne ouverte sur le serveur partagé. Proposition : **sa page vérifie toutes les 30 secondes si quelque chose a changé**, à un coût presque nul. Les personnes connectées reçoivent l'information à l'instant.
- **Le calendrier ne permet pas tout avant le 8 novembre** : environ douze jours de travail pour cinq disponibles. Un ordre est proposé au § 5.

---

## 1. Ce que veut dire « temps réel » ici

Une page ouverte apprend qu'une chose a changé et se met à jour seule, sans que la personne recharge. Deux manières d'y arriver, que la personne ne distingue pas :

- **À l'instant** : la page garde une ligne ouverte avec le serveur, qui la prévient dès qu'un changement la concerne. C'est ce que décrit l'étude. Chaque page ouverte occupe une place sur le serveur web, partagé avec le site Finance durable, et ces places sont comptées.
- **Toutes les 30 secondes** : la page demande « quelque chose a changé ? » ; la réponse est presque toujours « non », en quelques octets. Aucune place n'est occupée entre deux questions.

La règle proposée : **à l'instant pour les personnes connectées** (quelques centaines au plus), **toutes les 30 secondes pour le public anonyme** (peut-être deux mille pages ouvertes pendant un direct très suivi). Pour le public, 30 secondes de retard sur un incident ou un direct qui démarre ne se remarquent pas ; une ligne ouverte par visiteur, si.

---

## 2. Indispensable pour la COP31 (8 au 20 novembre)

### 2.1 Les échanges d'un dossier, des deux côtés (déjà demandé)

- **Pour qui** : l'équipe IFDD et les organisations dont l'activité est retenue.
- **Aujourd'hui** : côté organisation, les messages arrivent dans les 15 secondes quand la fenêtre est ouverte. Côté IFDD, **rien ne bouge** : la réponse d'une organisation n'apparaît qu'au rechargement. Quand le comité accepte un dossier ou demande une correction, le message arrive chez l'organisation mais **l'étiquette d'état du dossier reste l'ancienne**.
- **En temps réel** : le message apparaît sur la fiche ouverte des deux côtés, le compteur de non-lus et les pastilles des onglets bougent, l'état du dossier change sous les yeux de l'organisation.
- **Gain : fort.** C'est la conversation qui prépare le jour J (point focal, besoins techniques, présentations), et une conversation où l'un des deux doit recharger n'en est pas une.
- **Quand** : maintenant, la semaine qui précède la COP, et pendant.
- **Coût** : un jour et demi (étude, étape 2), l'état du dossier et les pastilles compris.

### 2.2 Les envois de courriels (déjà demandé)

- **Pour qui** : l'équipe IFDD qui envoie convocations, rappels et consignes.
- **Aujourd'hui** : le compteur « envoyés » ne bouge qu'en cliquant « Actualiser ». Si le serveur de courriel tombe une minute, l'envoi reste « en cours » pour toujours, sans que rien ne le dise.
- **En temps réel** : le compteur monte pendant l'envoi ; une notification brève dit « parti » ou « problème », même si la personne est passée sur une autre page.
- **Gain : fort**, surtout pour le « problème » : un envoi bloqué la veille d'une activité, c'est une salle vide.
- **Quand** : la semaine qui précède et chaque jour de la COP.
- **Coût** : un jour et demi à deux jours (étude, étape 3), dont la représentation du « problème », qui manque aujourd'hui.

### 2.3 Le planificateur, à plusieurs

- **Pour qui** : les deux à quatre personnes de l'IFDD qui placent et déplacent les activités.
- **Aujourd'hui** : chacune ne voit que ses propres déplacements. Ceux d'un collègue restent invisibles jusqu'au rechargement : on peut poser une activité sur un créneau qui vient d'être pris, et la liste des conflits à l'écran est celle d'il y a une heure. Si deux personnes modifient la même activité, la dernière efface l'autre sans le savoir.
- **En temps réel** : le déplacement d'un collègue apparaît sur la grille, les conflits et le compte de ce qui reste à faire avant publication se mettent à jour.
- **Gain : fort pendant la COP**, où le programme bouge tous les jours (reports, annulations, intervenant en retard). Moyen avant.
- **À savoir** : le temps réel montre le travail des autres, il n'empêche pas d'écraser une modification faite dans la même seconde. Une protection simple s'y ajoute (« cette activité vient d'être modifiée par un collègue, voici sa version ») : un jour de plus, recommandé.
- **Coût** : un jour, plus un jour pour la protection.

### 2.4 Le direct sur les pages publiques

- **Pour qui** : le public (programmation, accueil, page d'une journée, page d'une activité).
- **Aujourd'hui** : le bandeau « en direct maintenant » n'apparaît jamais. La page affiche l'état connu à son ouverture ; et de toute façon, le démarrage de la réunion Zoom n'est pas reporté sur l'activité, donc **même un rechargement ne l'allume pas**.
- **En temps réel** : quand l'hôte lance la réunion, le bandeau « en direct » apparaît dans les 30 secondes sur toutes les pages publiques ouvertes, avec le bouton pour rejoindre ; il s'éteint à la fin.
- **Gain : fort le jour J.** C'est la première chose qu'un visiteur cherche pendant la COP, et une personne qui garde la programmation ouverte en fond ne recharge pas.
- **Préalable** : reporter le démarrage et la fin de la réunion sur l'activité (une demi-journée à un jour), indépendamment du temps réel.
- **Coût** : compris dans le « pouls de l'édition » (§ 4), une demi-journée pour brancher les pages.

### 2.5 Les messages d'incident

- **Pour qui** : le public.
- **Aujourd'hui** : un message (« l'activité de 14 h commence avec 20 minutes de retard », « le direct est interrompu ») ne s'affiche que sur la page de programmation, **lu une seule fois à l'ouverture**. Il n'apparaît ni sur la page de l'activité, ni sur l'accueil.
- **En temps réel** : le message apparaît, change ou disparaît sur les pages ouvertes dans les 30 secondes.
- **Gain : fort le jour J.** Un message d'incident est écrit pour les gens qui sont déjà là ; s'il ne touche que ceux qui arrivent ensuite, il manque sa cible.
- **Préalable** : l'afficher aussi sur la page de l'activité (une demi-journée).
- **Coût** : compris dans le pouls de l'édition.

### 2.6 Les changements du programme public

- **Pour qui** : le public.
- **Aujourd'hui** : une activité annulée, reportée ou déplacée garde son ancien horaire sur la page ouverte.
- **En temps réel** : la carte change dans les 30 secondes (« Reportée », nouvel horaire).
- **Gain : moyen.** Le changement est rare, mais la personne qui se déplace au pavillon sur la foi d'un horaire périmé le paie cher.
- **Coût** : nul une fois le pouls de l'édition en place, même signal.

### 2.7 Guide Négo : les sessions de négociation sur un écran ouvert

- **Pour qui** : les négociatrices et négociateurs, pendant les deux semaines.
- **Aujourd'hui** : un écran ouvert (la journée, une session, l'agenda) **ne se relit jamais** ; il faut changer d'écran. Il ne se relit même pas quand on rallume le téléphone. Une salle changée ou une session annulée, lue à la source officielle ou validée par un relais, n'apparaît qu'à la navigation suivante ; le bandeau de rappel garde l'ancienne salle.
- **En temps réel** : « Salle changée », avec l'ancienne salle barrée, apparaît sur l'écran ouvert et le rappel suit la nouvelle salle.
- **Gain : fort.** C'est la promesse même de l'application, et le moment où elle compte est le couloir, dix minutes avant la séance.
- **À savoir** : la lecture de la source officielle des Nations unies est encore bloquée par leur protection contre les robots ; tant qu'elle l'est, les changements viennent des signalements validés et des réunions saisies à la main. Et le temps réel ne sert que l'application ouverte : faire sonner un téléphone en poche, c'est la notification du téléphone, prévue avec la version des magasins d'applications.
- **Coût** : un jour. S'y ajoute, sans attendre le socle, la relecture au retour sur l'application (un quart de journée), qui couvre déjà l'essentiel de l'usage réel : on sort le téléphone, on regarde, on le range.

### 2.8 Guide Négo : la cloche

- **Pour qui** : les négociatrices et négociateurs.
- **Aujourd'hui** : le nombre de notifications non lues n'est relu qu'en revenant à l'accueil ou sur la page des notifications.
- **En temps réel** : la cloche s'allume quand une session suivie change, qu'une place se libère à une réunion de la Francophonie, qu'un expert répond.
- **Gain : moyen**, fort par ricochet : c'est elle qui avertit d'un changement quand on regarde autre chose que la session concernée.
- **Coût** : une demi-journée, le canal personnel servant déjà aux envois de courriels.

---

## 3. Utile ensuite

| Usage | Pour qui | Pourquoi pas pour la COP31 |
|---|---|---|
| Liste des propositions et fiche d'évaluation : nouveaux dossiers, attributions, notes des pairs, décision prise par un collègue | équipe IFDD | l'évaluation de la COP31 s'achève ; utile au prochain appel |
| File de validation des signalements de Guide Négo | relais | déjà relue toutes les 30 secondes, ce qui suffit pour quelques relais |
| Files des experts et demandes d'accès à Guide Négo | équipe IFDD, experts | peu de volume ; deux personnes qui traitent la même demande sont refusées proprement |
| Réunions de la Francophonie et programme du pavillon dans Guide Négo, écran ouvert | négociateurs | les changements arrivent déjà par la cloche (§ 2.8) |
| Réponse d'un expert, sort de mes signalements | négociateurs | idem, par la cloche ; aucune urgence |
| Demande de rattachement à une organisation et son acceptation | organisations | rare ; la personne acceptée le voit à la navigation suivante |
| Questions du public pendant une activité | public, modérateur | la réunion Zoom s'ouvre en plein écran et a ses propres questions ; à revoir si le modérateur les lit en séance |
| Poste de direct : nombre de personnes dans la salle Zoom, échecs d'inscription à Zoom | équipe technique | la donnée n'est pas encore calculée ; à construire avant de la rendre vivante |
| Échanges de Guide Négo (messagerie entre négociateurs) | négociateurs | pas encore construits ; ce sera le vrai test de volume, et peut-être le moment d'une ligne dans les deux sens |

---

## 4. Pas la peine

- **Places restantes et liste d'attente** : en 2026, aucune limite de places hors celle de Zoom, et les personnes en liste d'attente reçoivent le lien comme les autres ; il n'y a rien à suivre.
- **Ouverture de la salle Zoom** : la salle est joignable à tout moment et Zoom fait patienter jusqu'à l'hôte ; la relecture actuelle (une à deux minutes) suffit.
- **Tableau de bord du back-office** : les chiffres sont recalculés tous les quarts d'heure ; les relire à l'ouverture suffit.
- **Traitement d'une image** : quelques secondes, pour une personne qui attend devant ; une relecture courte suffit.
- **Extraction d'un document de Guide Négo** : déjà relue régulièrement pendant le traitement.
- **Journal de lecture de la source officielle** : une ou deux personnes, déjà relu après « Lire maintenant ».
- **Doublons et fusion d'organisations** : rare, et une fusion déjà faite par un collègue est refusée proprement.
- **Formulaires d'administration** (événements, journées spéciales, vitrine, gabarits, utilisateurs, lexique, foire aux questions, codes d'invitation) : une personne à la fois, rien à suivre.
- **Interrupteurs des espaces et page de maintenance** : rares ; la navigation suivante suffit.
- **Accueil hors direct** (bandeau, éditions, témoignages) : change quelques fois par an.

---

## 5. Ce que le socle doit savoir faire

### 5.1 Pour couvrir les huit usages sans retouche

1. **Prévenir une personne** (ses envois, sa cloche), **les deux côtés d'un dossier** sans jamais dire à l'organisation qu'on parle d'elle en interne, **toute l'équipe d'une édition** (planificateur) en respectant la règle « un administrateur peut n'avoir accès qu'à un événement », **tous les négociateurs** d'une COP d'un seul geste, et **le public d'une édition**, sans compte.
2. **Servir le public anonyme sans ligne ouverte** : la vérification toutes les 30 secondes décrite au § 1. Les pages ne savent pas laquelle des deux manières elles reçoivent : si la mesure du serveur le permet un jour, le public passe à l'instant sans toucher une page.
3. **Encaisser l'arrivée de tous à la fois** : quand une salle change, trois cents téléphones relisent l'agenda. Chacun attend un instant au hasard (zéro à cinq secondes) avant de relire, pour ne pas arriver ensemble.
4. **Vivre sur un téléphone** : la ligne se ferme dès que l'application passe en arrière-plan et l'écran se relit au retour. Rien ne se perd : chaque retour relit tout.
5. **Recevoir l'annonce de travaux qui tournent en coulisse** autant que des gestes d'une personne : la plupart des changements du jour J naissent hors de tout clic (une réunion Zoom qui démarre, la source officielle relue toutes les cinq minutes, un signalement publié trente secondes après sa validation, un lot de courriels parti).

### 5.2 Volume au pic de la COP (ordres de grandeur, à confirmer)

- **Lignes ouvertes** (personnes connectées) : équipe IFDD 20 à 60, points focaux des organisations quelques dizaines, négociateurs 100 à 300 téléphones ouverts le matin des séances. **Soit 200 à 400 lignes**, au lieu des « quelques dizaines plus quelques centaines de déposants » de l'étude : Guide Négo en apporte la plus grosse part. Le serveur web doit pouvoir les tenir **en plus** de ses visiteurs ordinaires et de ceux du site voisin : c'est la mesure à faire avant tout.
- **Public** : 500 à 2 000 pages ouvertes pendant un direct très suivi, soit 15 à 70 petites questions par seconde, servies de mémoire par l'API, sans toucher à la base.
- **Annonces** : quelques-unes par minute, des rafales pendant un envoi de courriels, regroupées par demi-seconde. Négligeable pour la base.

Ces chiffres sont des estimations : la fréquentation de la COP30 dans la v1, et celle mesurée par la v2 depuis le 26/10, les remplaceraient avantageusement.

### 5.3 Ce qui change par rapport à l'étude

- **Deux portées de plus** : toute l'équipe d'une édition (planificateur) et tous les négociateurs d'une COP. L'étude ne visait qu'une personne, un dossier, une organisation, un envoi.
- **Le public n'est plus laissé de côté** : il a sa propre manière, la vérification régulière, derrière la même interface que les connectés. Elle remplace aussi le repli prévu par l'étude (chaque page relisant sa propre donnée chaque minute) : une seule petite question pour tous les sujets d'une page.
- **Un délai au hasard avant de relire** pour les annonces qui touchent des centaines de personnes ; l'étude ne regroupait que côté serveur.
- **Le téléphone** : fermeture immédiate en arrière-plan, pas après 30 secondes.
- **Trois préalables hors temps réel**, sans lesquels il n'y a rien à rendre vivant : le direct reporté sur l'activité, l'incident affiché sur la page de l'activité, la protection du planificateur contre l'écrasement. S'y ajoute celui de l'étude, l'envoi de courriels en « problème ».

### 5.4 Ce qui tient avant le 8 novembre

Il reste cinq jours ouvrés. Les huit usages, préalables et socle compris, en demandent environ douze. Ordre proposé, du plus utile le jour J au moins urgent :

| Ordre | Contenu | Jours |
|---|---|---|
| 1 | Préalables du jour J : direct reporté sur l'activité, incident sur la page de l'activité ; Guide Négo relu au retour sur l'application | 1,5 |
| 2 | Socle, première manière : la vérification régulière, pour le public (30 s) **et provisoirement pour les connectés** (15 s) ; notification brève | 2 |
| 3 | Pages publiques branchées (direct, incidents, programme) ; échanges des dossiers ; envois de courriels | 2,5 |
| 4 | Pendant la COP, si le serveur le permet : la ligne ouverte pour les connectés, sans retoucher les pages ; planificateur et sa protection ; sessions et cloche de Guide Négo | 5 à 6 |

La vérification régulière d'abord parce qu'elle ne demande **rien au serveur partagé** et sert de toute façon au public et de secours aux connectés : rien n'est jeté quand la ligne ouverte arrive.

---

## 6. Questions au commanditaire

Une à la fois, dans cet ordre. Elles complètent celles de l'étude (§ 4), dont la sixième (toucher au réglage du serveur partagé) reste la condition de la ligne ouverte.

**Question 1. Pour le public, 30 secondes suffisent-elles ?** Quand un direct démarre ou qu'un message d'incident est publié, un visiteur qui a la page ouverte le voit apparaître.
- A. Dans les 30 secondes. Aucun effet sur le serveur partagé, prêt pour la COP.
- B. À l'instant. Chaque visiteur occupe une place sur le serveur tant que sa page est ouverte ; pendant un direct suivi par deux mille personnes, le site Finance durable peut ralentir. Seulement après mesure du serveur, donc pas sûr pour la COP.
- *Recommandation : A*, la différence ne se voit pas et le risque disparaît.

**Question 2. Pour la COP, accepte-t-on 15 secondes aussi pour l'équipe et les organisations ?** Un message d'une organisation, la fin d'un envoi de courriels ou le déplacement d'un collègue dans le planificateur apparaîtraient en 15 secondes au plus au lieu d'aussitôt.
- A. Oui pour la COP : prêt plus tôt, sans toucher au serveur partagé. L'instantané vient ensuite, sans rien changer aux écrans.
- B. Non, l'instantané dès la COP : il faut d'abord mesurer et régler le serveur partagé (accès requis), et l'échange des dossiers et les courriels passent après le direct et les incidents dans le calendrier.
- *Recommandation : A*, c'est déjà ce qu'ont les organisations aujourd'hui, et l'équipe gagne beaucoup à passer de « jamais » à 15 secondes.

**Question 3. Le planificateur doit-il empêcher d'écraser le travail d'un collègue ?**
- A. Oui : si deux personnes déplacent la même activité presque en même temps, la seconde voit « modifiée à l'instant par un collègue » et la version à jour avant de confirmer. Un jour de travail.
- B. Non : chacun voit les déplacements des autres, mais deux modifications simultanées de la même activité restent possibles, la dernière gagne.
- *Recommandation : A* pendant la COP, où l'on corrige le programme vite et à plusieurs.

---

## 7. Décisions du commanditaire (31/10)

- **Retenus pour la COP31** : 2.1 échanges d'un dossier, 2.2 envois de courriels, 2.3 planificateur, 2.4 direct sur les pages publiques, 2.5 messages d'incident, 2.6 changements du programme public ; s'y ajoutent, du groupe « utile ensuite », les questions du public pendant une activité et le poste de direct (présents dans la salle Zoom, échecs d'inscription). Guide Négo (2.7, 2.8) n'est pas retenu à ce stade.
- **Question 1 : A.** Le public suit par le pouls, 30 s au plus.
- **Question 2 : B.** Instantané dès la COP pour l'équipe et les organisations connectées : ligne ouverte, repli sur le pouls. Mesure du serveur le 31/10 : Apache MPM `event`, `MaxRequestWorkers 1000`, HTTP/2 non chargé ; marge suffisante pour 200 à 400 lignes.
- **Question 3 : B.** Pas de protection contre l'écrasement dans le planificateur : une seule personne corrige le programme, en général.

## Annexe technique

**Sujets nécessaires au premier groupe.** « Signal » au sens de l'étude (§ 2.2) : le navigateur relit la route REST qu'il appelle déjà.

| Usage | Sujet | Autorisation à l'abonnement | Émis par |
|---|---|---|---|
| 2.1 Échanges, état du dossier | `proposal:<id>` (`audience` comité ou déposant), `organization:<id>` | `perimeter::acces_au_dossier` ; appartenance | API : écriture, résolution d'un message, transition d'état (trigger de `programme.proposals`) |
| 2.2 Courriels | `mailing:<id>`, `event:<id>:mailings`, `person:<id>` | `engagement.mailing.send` sur `Scope::Event` ; soi | worker : `send_mailing` par lot, clôture, échec |
| 2.3 Planificateur | `event:<id>:planner` | même garde que les routes du planificateur (`garder`, périmètre de l'édition, règle 8) | API : `PUT …/schedule`, placement dans une journée, publication ; worker : `PublicationConsumer` |
| 2.4 à 2.6 Direct, incidents, programme public | `edition:<id>:public` (sous-sujets `live`, `incidents`, `schedule`) | aucune, **anonyme**, servi par le pouls | worker : `live.process_webhook_event` (`meeting.started`, `ended`) ; API : `live.publish_incident()`, `unpublish` ; trigger des statuts de `programme.sessions` (`rescheduled`, `cancelled`, `postponed`) |
| 2.7 Sessions Guide Négo | `negotiation:<edition>:sessions` | accès Guide Négo de la personne (`mon_acces`) | worker : `import_official_sessions` quand `change_count > 0`, `report.publish` ; API : réunions saisies à la main |
| 2.8 Cloche | `person:<id>` (`kind: notifications`) | soi | worker : `NotificationsConsumer`, à l'insertion dans `engagement.notifications` |

**Le pouls.** `GET /api/realtime/pulse?topics=…` rend `{ boot, versions: { sujet: n } }`. Les versions sont un compteur en mémoire dans le concentrateur de l'API, alimenté par la même écoute `LISTEN platform_signal` que le flux : pas de table, pas de Valkey. Un `boot` différent (API redémarrée) vaut `resync`. Sujets publics sans session ; sujets privés contrôlés à chaque appel par le même `TopicAuthorizer` que le flux, avec un cache de 60 s par session (la revalidation de l'étude). Réponse `Cache-Control: no-cache` avec ETag : 304 dans le cas courant. `useRealtime()` choisit le transport par sujet (pouls si public ou si le flux est indisponible), les pages n'en savent rien.

**Diffusion large.** Pour `negotiation:<edition>:sessions` et `edition:<id>:public`, le client tire un délai aléatoire de 0 à 5 s avant de relire. La route des sessions de Guide Négo porte déjà un ETag (`composables/api/etiquete.ts`) ; celle de la programmation publique gagnerait à en porter un.

**Préalables constatés.**
- Le repère « en direct » lit `programme.sessions.status = 'live'` (`components/event/Programme.vue`, `home/AsidePanel.vue`) ; **aucune route ni aucun job n'écrit ce statut**. Le webhook `meeting.started` ne met à jour que `live.meetings` (`live/src/jobs/webhook.rs`), et `live.streams` n'est écrit par personne (pas de YouTube en 2026). À trancher : passer la séance à `live` au démarrage de la réunion et la sortir à la fin, ou faire lire au site l'état de la réunion. SQL d'abord.
- `UiIncidentBanner` n'est monté que dans `pages/programme.vue`.
- Planificateur sans contrôle de version (`api.planner.schedule`, aucune colonne de version ni `If-Match`) ; le motif existe dans Guide Négo (`kernel/src/empreinte.rs`, 412).
- Guide Négo ne relit pas un écran au retour au premier plan (seuls l'accès et la session le font, `useGnAcces.ts`, `useGnSession.ts`).
- Envoi de courriels sans état « problème » (étude, § 1).

**Sondages existants**, à rebrancher sur `useRealtime()` puis à garder en repli : `usePolling` des échanges (`pages/organization/proposal/[id].vue`), la réunion Zoom toutes les 60 à 120 s (`composables/useMeetingRoom.ts`, à garder tel quel), la file des signalements toutes les 30 s (`pages/guide-nego/validation/signalements.vue`), l'extraction d'un document (`pages/admin/negociations/documents/[id]/index.vue`, à garder). Les autres minuteries ne font qu'avancer l'heure affichée.

**Événements d'outbox et signal.** Les faits qui donnent un signal existent presque tous dans l'outbox (`programme.session.*`, `live.incident.*`, `negotiation.meeting.changed`, `report.published`…), mais le relais n'a pas à porter l'écran (étude, § 2.3) : le signal est émis à côté, dans la même transaction, par `platform.signal()`. Le canal `negotiation_channel_message` (`100_negotiations.sql`) reste sans usage tant que les échanges de Guide Négo ne sont pas construits.
