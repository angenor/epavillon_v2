# A10 — Gestion des événements

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de cet écran est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md).

**État** : ✅

---

## Ce qui a été livré

Fait le 18/08. **Aucune modification du modèle** — le deuxième écran du jalon dont le SQL couvrait déjà tout (`060_events.sql` § 1 à 7 portait les dix-huit champs d'une édition, la cardinalité 0..1 de l'appel, la grille pondérée et le comité). 3 pages sous `app/pages/admin/evenements/`, 8 composants sous `app/components/admin/events/`, 1 utilitaire pur (`utils/edition-list.ts`), 1 fichier de contrats (`types/admin-events.ts`), **1 dossier de mocks découpé en trois** (`mocks/admin-events/` : `core`, `detail`, `writes` — le fichier unique dépassait le garde-fou de mille lignes), 1 fabrique d'API (`composables/api/admin-events.ts`, montée sous `adminEvents` pour ne pas se confondre avec les lectures publiques d'`events`), 6 fichiers de traduction (3 × 2 locales). **Deux ajouts au jeu de données** : les thématiques des journées spéciales (`('event','programme_tracks',id)` dans `reference.entity_terms`, annoncées par le commentaire du SQL et absentes des mocks) et `mockUuid`, exporté d'`ids.ts` pour les entités créées à l'exécution. **Une action du store** : `adminScope.reload()` — une édition créée entre dans le périmètre, et `ensureLoaded()` est idempotent par personne.

---

## Écarts relevés en écrivant la gestion des événements (A10, 18/08)

Le modèle n'a rien exigé de nouveau — le premier point n'est pas un manque du SQL mais une frontière mal placée entre lui et l'application.

1. **`event.event_days` n'a AUCUN trigger de dérivation, et le prompt demande « générées depuis les dates ».** La génération est donc un comportement d'APPLICATION, non du modèle : rien en base ne crée un jour quand une édition change de période, et rien n'en supprime quand elle se resserre. C'est défendable — supprimer un jour détacherait ses séances (`xmod_fk_sessions_event_day ON DELETE SET NULL`), et cette décision appartient à l'équipe. L'écran en a donc fait un GESTE EXPLICITE : un bouton, un plan annoncé avant d'agir (combien à créer, lesquels sortent de la période, combien de séances ils portent), et le retrait des jours hors période sous une case à cocher. **À trancher au prompt B3** : la génération reste-t-elle applicative, ou devient-elle une fonction `event.generate_event_days(uuid)` que l'API appelle ? La seconde option a le mérite de garantir qu'aucune édition ne vive sans calendrier.

2. **`event.event_days` n'a pas de sens pour une série de webinaires.** Le modèle décrit un CALENDRIER de conférence : une ligne par jour, ce qui donne douze lignes pour une COP. L'édition `paco2027` du jeu de données court sur près d'un an : la génération proposait **302 journées vides**. Rien n'est faux — la période est légitime, c'est la granularité jour par jour qui ne l'est pas pour ce genre de série. L'écran avertit au-delà de quarante jours plutôt que d'écrire trois cents lignes en silence. **À arbitrer avec le commanditaire** : soit `event_days` n'est généré que pour les séries de genre COP, soit une édition `webinar_series` s'en passe entièrement et ses séances se rattachent au jour par leur seule date. Aucune correction du SQL n'est requise dans l'immédiat.

3. **`EditionListRow` ne porte ni `description` ni `highlights`, et c'est délibéré.** Une ligne de tableau à huit colonnes n'a pas à charger deux paragraphes par édition. Les deux textes sont donc portés par `EditionDetail`, que le formulaire de modification consomme. Première tentative : les relire par `api.events.bySlug()` — un appel de plus pour deux colonnes de la même table. Corrigé avant la fin. **Obligation inscrite au prompt B3** : `GET /admin/events/:id` embarque la description, le message d'accueil ET la bannière résolue, comme le fait déjà `v_public_schedule` pour les séances.

4. **~~La bannière d'une édition ne se téléverse pas encore.~~ REFERMÉ LE 26/08.** `event.events` ne porte toujours pas son image — le rattachement média reste polymorphe (`media.attachments`) —, mais le fichier part maintenant de l'écran : `MediaImageField` dépose par `POST /media/assets`, le formulaire rattache par `PUT /media/attachments`. Les trois déclinaisons sont recadrées **à la forme exigée par `media.attachable_roles`**, lue par `GET /media/roles` : la personne cadre, l'algorithme ne rogne pas. Le champ d'identifiant d'objet a disparu — il n'était remplissable que depuis la base.

5. **Deux permissions distinctes, et c'est une bonne chose.** `event.event.manage` couvre l'édition, ses journées, ses fils, ses lieux et ses canaux ; `event.call.manage` couvre l'appel et son comité. Les tester séparément permet à un chargé de programmation de composer les journées spéciales sans toucher à la grille d'évaluation. **La création d'une édition demande la portée GLOBALE** : une édition qui n'existe pas encore n'a aucun périmètre où vérifier un droit. C'est la règle métier n° 8 prise par l'autre bout, et elle mérite d'être confirmée au prompt B3.

6. **Le jeu de données ne portait aucune thématique de journée spéciale**, alors que `060_events.sql` § 3 bis l'annonce en toutes lettres (« Les thématiques d'un fil passent par reference.entity_terms ('event', 'programme_tracks', <id>) »). L'onglet des journées spéciales est le premier écran à les afficher : trois rattachements ont été ajoutés à `mocks/reference.ts`, et `themesOf()` s'est ouvert au schéma `event`. Corrigé le jour même.

7. **`Europe/Geneva` est un alias que la base de fuseaux de certaines exécutions Node refuse** (« Invalid time zone specified »). PostgreSQL l'accepterait ; l'affichage, lui, tombait, et **une seule exception emportait la liste entière des fuseaux** — le formulaire d'une édition restait indéfiniment sur son squelette de chargement, sans le moindre message. Deux corrections : le fuseau canonique `Europe/Zurich` porte le libellé « Genève », et le calcul du décalage rend l'identifiant brut plutôt que de lever. **La leçon dépasse ce champ** : la page rend désormais l'erreur de ses listes de référence au lieu de tourner. Un écran qui charge sans fin ne dit rien à personne.

8. **Le statut OIF de 220 pays reste à renseigner.** L'extension du semis à la norme ISO 3166-1 entière laisse `oif_status = 'none'` sur tout ce qui n'était pas déjà curé. Ce n'est pas un oubli : l'OIF compte 88 États et gouvernements, la liste évolue à chaque Sommet et fait autorité chez elle. Une trentaine de membres — dont plusieurs États d'Europe centrale et orientale, et plusieurs observateurs — porte donc `none` à tort. **À reprendre avec l'IFDD depuis la liste officielle** ; d'ici là, tout écran qui filtrerait sur `oif_status` sous-compterait.

9. **Les noms de pays ajoutés viennent de CLDR, pas du protocole.** « Congo-Kinshasa » plutôt que « République démocratique du Congo », « Turquie » plutôt que « Türkiye ». Les 29 fiches curées gardent leur forme protocolaire et priment ; les autres se corrigeront depuis le back-office au fil de l'eau, `reference.countries` étant une table ordinaire. Le compromis est assumé : recopier 220 noms à la main aurait offert 220 occasions de faute de frappe, sans garantie d'être plus protocolaire.

---

## Ce qui a été vérifié le 18/08 sur la gestion des événements, et comment

`npm run typecheck` et `npm run build` au vert. Le reste au navigateur, connecté comme administratrice globale, sur les données simulées.

- **La liste** rend les cinq éditions avec leurs décomptes joints : « COP31 · 12 jours · 35 propositions · appel ouvert · publiée le 3 août 2026 · 24 sur 30 activités placées ». Chaque ligne affiche ses dates DANS SON PROPRE FUSEAU (« du 9 au 20 novembre 2027 — heure de Belém »), ce qui est la particularité de cet écran : ailleurs tout se lit dans le fuseau d'une édition retenue.
- **Les six onglets** de la COP31 s'ouvrent et portent les bonnes données : 12 journées (dont « Ouverture du pavillon » et « Journée finance » avec leur page dédiée), 2 journées spéciales avec leurs thématiques et leur responsable, 2 lieux et leurs salles (le stand physique et l'atelier virtuel, chacun rappelant laquelle des deux règles de conflit s'y applique), 1 canal par défaut, l'appel avec sa prolongation ET son échéance d'origine côte à côte, les 6 critères dont l'éliminatoire et ses 32 notes déjà posées, les 5 membres du comité avec leur charge.
- **Trois refus, provoqués à la main.** `ux_programme_tracks_code` : « Ce code est déjà utilisé par un autre fil de cette édition. » `ux_events_slug` : « Cet identifiant d'adresse est déjà pris par une autre édition. » `ck_events_physical_location` : « Le pays et la ville sont obligatoires dès que l'édition n'est pas entièrement en ligne. » Chacun sur le bon champ, aucun message technique.
- **Une écriture qui aboutit** : un troisième fil créé sur la COP31, le tiroir se ferme, le compteur d'onglet passe de 2 à 3 et la carte apparaît. La fermeture n'a lieu qu'APRÈS acceptation — un refus laisse le formulaire ouvert avec sa saisie.
- **Le plan de génération des journées** : sur la COP31, le bouton est inerte (rien à créer, rien hors période) ; sur `paco2027`, il annonce 302 journées et l'avertissement de période anormalement longue.
- **Ce que le découpage a coûté** : le fichier de mocks d'un seul tenant faisait 1 474 lignes. Découpé en `core` / `detail` / `writes` (301 / 370 / 930), sans cycle — `writes` → `detail` → `core` —, puis les quatre écrans revérifiés au navigateur après découpage.
- **Un piège d'outillage, pas de code** : deux serveurs de développement partagent `frontend/.nuxt/`. En lancer un second pendant qu'un premier tourne casse les deux (« worker entry not found »). Se rabattre sur celui qui tourne, ou arrêter avant de relancer.

**Après les retours du commanditaire (reprise du 18/08).** Base **rechargée de zéro** (`down -v` puis `up`) : le schéma se charge sans une erreur, 249 pays semés, `event.events` porte bien `latitude` et `longitude`, et le rapport des frontières inter-modules reste à zéro. Les trois contraintes de coordonnées éprouvées sur une ligne réelle dans une transaction annulée — une latitude seule est refusée, une latitude de 95 est refusée, le point de Belém est accepté. `platform.slugify()` comparée à `utils/slug.ts` sur six libellés, dont « Côte d'Ivoire » et « COP31 — Conférence… » : sorties identiques. Au navigateur : **250 options de pays et 419 de fuseau** dans le formulaire, le slug se compose pendant la frappe (« COP32 » → `cop32`), il cesse de suivre après correction manuelle, il ne bouge pas sur une édition existante dont on change le libellé, l'éditeur enrichi est monté avec sa barre d'outils (gras, italique, listes), et une latitude sans longitude est refusée au bon champ. Le lien « Vérifier le point sur un plan » ouvre bien la bonne épingle.

---

## Le téléversement des trois déclinaisons — 26/08

Écart n° 4 refermé, à la demande du commanditaire : « impossible de téléverser directement une image ».

**Ce qui existait, et ce qui manquait.** Le dépôt (`POST /media/assets`, B6) et le rattachement (`PUT /media/attachments`, B3) étaient tous deux servis. Entre les deux, l'écran demandait un **identifiant d'objet** — un champ de texte que seule une personne ayant accès à la base pouvait remplir. Les trois emplacements étaient donc décoratifs.

**Ce qui a été écrit.**

- `UiImageEditor` — le recadrage, sur canevas. Sélection déplaçable et redimensionnable à la souris, au doigt (événements *pointer*, capture comprise) et **au clavier** (flèches pour déplacer, `Maj` pour redimensionner, `Alt` pour le pas fin). Grille des tiers. Format JPEG · WebP · PNG, qualité, largeur de sortie. **Le poids affiché est mesuré, jamais estimé** : chaque réglage réencode l'image et conserve le fichier produit, qui est celui qui partira. Les couleurs du canevas sont lues **une fois** dans les jetons CSS — un canevas ignore les variables, et une couleur écrite en dur rouvrirait la porte que `design-tokens.css` a fermée.
- `MediaImageField` — l'emplacement complet : choisir, recadrer, décrire, déposer, remplacer, retirer. Générique : il ne connaît ni les éditions ni leurs trois rôles.
- `api.media` — `roles()` et `upload()`, plus la primitive `sendForm` dans `useApi.ts`, seule écriture du site qui ne parle pas JSON. Les métadonnées sont ajoutées **avant** le fichier, comme la route l'exige : c'est ce qui lui permet de refuser un type, un poids ou un droit sans avoir lu un octet.

**Trois décisions qui méritent d'être retenues.**

1. **La forme est IMPOSÉE, plus apprise par le refus.** La poignée est verrouillée sur `expected_aspect_ratio` (3,5556 · 1,7778 · 1,0000), servi par `GET /media/roles`. Le trigger des 2 % reste en place et redevient ce qu'il doit être — un filet, pas un mode d'emploi. **Deux champs manquaient au type du site** ; l'obligation inscrite au contrat de la route est refermée.
2. **Une seule image part, pas trois tailles.** `lg`, `md` et `thumb` sont produites par le worker (`domain/variants.rs`). En fabriquer côté navigateur écrirait une seconde fois un invariant déjà porté, et déposerait trois objets pour un rôle, dont deux orphelins le jour même.
3. **Le texte alternatif se saisit DANS l'éditeur.** `ck_assets_alt_text_required` le rend obligatoire et le dépôt le refuse avant de lire le flux. Le demander après ferait perdre le fichier ; le demander avant le recadrage ferait décrire une image qu'on n'a pas encore cadrée.

**Trois pièges traités.**

- Un PNG transparent aplati en JPEG donne du **noir** là où il n'y avait rien — le canevas est peint en blanc avant le dessin, et le défaut ne se serait vu qu'après le dépôt.
- Le poids annoncé doit être **exact** : la route refuse un flux dont le poids diffère de sa déclaration. Il vient du `Blob`, jamais d'une estimation.
- La mesure du cadre passe par un **observateur de taille** : la modale vient de s'ouvrir au montage du composant, et une mesure unique dessinerait dans une largeur nulle sans rien signaler.

**Ce qui a été vérifié, et comment.** `npm run typecheck`, `npm run build` et `node scripts/check-api-contract.mjs` au vert — le contrôle du contrat reconnaît désormais `sendForm`, le chemin `/media/assets` est donc vérifié comme les autres. **Au navigateur, sur les données d'exemple, connecté comme administratrice globale :**

- `/admin/evenements/nouveau` — les trois emplacements annoncent leur forme et leur plafond lus de la base (« Format 32:9 — 15 Mo au plus »). Dépôt bout en bout sur le bandeau : choix du fichier, recadrage verrouillé au 32:9, description saisie, « Téléverser », aperçu affiché, boutons « Remplacer » et « Retirer ».
- `/admin/evenements/<COP31>` — le formulaire de modification rend les trois emplacements avec leurs images en place. Éditeur ouvert sur la couverture : forme annoncée « 16:9 », déplacement de la sélection à la souris avec butée sur le bord de l'image, redimensionnement par un coin. **Rapport de sortie mesuré à 851 × 479, soit 1,7766 pour 1,7778 exigé — 0,07 % d'écart, pour 2 % tolérés.**

**Corrigé dans la foulée, signalé par le commanditaire : « le bouton reste grisé ».** Aucun défaut de logique — la **description de l'image**, seul champ obligatoire, était rangée en bas de la colonne de réglages, à **huit cents pixels sous le pli** d'une boîte de dialogue haute de 381 px, et rien ne disait pourquoi l'envoi était fermé. Deux corrections : ce que l'appelant EXIGE passe **avant** les réglages facultatifs, juste sous l'aperçu ; et **le pied de la boîte dit la raison** du refus, à côté du bouton. La leçon dépasse cet écran : **un bouton grisé sans raison visible est une impasse**, et l'ordre d'une colonne de réglages n'est pas cosmétique.

**Deuxième passe sur le même signalement, 27/08 — « toujours grisé ».** Trois causes possibles restaient invisibles, et deux étaient de vrais défauts.

- **`encode()` pouvait échouer en silence**, par trois chemins : un contexte de canevas refusé, un `toBlob` rendant `null` (format non gravé dans le navigateur), un `drawImage` qui lève sur une sélection dégénérée. Dans les trois cas le fichier n'existait pas, le bouton restait fermé, **et rien ne l'expliquait**. Les trois se disent désormais.
- **Une image sans dimensions passait pour valide** — un SVG qui ne porte qu'un `viewBox` se charge sans erreur et mesure zéro. Elle est refusée à la lecture, comme une image illisible.
- **Le français est la langue de repli, et le champ de description ne remonte RIEN tant qu'il manque** : une description saisie dans le seul onglet anglais laisse l'envoi fermé, et l'écran ne peut pas distinguer « rien saisi » de « saisi dans l'autre onglet ». Le message nomme donc la langue.

**Le pied de la boîte ne peut plus être muet** : lecture impossible, encodage impossible, poids dépassé, description manquante, fichier en préparation — cinq raisons, dans l'ordre de la correction à faire. Un bouton fermé sans raison visible n'existe plus dans cet écran.

**Ce qui n'a PAS été vérifié.** Rien contre l'API réelle : hors ligne, le dépôt simulé rend une adresse `blob:` qui meurt avec la page. Le refus de quota et la déduplication n'ont pas d'équivalent simulé et restent à voir passer.
