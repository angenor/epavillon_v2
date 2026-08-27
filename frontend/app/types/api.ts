/**
 * Contrat d'API — ENGENDRÉ, jamais écrit à la main.
 *
 *   make openapi
 *
 * Source : les annotations posées auprès des gestionnaires Rust, assemblées par
 * `backend/crates/api/src/openapi.rs`. Un chemin absent d'ici n'existe pas dans
 * l'API ; `make check-api-contract` refuse tout appel du site vers un chemin
 * qui n'y figure pas.
 *
 * CE QUE CE FICHIER PORTE : les chemins, les verbes, les paramètres, les codes
 * d'erreur et la forme du corps d'erreur (`ApiError`).
 *
 * CE QU'IL NE PORTE PAS : la forme des corps de requête et de réponse, qui
 * sortent en `Record<string, never>`. L'API les désigne par leur NOM
 * TypeScript, dans la description de chaque opération — `EditionCallPayload` →
 * `CallSaveResult` —, et leur source unique reste `frontend/app/types/`. C'est
 * une décision de l'API, pas un oubli : décrire deux fois la même forme, une
 * fois en Rust et une fois en TypeScript, produit deux vérités qui divergent au
 * premier ajustement. `make check-api-contract` vérifie que chaque
 * nom annoncé par l'API existe bien là-bas.
 *
 * Ce fichier est exclu du garde-fou des mille lignes de CLAUDE.md : il est
 * engendré, il ne se lit pas et ne se modifie pas.
 */

export interface paths {
    "/admin/calls": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Ouvrir un appel — **l'appel et sa grille en une transaction**.
         * @description `EditionCallPayload` → `CallSaveResult`. **L'appel et sa grille en une seule transaction** : un échec sur la grille ne laisse aucun appel derrière lui. L'édition vient du corps, **et elle est vérifiée** — le périmètre d'administration est appliqué avant toute écriture. Une édition qui porte déjà un appel non annulé rend `already_exists` en 200 ; un appel **annulé** n'empêche rien, l'index l'exclut. `scores_affected` prévient qu'un barème modifié va déplacer des moyennes déjà calculées.
         */
        post: operations["admin_appel_creer"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/calls/default-criteria": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * La grille par défaut, **lue en base**.
         * @description `EditionCriterion[]` — les six critères que `event.seed_default_criteria()` sème, avec leurs libellés bilingues, leurs poids et l'éliminatoire. **Lue en base, jamais recopiée** : la fonction du modèle est exécutée sur un appel jetable, dans une transaction annulée dont rien ne subsiste. Recopier les six lignes dans un tableau Rust en ferait une seconde vérité, désynchronisée au premier ajustement de la grille. Les identifiants rendus sont nuls : ce sont des lignes **nouvelles**, que l'écran proposera d'enregistrer.
         */
        get: operations["admin_appel_grille_par_defaut"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/calls/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Modifier un appel — écriture **totale**, grille comprise.
         * @description `EditionCallPayload` → `CallSaveResult`. L'édition vient de **l'ascendance de l'appel**, jamais du corps. Écriture totale : tous les champs modifiables sont réécrits, y compris à nul, ce qui permet de retirer une prolongation. La grille est enregistrée par **diff sur le code** — insertion, mise à jour, suppression. **Retirer un critère porteur de notes est refusé** en 422 : la clé est `ON DELETE CASCADE`, et la base effacerait sans un mot l'argumentaire des évaluations rendues.
         */
        put: operations["admin_appel_modifier"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/calls/{id}/reviewers": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Enregistrer la composition — **ajouts, retraits et plafonds d'un seul
         *     geste**.
         * @description `CommitteePayload` → `CommitteeSaveResult`. **Ajouts, retraits et plafonds d'un seul geste**, dans une transaction : l'écran envoie la liste complète, et ce qui n'y figure plus est retiré. Les doublons de charge utile sont **dédoublonnés par le service**, jamais remontés comme erreur de base. Une personne inconnue rend `EVENT_UNKNOWN_REFERENCE` en 422, **en la nommant** — la clé étrangère refuserait aussi, mais sans dire laquelle des lignes est en cause. `removed_with_assignments` nomme les membres retirés portant encore des dossiers : leurs revues rendues restent au dossier, mais quelqu'un doit reprendre le reste. **Siéger n'accorde aucun droit** : `has_review_permission` le dit, il ne le donne pas.
         */
        put: operations["admin_comite_enregistrer"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/channels": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Créer un canal d'édition.
         * @description `EditionChannelPayload` → `EditionTabResult`. **Poser le canal par défaut retire le précédent dans la même transaction** : `ux_broadcast_channels_default` n'est pas différable, et l'ordre inverse échouerait. Le canal **général de la plateforme** forme son propre groupe et n'est jamais délogé — il sert les diffusions dont l'événement n'a pas le sien.
         */
        post: operations["admin_canal_creer"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/channels/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Modifier un canal — **jamais un canal général de la plateforme**.
         * @description `EditionChannelPayload` → `EditionTabResult`. Un canal **général de la plateforme** — sans édition — rend `platform_channel` en 200 : il sert plusieurs événements, et le modifier depuis l'un d'eux le changerait pour tous. Ce n'est ni un introuvable ni un refus de périmètre, c'est un refus que l'écran sait expliquer.
         */
        put: operations["admin_canal_modifier"];
        post?: never;
        /**
         * Retirer un canal — **désactivé s'il a servi, supprimé sinon**.
         * @description `EditionTabResult`. **`error_code: 'deactivated'` accompagne `ok: true` et n'est PAS un refus** : le canal a servi, il est désactivé plutôt que supprimé, pour garder la trace du canal sur lequel une activité passée a été diffusée. Sans séance à son compte, il est supprimé et `error_code` reste nul. `sessions_detached` porte le nombre de séances concernées, **compté avant**.
         */
        delete: operations["admin_canal_supprimer"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/dashboard": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * @description `AdminDashboard` — tout l'écran d'une édition **en une réponse et un instant** : l'édition et son fuseau, son appel, les cinq familles d'alerte, les chiffres, la santé opérationnelle et les messages d'incident actifs.
         *
         *     **Une transaction de lecture, un instant** : `now()` y est constant, et les dix lectures parlent donc du même. C'est la réponse aux « neuf instants de mesure » que le contrat du site interdit.
         *
         *     **Gardée par le périmètre ET par `analytics.dashboard.read` sur l'édition demandée.** Le rôle `programmer` la détient depuis le 27/08 : il lit déjà, écran par écran, tout ce que le tableau de bord agrège — la lui refuser lui retirerait un raccourci, pas un droit.
         *
         *     **Le tableau de bord n'a pas d'issue de contrat** : il s'ouvre, ou il se refuse. Périmètre vide ou permission absente → 403 ; édition hors périmètre → 404, **jamais 403**.
         */
        get: operations["admin_tableau_de_bord"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/email-suppressions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * La liste.
         * @description `EmailSuppression[]` — les adresses écartées du circuit, de la plus récente à la plus ancienne.
         *
         *     **Une suppression échue reste visible.** Elle n'écarte plus rien — `is_email_suppressed()` compare `expires_at` à maintenant —, mais savoir qu'une adresse a rebondi le mois dernier a de la valeur. Aucun travail récurrent ne les efface : une purge programmée serait un second dispositif à tenir d'accord avec la fonction du modèle.
         */
        get: operations["engagement_liste_de_suppression"];
        put?: never;
        /**
         * Inscrire une adresse.
         * @description `EmailSuppression` — **aucun module n'écrira plus à cette adresse**, y compris ceux livrés avant ce jalon : la garde enveloppe le contrat d'envoi du noyau, et aucun d'eux n'a été modifié.
         *
         *     Une adresse déjà inscrite est **mise à jour**, jamais refusée : un second rebond ne doit pas produire un conflit, et le motif le plus récent est celui qui explique le mieux pourquoi la personne ne reçoit plus rien.
         *
         *     `expires_at` lève la suppression toute seule le moment venu — une boîte pleine n'est pas une adresse morte.
         *
         *     L'inscription émet `engagement.email.suppressed`, dont la charge utile porte l'adresse **hachée** : l'outbox est durable, indexée et relayée, et une adresse électronique est une donnée personnelle.
         */
        post: operations["engagement_supprimer_une_adresse"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/email-suppressions/{email}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /**
         * Retirer une adresse.
         * @description `{ removed }` — l'adresse redevient joignable. `false` dit qu'elle n'y était pas : ce n'est pas une erreur, et rendre 404 obligerait l'écran à traiter comme un échec un état qui est celui qu'on voulait.
         */
        delete: operations["engagement_retirer_une_suppression"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/events": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * La liste, bornée par le périmètre.
         * @description `EditionListScreen` — les lignes, les séries proposables au filtre et les millésimes présents, **en une réponse**, les facettes comptées sur le **même jeu de lignes** que la liste. `is_global_scope` dit si l'appelant administre la plateforme entière, pour que l'écran distingue un filtrage d'une absence. **Un périmètre vide reçoit 403, jamais une liste vide** : personne ne doit avoir à deviner s'il n'y a rien à voir ou s'il n'a pas le droit de voir.
         */
        get: operations["admin_editions_lister"];
        put?: never;
        /**
         * Créer une édition.
         * @description `EditionFormPayload` → `EditionSaveResult`. **Portée GLOBALE exigée** : une édition qui n'existe pas encore n'offre aucune portée où vérifier un droit — `EVENT_GLOBAL_SCOPE_REQUIRED` sinon. Les refus de saisie sortent en **200**, dans `errors`, chacun sur son champ. Une édition **dont le pavillon est tenu** doit porter un sigle : le refus emprunte `{ code: 'required', field: 'acronym' }` et la réponse porte en plus `suggested_acronym`, une valeur dérivée du libellé, utilisable telle quelle. `days_created` compte les journées que la période a ajoutées ; `days_removed` et `sessions_detached` valent **toujours zéro** — un enregistrement d'édition ne supprime aucune journée.
         */
        post: operations["admin_edition_creer"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/events/form-options": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Ce qu'il faut pour ouvrir le formulaire.
         * @description `EditionFormOptions` — séries avec leur décompte d'éditions, pays, fuseaux et statuts. Servie **à part de la liste** : le référentiel des pays ne repart pas à chaque affichage du tableau. Les fuseaux viennent de `pg_timezone_names`, le dictionnaire même contre lequel le domaine du modèle vérifie ce qu'on écrit ; les statuts sont lus dans l'énuméré du modèle, dans l'ordre où il les déclare.
         */
        get: operations["admin_editions_options_de_formulaire"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/events/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Le détail d'une édition — **les six onglets en une réponse**.
         * @description `EditionDetail` — l'édition, ses deux textes longs, sa période en dates civiles **dans son fuseau**, ses trois déclinaisons d'image résolues, ses journées, ses fils, ses lieux et salles, ses canaux, son appel et sa grille, son comité, le personnel assignable et les thématiques disponibles. **Ouvrir un onglet ne demande aucun appel supplémentaire** : les douze lectures se font sur une seule connexion, dans une transaction en lecture seule, pour que les décomptes des six onglets soient cohérents entre eux. Une édition **inexistante ou hors périmètre** rend 404 — les deux sont indiscernables, sans quoi une URL forgée dirait à qui la forge si l'objet existe.
         */
        get: operations["admin_edition_detail"];
        /**
         * Modifier une édition — écriture **totale**.
         * @description `EditionFormPayload` → `EditionSaveResult`. Écriture **totale** : tous les champs modifiables sont réécrits, y compris à nul — c'est ce qui permet d'effacer un sigle, une ville ou des coordonnées. `programme_published_at` n'est **jamais** touchée ici : elle est posée par la publication seule. L'identifiant vient de l'adresse, jamais du corps. Une édition hors périmètre rend **404**, indiscernable d'une édition inexistante.
         */
        put: operations["admin_edition_modifier"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/events/{id}/days": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Générer le calendrier.
         * @description `{ remove_outside_period }` → `EditionTabResult`. **Le plan est recalculé dans la transaction d'écriture**, jamais repris du client : entre l'affichage et le clic, quelqu'un peut avoir modifié la période, et écrire d'après un état périmé reviendrait à supprimer une journée qui vient d'y entrer. **Sans le drapeau, aucune journée n'est retirée.** `sessions_detached` est compté **avant** le retrait. Le contenu éditorial des journées conservées n'est jamais écrasé.
         */
        post: operations["admin_journees_generer"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/events/{id}/days/plan": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * **Ce que la génération ferait, sans rien faire.**
         * @description `DayGenerationPlan | null` — **lecture seule : rien ne s'écrit**. Les dates de la période qui n'ont pas encore de journée, les journées **hors période avec le nombre de séances qu'elles portent**, et le nombre de journées déjà en place. Une période d'un an annonce plus de trois cents journées sans en écrire une. Ce chiffre par journée est ce qui permet à l'équipe d'arbitrer plutôt que de subir un retrait.
         */
        get: operations["admin_journees_plan"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/events/{id}/days/{dayId}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Habiller une journée — contenu **éditorial** seul.
         * @description `EditionDayPayload` → `EditionTabResult`. Titre, adresse de page, description, mise en avant et couleur. **La date ne se modifie pas** : une journée tient sa date de la période de l'édition, et la déplacer ferait un doublon ou un trou. L'édition vient de l'adresse ; l'identifiant de la journée est vérifié comme appartenant à cette édition.
         */
        put: operations["admin_journee_habiller"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/incidents": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * @description `IncidentListScreen` — tout l'écran d'une édition **en une réponse et un instant** : l'en-tête (titre, fuseau, ville), les lignes rendues par `live.event_incidents()` **dans l'ordre où elle les rend** (actifs, programmés, brouillons, historique ; gravité décroissante à état égal), le poste de direct, les compteurs par état, les natures d'incident et les cibles visables.
         *
         *     **Aucune permission n'est exigée** : lire les messages d'une édition qu'on administre n'est pas un privilège — un bandeau publié est de toute façon public. Ce qui est gardé, c'est le périmètre.
         *
         *     **Les cinq portées remontent**, la portée `organization` comprise dès lors que l'organisation anime une activité de l'édition, et un message `global` apparaît sur **chaque** édition administrée : une équipe qui pilote un pavillon doit savoir qu'un bandeau d'entretien le couvre.
         */
        get: operations["admin_incidents_lister"];
        put?: never;
        /**
         * @description `CreateIncidentPayload` → `IncidentWriteResult`, **toujours en 200**. Rédiger, et publier dans le même geste si `publish` est vrai.
         *
         *     **`granted` n'existe pas** : le site l'envoyait pour rejouer l'autorisation sur des données d'exemple, et un client qui déclare ses droits n'est pas un contrôle d'accès. **`from_event_id` reste** — c'est l'édition depuis laquelle on agit, donc l'ancre du contrôle de périmètre.
         *
         *     **Les dix issues sortent en 200**, `forbidden` et `not_found` compris : le contrat du site les nomme et l'écran les traduit champ par champ. L'autorisation se vérifie sur la **portée visée** — l'édition de la cible, ou la portée globale pour un message `global`.
         */
        post: operations["admin_incidents_creer"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/incidents/overrun-template": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * @description `OverrunTemplate` — de quoi pré-remplir le formulaire depuis le raccourci « Signaler un débordement » du planificateur, sans une saisie pendant que la salle attend : l'activité, son titre **résolu**, son créneau et son édition.
         *
         *     **`title` est ici résolu et non brut**, à la différence du reste : c'est une valeur de pré-remplissage de champ, que le site pose telle quelle. Le site lit cette route par `callOrNull` — un 404 y est une réponse, pas une panne.
         *
         *     **Cette route est déclarée AVANT `/admin/incidents/{id}`**, toutes deux étant en `GET`.
         */
        get: operations["admin_incidents_gabarit_debordement"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/incidents/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * @description `ManagedIncident` — un message, pour le relire et le corriger.
         *
         *     **L'édition d'un message se CALCULE, elle ne se lit pas** : pour les portées `session`, `event_day` et `organization`, la ligne ne porte aucune colonne d'édition. La route retrouve donc le message **par `live.event_incidents()`** sur les éditions du périmètre, ce qui rend le contrôle et la lecture indissociables.
         *
         *     Le site la lit par `callOrNull`.
         */
        get: operations["admin_incidents_relire"];
        /**
         * @description `UpdateIncidentPayload` → `IncidentWriteResult`. Corriger.
         *
         *     **La portée peut changer, et l'autorisation se vérifie sur celle d'ARRIVÉE** : déplacer un message d'une édition vers la portée globale exige la permission globale.
         *
         *     **Republier efface la dépublication** — instant, auteur, motif —, exactement comme le fait `live.publish_incident()`. Le comportement n'est pas recomposé : la fonction est appelée.
         */
        put: operations["admin_incidents_corriger"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/incidents/{id}/publish": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * @description `IncidentWriteResult` (`published`). Publier un brouillon depuis la ligne de liste, ou rétablir un message retiré.
         *
         *     Appelle `live.publish_incident(id)` : la fonction horodate, attribue depuis la session, efface le retrait **et émet** `live.incident.published`. **Le service n'émet rien** — un `emit_event` ajouté ici doublerait chaque ligne d'outbox.
         */
        post: operations["admin_incidents_publier"];
        /**
         * @description `UnpublishIncidentPayload` → `IncidentWriteResult` (`unpublished`). Retirer un bandeau, avec un motif. **Ce n'est pas une suppression** : la ligne demeure — instant, auteur, motif — et reparaît à l'historique de la liste.
         *
         *     **Un `DELETE` porteur d'un corps, et c'est délibéré** : le chemin est celui de la publication, le verbe dit qu'on la retire, et le motif accompagne le geste.
         *
         *     Appelle `live.unpublish_incident(id, motif)`, qui lève sur un message jamais publié ; le service **traduit** la levée en issue `not_published` plutôt que de rejouer la condition en amont.
         */
        delete: operations["admin_incidents_depublier"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/media/orphans": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les objets que plus rien n'utilise.
         * @description `OrphanAsset[]` — les objets **servables et non rattachés** depuis un délai, du plus lourd au plus léger, **variantes comprises**. C'est le mécanisme qui manquait totalement à la v1 : sans registre des usages, un fichier retiré d'une page restait sur le disque pour toujours.
         *
         *     `min_age_days` remplace le délai par défaut, qui vient des réglages. Zéro est accepté et rend **tous** les objets non rattachés, y compris ceux déposés il y a une minute : c'est ce qu'on veut pour vérifier, jamais pour purger en masse.
         *
         *     Un objet **rattaché n'y figure jamais**, quel que soit son âge — c'est la définition de l'orphelin que porte le modèle.
         */
        get: operations["media_orphelins"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/media/quotas": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Le tableau des quotas.
         * @description `QuotaRow[]` — plafond, consommation, nombre de fichiers et **part consommée**, par organisation, **triés par proximité du plafond** : ce qui demande une décision est en haut.
         *
         *     **Une organisation qui n'a rien déposé n'y figure pas** : sa ligne de quota n'existe pas tant qu'aucun octet n'a été écrit, et le plafond par défaut s'applique. L'absence de ligne est donc « rien déposé », jamais « aucun quota ».
         */
        get: operations["media_quotas"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/media/quotas/{organizationId}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Relever le plafond d'une organisation.
         * @description `QuotaRow` — le relèvement **prend effet immédiatement** : `media.has_storage_capacity()` lit la ligne à chaque dépôt, sans cache ni rafraîchissement.
         *
         *     La ligne est créée si elle n'existe pas : un plafond peut être relevé **avant** le premier dépôt, et les compteurs partent alors de zéro. Le geste est tracé par le journal d'audit du modèle.
         */
        put: operations["media_relever_le_plafond"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/message-templates": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * La liste des modèles.
         * @description `MessageTemplateRow[]` — les modèles, avec le nombre de révisions de chacun et **celle qui est servie**. `current_version` nul dit qu'aucune n'est publiée : le type part alors avec le texte de secours du module, et la trace d'expédition le dit.
         *
         *     Le compte des révisions vient de la même réponse que la liste : deux appels donneraient deux instants, et un écran annonçant « 3 révisions » sur une liste qui en montre quatre.
         */
        get: operations["engagement_modeles_de_messages"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/message-templates/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Le détail d'un modèle.
         * @description `TemplateDetail` — les révisions de la plus récente à la plus ancienne, celle qui est **servie**, et **les variables que le type promet**.
         *
         *     Cette dernière liste n'est pas décorative : sans elle, l'écran ne peut annoncer les variables disponibles qu'en les devinant, et un administrateur découvrirait le refus à la publication, après avoir écrit son gabarit.
         */
        get: operations["engagement_modele_de_message"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/message-templates/{id}/preview": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * L'aperçu — il n'envoie rien.
         * @description `{ fr, en }` — le rendu dans les **deux langues**, avec des valeurs d'exemple. **N'envoie rien, n'écrit aucune trace d'expédition, n'appelle pas l'expéditeur.**
         *
         *     Sans `version`, la révision servie est rendue — ou la plus récente si aucune n'est publiée, un brouillon devant se relire avant d'être publié.
         *
         *     **Une variable absente ne fait pas échouer l'aperçu** : elle prend une valeur d'exemple visible, `« prenom »`. Un aperçu sert à regarder une mise en page ; refuser de la montrer parce qu'un exemple manque le rendrait inutile. À l'envoi, la règle est l'inverse — un trou part chez deux mille personnes, et l'échec est la bonne réponse.
         *
         *     Une langue absente du gabarit se replie sur le français.
         */
        post: operations["engagement_apercu_de_modele"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/message-templates/{id}/versions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Écrire une révision.
         * @description `TemplateVersion` — le corps HTML est **assaini à l'écriture**, langue par langue, contre une liste blanche propre au courriel : tableaux et styles en ligne y sont admis, parce que les clients de messagerie ignorent les feuilles de style.
         *
         *     **Un `href="{{lien}}"` survit** : pour un analyseur d'URL, une variable est une adresse relative, et la normaliser détruirait le lien — un défaut qui ne se voit qu'à la réception du courriel.
         *
         *     **Une révision écrite n'est PAS servie.** Publier est un second geste : sans cette séparation, enregistrer une correction à moitié faite l'enverrait à deux mille personnes.
         *
         *     Le numéro de révision n'est pas reçu, il est **posé** : deux administrateurs qui enregistrent en même temps ne doivent pas se disputer un numéro.
         */
        post: operations["engagement_ecrire_revision_de_modele"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/message-templates/{id}/versions/{version}/publish": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Publier une révision — ou revenir à une précédente.
         * @description `TemplateDetail` — **publier fait avancer un pointeur, et republier une révision antérieure est le retour arrière.** Rien n'est jamais effacé.
         *
         *     **Refusée si le gabarit cite une variable que le type ne promet pas**, en la nommant et en listant celles qui le sont. Le refus arrive ici et non à l'envoi : à l'envoi, il serait trop tard pour corriger sans que personne n'ait rien reçu — le courriel partirait avec un trou, « Bonjour  , », et le trou ne se verrait qu'à la réception.
         */
        post: operations["engagement_publier_revision_de_modele"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/notifications/broadcast": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Diffuser une annonce.
         * @description `{ recipients, emailed }` — une notification par destinataire, **groupée par clé** : une même diffusion relayée deux fois n'écrit qu'une ligne par personne.
         *
         *     **Deux audiences et pas une de plus** : toute la plateforme, ou les inscrits d'une édition. Une troisième — « les référents d'organisation », « les négociateurs » — demanderait une définition que rien ne porte aujourd'hui, et l'inventer produirait une liste que personne n'aurait validée.
         *
         *     **Chaque canal est consulté séparément** : qui a coupé le courriel garde l'avis à l'écran, et inversement. L'annonce est de criticité basse — elle se coupe, contrairement à une alerte de sécurité.
         *
         *     `link_path` est un **chemin relatif** : un nom d'hôte de préproduction ne doit pas entrer en base.
         *
         *     **L'expédition est faite dans la requête**, sans travail différé : aucune des cinq tâches du jalon n'en prévoit un, et le geste se fait quelques fois par an.
         */
        post: operations["engagement_diffuser_une_annonce"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/organizations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * La liste du back-office, **bornée par le périmètre**.
         * @description `OrganizationListScreen`. Filtrée par périmètre — **organisations ayant déposé ou tenu une activité** dans les éditions administrées : une organisation n'appartient à aucune édition, c'est l'activité qui la rattache. `scoped_to_events` dit que la liste est restreinte. Les facettes sont comptées sur le même jeu de lignes. Un périmètre vide se refuse explicitement, **jamais par une liste vide**.
         */
        get: operations["admin_organisations_liste"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/organizations/duplicates": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * La file des doublons. **Permission de fusion, portée GLOBALE.**
         * @description `DuplicateQueueScreen` — deux sections : en attente (triées par similarité décroissante) et déjà tranchées. **Permission de fusion en portée globale.**
         */
        get: operations["admin_file_des_doublons"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/organizations/duplicates/{pairId}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Tranche une paire. **Permission de fusion, portée globale.**
         * @description `DuplicateDecisionResult`. `distinct` retire la paire de la file — le balayage ne la ressuscite pas ; `deferred` la met de côté. Rien n'est définitif : `deferred` posé sur une paire **déjà sortie** de la file l'y **ramène**, écartée comme reportée. Une paire **fusionnée** ne se rejuge pas — la réécrire effacerait la trace de la fusion sans la défaire. `merged` ne se pose jamais ici : c'est la fusion qui l'écrit.
         */
        put: operations["admin_decision_de_doublon"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/organizations/merge": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * La fusion. Permission de fusion, **portée globale**.
         * @description `MergeResult` — quatre issues. Un choix portant sur l'**adresse d'URL** est refusé en 422, champ nommé : elle reste celle de la fiche absorbée, et c'est ce qui fait que ses anciens liens continuent de fonctionner. Les arbitrages de champ s'appliquent **après** l'appel de la fonction de base, dans la même transaction.
         */
        post: operations["admin_fusionner"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/organizations/similar": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * **La seconde des deux lectures de recherche** (écart n° 23).
         * @description `SimilarOrganization[]` — **lecture non filtrée** : le domaine partagé fait entrer la fiche, c'est le signal le plus fiable du modèle. À l'inverse de `/organizations/similar`, qui n'admet que les ressemblances de dénomination.
         */
        get: operations["admin_organisations_similaires"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/organizations/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * La fiche entière — huit lectures assemblées.
         * @description `OrganizationDetail | null` — huit lectures assemblées. Une fiche **absorbée** s'ouvre normalement, coiffée de son renvoi vers la fiche vivante. Une fiche **hors périmètre** rend `null`, indiscernable d'une fiche inexistante — URL forgée comprise.
         */
        get: operations["admin_organisation_fiche"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/organizations/{id}/domains/{domainId}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Vérifie un domaine, et règle son rattachement automatique. **Permission de
         *     gestion.**
         * @description `OrganizationWriteResult`. `domain_taken` **nomme la fiche** qui détient déjà le domaine vérifié — sans ce nom, le refus est incompréhensible. Le rattachement automatique sur un domaine non vérifié rend `ORG_DOMAIN_VERIFICATION_REQUIRED`. Seule la méthode `manual` est livrée.
         */
        put: operations["admin_organisation_domaine"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/organizations/{id}/merge-preview": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * L'aperçu de fusion, **pour un sens donné**. Permission de fusion, portée
         *     globale.
         * @description `MergePreview | null` — calculé pour un sens, **recalculé à l'inversion** : le décompte n'est pas symétrique. `null` si l'une des fiches est introuvable ou déjà absorbée. Les avertissements sont **non bloquants** : l'écran ne décide pas à la place de l'équipe.
         */
        get: operations["admin_apercu_de_fusion"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/organizations/{id}/names/{nameId}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Confirme une dénomination. **Permission de gestion.**
         * @description `OrganizationWriteResult`. Une dénomination **posée par la base** — le nom légal, le sigle — ne se retire pas à la main : `ORG_NAME_IS_DERIVED`.
         */
        put: operations["admin_organisation_denomination"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/organizations/{id}/verification": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Pose ou retire le sceau. **Permission de gestion.**
         * @description `OrganizationWriteResult` — la fiche entière recomposée. Poser le sceau sur une fiche `candidate` l'**admet** du même geste ; le retirer ne change pas le statut.
         */
        put: operations["admin_organisation_verification"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/planner": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Tout l'écran d'arbitrage, en une réponse.
         * @description `PlannerScreen` — **tout l'écran en une réponse, conflits compris** : le fuseau de l'édition et le nom de sa ville, la date de publication du programme, les jours du calendrier, les salles, les journées spéciales, les canaux de diffusion, les séances **placées**, celles **à placer**, et les chevauchements. Les conflits ne sont pas un second appel : une grille affichée avant de savoir ce qui s'y chevauche montre, pendant une seconde, une programmation qui a l'air saine. Le tout est lu dans **une transaction en lecture seule, sur une connexion** — lus à un autre instant, les conflits décriraient une grille que l'écran n'affiche pas.
         */
        get: operations["seances_ecran_du_planificateur"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/planner/publish": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Publier la programmation — **le seul contrôle bloquant du module**.
         * @description `{ event_id }` → `PublishProgrammeResult`. **Le seul contrôle bloquant du module** : un point de gravité `blocking` rend `blocked: true`, **rien n'est écrit**, et la liste dit quoi régler. Les avertissements accompagnent sans retenir. Une publication qui aboutit estampille l'édition, **annonce** par un événement de domaine, et rend `published_count` — un décompte de désignation, pris sous l'instantané de la transaction, avec le prédicat même que l'annonce porte. **Republier est inoffensif** : la date d'origine ne s'écrase pas et aucun second événement n'est émis. Une édition **sans aucune séance publie**, avec zéro séance et une liste vide : ce n'est pas un conflit.
         */
        post: operations["planificateur_publier"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/planner/readiness": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Ce qui doit être réglé avant de publier.
         * @description `PublicationReadinessIssue[]` — **lecture seule**, consultable avant toute tentative : l'équipe voit ce qui bloque avant d'essayer, plutôt que de découvrir la liste après un clic. Conflits détectés et manques : séance sans créneau valide, séance sans lieu ni précision de lieu, diffusion sans canal, intervenant absent. **`occurs_at` est un instant, jamais un intervalle mis en forme** — une chaîne figée en base ne peut ni se traduire ni se situer dans le fuseau de l'édition. Seule la gravité `blocking` retient la publication.
         */
        get: operations["planificateur_controle_de_publication"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/privacy-requests": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description `PrivacyQueueScreen`. **Portée globale exigée** : jamais une file filtrée. */
        get: operations["file"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/privacy-requests/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Les quatre issues sortent en **200** : ce sont des refus prévus par le
         *     contrat du site. Le refus d'autorisation, lui, est un 403 rendu par
         *     l'extracteur, avant que ce gestionnaire existe.
         * @description `HandlePrivacyRequestPayload` → `PrivacyWriteResult`. **Les quatre issues sortent en 200.**
         */
        put: operations["traiter"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/proposals/transitions-backfill": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Semer les transitions déductibles des dossiers repris de la v1.
         * @description Nombre de dossiers traités et de lignes semées. **Portée GLOBALE exigée** : une reprise porte sur tout le corpus, et la borner à une édition n'aurait aucun sens. **Synchrone et rejouable** : la condition « journal vide » est dans la requête d'insertion, si bien qu'une seconde exécution rend zéro. **Elle n'émet AUCUN événement** — elle écrit dans le journal sans passer par la mise à jour de l'état, donc sans réveiller le déclencheur : émettre huit mille événements de dossiers décidés il y a deux ans déclencherait autant de courriels, le pire effet possible d'une reprise. Elle ne devine ni le passage par l'évaluation, ni une demande de correction : ce qui n'est pas dans les dates du dossier n'est pas déductible, et l'inventer serait pire qu'un trou.
         */
        post: operations["propositions_deduire_les_transitions_v1"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/reminder-rules": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les règles d'une édition.
         * @description `ReminderRule[]` — la règle de l'édition **et** celles de ses séances, dans cet ordre. Les décalages sont rendus **en minutes**, rangés du plus lointain au plus proche : `'1 day'` et `'24 hours'` sont le même intervalle pour la base et deux chaînes différentes pour un écran, ce qui suffirait à afficher deux fois le même rappel.
         *
         *     Gardé par `engagement.reminder.manage` **sur la portée de l'édition**, et borné par le périmètre d'administration : un compte détaché sur une COP ne lit pas les règles de celle d'à côté, y compris en forgeant l'URL.
         */
        get: operations["engagement_regles_de_rappel"];
        /**
         * Écrire — ou modifier — la règle d'une portée.
         * @description `ReminderRule` — **une LISTE de décalages, jamais un décalage seul.** Les quatre valeurs du défaut — 2 jours, 1 jour, 1 heure, 30 minutes — sont **cumulées** : ce n'est pas un choix parmi quatre, les quatre rappels partent. Une écriture qui n'accepterait qu'une valeur ferait croire le contraire, et la faute ne se verrait qu'au jour de la séance.
         *
         *     **Une règle de séance REMPLACE celle de son édition**, sans cumul — c'est ce qui permet de savoir ce qui va partir.
         *
         *     **Une seconde écriture pour la même portée MODIFIE la première** : l'unicité du modèle est traitée comme une modification, jamais comme une erreur. Rendre un conflit dirait « impossible » là où l'on voulait simplement changer ses décalages.
         *
         *     **La portée est exactement l'une des deux** — une édition ou une séance, jamais les deux, jamais aucune. Le refus sort sur le champ `scope`, et celui des décalages sur `offsets`, en disant lequel des quatre cas s'applique : trop peu, trop, négatif, ou **répété** — ce dernier étant absorbé en silence par la clé d'unicité du modèle, l'écran annonçant alors un envoi de plus qu'il n'y en aurait.
         */
        put: operations["engagement_ecrire_regle_de_rappel"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/reminder-rules/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /**
         * Couper une règle.
         * @description `{ cancelled_reminders }` — supprime la règle **et annule les rappels encore à traiter qu'elle gouvernait**, en rendant leur nombre.
         *
         *     Les annuler est ce qui distingue une coupure d'un simple oubli : sans cela, les rappels **déjà matérialisés** partiraient quand même, et l'administrateur qui vient de retirer la règle les verrait arriver sans comprendre.
         *
         *     Pour **couper sans supprimer**, écrire la règle avec `is_active: false` : les rappels déjà posés restent alors en place.
         */
        delete: operations["engagement_supprimer_regle_de_rappel"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/rooms": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Créer une salle.
         * @description `EditionRoomPayload` → `EditionTabResult`. L'édition vient du **lieu** désigné par la charge utile, vérifié en base : sans cela, une salle pourrait être posée dans le lieu d'une autre édition. **`is_virtual` est écrit tel quel et jamais déduit du mode de participation** — une salle virtuelle accepte les créneaux simultanés, et la déduire ferait taire le conflit de gravité haute qu'un stand unique doit signaler.
         */
        post: operations["admin_salle_creer"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/rooms/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Modifier une salle.
         * @description `EditionRoomPayload` → `EditionTabResult`. **L'édition vient de l'ascendance de la salle** — son lieu —, jamais du corps. Le lieu visé par la charge utile est vérifié : déplacer une salle d'un lieu à l'autre de la **même** édition est légitime, la déplacer ailleurs ne l'est pas.
         */
        put: operations["admin_salle_modifier"];
        post?: never;
        /**
         * Retirer une salle.
         * @description `EditionTabResult`. Les séances installées dans cette salle **retournent au panneau « à placer »** — la clé est `ON DELETE SET NULL`, aucune séance n'est perdue. `sessions_detached` les compte **avant** l'ordre de suppression.
         */
        delete: operations["admin_salle_supprimer"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/showcase": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * La liste et ses facettes.
         * @description `ShowcaseListScreen` — les lignes du périmètre, leurs facettes et les référentiels du filtre, **en une réponse**. Les lignes arrivent triées par emplacement puis rang, avec `is_first` / `is_last` déjà posés : c'est ce qui désactive les boutons d'ordre aux extrémités sans que l'écran recompte.
         *
         *     **`broadcast_state` n'est pas `status`** : une diapositive publiée dont la fenêtre s'ouvre la semaine prochaine est `scheduled`, une autre dont la fenêtre est close est `expired`. La liste dit ce que le public voit, pas seulement ce que l'éditeur a décidé.
         *
         *     **Un périmètre vide reçoit 403, jamais une liste vide.** Un contenu de plateforme (`event_id` nul) n'est visible qu'en portée globale.
         */
        get: operations["admin_vitrine_lister"];
        put?: never;
        /**
         * Création.
         * @description `ShowcaseFormValues` → `ShowcaseWriteResult`. La diapositive se place **en fin d'emplacement** : la placer en tête déplacerait silencieusement tout le reste du bandeau. `placement_rows` rend l'emplacement entier renuméroté.
         *
         *     **Les refus de validation sortent en 200**, avec leur champ et leur code : fenêtre inversée, organisation désignée ET nommée, libellé de lien sans lien, français manquant, couleur mal formée, contenu de plateforme sans portée globale.
         */
        post: operations["admin_vitrine_creer"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/showcase/new": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * L'écran de formulaire, en création.
         * @description `ShowcaseFormScreen` — le formulaire vierge et ses référentiels : natures, éditions du périmètre, organisations, personnes, pays, thématiques et les trois emplacements de média avec leurs contraintes, lues de `media.attachable_roles`.
         *
         *     `preview` porte le contrat **exact** du bandeau public : l'aperçu est rendu par le composant qui sert la vitrine, jamais par une seconde mise en page qui divergerait au premier ajustement de charte.
         *
         *     **Une administratrice détachée n'ouvre pas un contenu de plateforme** : `is_global_scope` est faux, et le formulaire s'ouvre alors sur son édition.
         */
        get: operations["admin_vitrine_formulaire_vierge"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/showcase/sessions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les séances d'une édition, pour la cascade « édition → séance ».
         * @description `ShowcaseSessionOption[]` — les séances **publiées** de l'édition demandée, dans l'ordre du temps, chacune avec son fuseau. Changer d'édition dans le formulaire change la liste sans recharger l'écran : sans cette route, la saisie en cours serait perdue à chaque changement. L'édition est vérifiée contre le périmètre **avant** la lecture.
         */
        get: operations["admin_vitrine_seances"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/showcase/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les valeurs seules — pour recharger le fond du formulaire après une écriture.
         * @description `ShowcaseFormValues` — la diapositive **sans les référentiels**. Sert à relire le fond du formulaire après une écriture sans repayer les natures, les éditions, les organisations et les pays que l'écran de formulaire embarque. Mêmes refus que lui.
         */
        get: operations["admin_vitrine_valeurs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        /**
         * Modification.
         * @description `ShowcaseFormValues` → `ShowcaseWriteResult`. Mêmes refus que la création. **Le périmètre se vérifie sur la source ET sur la cible** : on ne déplace pas une diapositive vers une édition qu'on n'administre pas, et on n'en fait pas un contenu de plateforme sans la portée globale.
         *
         *     `published_at` ne se rejoue jamais : c'est le déclencheur du modèle qui le pose au premier passage en `published`.
         */
        patch: operations["admin_vitrine_modifier"];
        trace?: never;
    };
    "/admin/showcase/{id}/duplicate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Dupliquer — le geste qui remet un témoignage de la COP30 à la COP31.
         * @description `ShowcaseWriteResult` — la copie part **en brouillon**, en fin d'emplacement : dupliquer un contenu publié et le voir sortir aussitôt sur l'accueil serait une publication que personne n'a demandée. Les thématiques suivent. `row` porte la COPIE, et `placement_rows` l'emplacement renuméroté.
         */
        post: operations["admin_vitrine_dupliquer"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/showcase/{id}/form": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * L'écran de formulaire, en modification.
         * @description `ShowcaseFormScreen` — la diapositive et tous ses référentiels. **Deux issues pour deux choses différentes** : une diapositive inexistante ou hors périmètre rend 404 — les deux sont indiscernables, sans quoi une URL forgée dirait à qui la forge si l'objet existe —, tandis qu'un contenu de plateforme demandé sans portée globale rend 403 **en le disant** : l'écran doit pouvoir expliquer pourquoi une ligne visible n'est pas modifiable.
         */
        get: operations["admin_vitrine_formulaire"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/showcase/{id}/order": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Monter ou descendre d'un cran, dans son emplacement.
         * @description `ShowcaseReorderPayload` → `ShowcaseWriteResult`. L'ordre est la fonction principale de cet écran — son absence était le défaut n° 6 de la v1. **Aux extrémités, la réponse est `ok: true` sans changement** : les boutons y sont déjà désactivés, et une erreur pour une action que l'écran n'offrait pas serait du bruit.
         *
         *     `placement_rows` rend l'emplacement **entier**, renuméroté : deux lignes ont bougé, et rafraîchir la seule ligne cliquée laisserait sa voisine afficher un rang faux.
         */
        post: operations["admin_vitrine_ordonner"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/showcase/{id}/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Publier, retirer, archiver — depuis la liste, sans ouvrir le formulaire.
         * @description `ShowcaseStatusPayload` → `ShowcaseWriteResult`. Trois actes de diffusion, pas une modification de contenu : ils ne touchent ni les textes ni les médias, et restent possibles à une main depuis le tableau. `placement_rows` est nul — aucun ordre ne change.
         */
        post: operations["admin_vitrine_statut"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/tracks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Créer un fil.
         * @description `EditionTrackPayload` → `EditionTabResult`. L'édition vient du corps **et elle est vérifiée**. Le fil, ses **thématiques** et sa page publique sont écrits dans le même geste : les séparer laisserait exister un fil publié sans ses pastilles. Les thématiques passent par le référentiel partagé, avec leur libellé et leur couleur — ce sont des **données**, jamais des traductions.
         */
        post: operations["admin_fil_creer"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/tracks/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Modifier un fil.
         * @description `EditionTrackPayload` → `EditionTabResult`. **L'édition vient de l'ascendance du fil**, jamais du corps. L'unicité du code et de l'adresse porte sur l'**édition** : deux COP peuvent chacune avoir leur « journée finance ». Refermer puis rouvrir la page publique **n'efface pas** la date de sa première ouverture.
         */
        put: operations["admin_fil_modifier"];
        post?: never;
        /**
         * Supprimer un fil — **la seule suppression du module qui cascade sur un
         *     rattachement éditorial**.
         * @description `EditionTabResult`. **Aucune séance n'est supprimée** : ce qui disparaît, ce sont les rattachements séance–fil, par cascade. `sessions_detached` compte ce travail éditorial perdu, **avant** l'ordre de suppression — après, le lien n'existe plus et le chiffre serait zéro. Le corps de la requête est ignoré.
         */
        delete: operations["admin_fil_supprimer"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/users": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description `UserListScreen`, borné par le périmètre d'administration. */
        get: operations["admin_utilisateur_liste"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/users/role-options": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Ce que l'appelant peut réellement accorder — rien de plus.
         * @description `RoleAssignmentOptions` — restreint à ce que l'appelant peut accorder.
         */
        get: operations["admin_utilisateur_options_dattribution"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/users/roles/{assignment_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /**
         * `DELETE` par la route, mais **pas une suppression** : la ligne reste, avec
         *     son auteur et son motif de retrait.
         * @description `RevokeRolePayload` → `RoleWriteResult`. **Pas une suppression** : la ligne reste.
         */
        delete: operations["admin_utilisateur_retirer_role"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/users/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Hors périmètre, la fiche sort quand même — avec `in_scope` à faux, qui la
         *     met en lecture seule côté écran. `null` ne dit qu'une chose : la personne
         *     n'existe pas.
         * @description `UserDetail | null`. **Hors périmètre → 200 avec `in_scope: false`.**
         */
        get: operations["admin_utilisateur_fiche"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/users/{id}/effective-permissions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description `EffectivePermissionsView`. */
        get: operations["admin_utilisateur_permissions_effectives"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/users/{id}/roles": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * **Deux gardes, et ils ne disent pas la même chose.** `identity.role.assign`
         *     sur *au moins une* portée ouvre la route : sans elle, on n'est pas
         *     administrateur du tout, et le refus est un **403** — pas un discriminant, qui
         *     laisserait un compte ordinaire lire les rôles de n'importe qui en sondant
         *     cette route. `forbidden_scope`, lui, répond à un administrateur qui vise une
         *     portée hors de la sienne : c'est un refus qu'il peut comprendre et corriger.
         * @description `GrantRolePayload` → `RoleWriteResult`. **Les six issues sortent en 200.**
         */
        post: operations["admin_utilisateur_attribuer_role"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/users/{id}/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * **Portée globale.** Une suspension vaut sur toute la plateforme : il n'existe
         *     aucune édition à laquelle la rapporter, et un administrateur détaché sur une
         *     COP ne peut pas fermer un compte qui sert ailleurs.
         * @description `SetPersonStatusPayload` → `PersonWriteResult`. Portée **globale**.
         */
        put: operations["admin_utilisateur_changer_le_statut"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/venues": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Créer un lieu.
         * @description `EditionVenuePayload` → `EditionTabResult`. L'édition vient du corps **et elle est vérifiée** : à la création, l'objet n'a pas encore d'ascendance en base. La réponse porte la **composition entière recalculée**, ce qui garantit que les décomptes des cinq autres onglets restent justes.
         */
        post: operations["admin_lieu_creer"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/admin/venues/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Modifier un lieu.
         * @description `EditionVenuePayload` → `EditionTabResult`. **L'édition vient de l'ascendance du lieu**, jamais du corps. Écriture totale : tous les champs modifiables sont réécrits, y compris à nul.
         */
        put: operations["admin_lieu_modifier"];
        post?: never;
        /**
         * Retirer un lieu — **et ses salles avec lui**.
         * @description `EditionTabResult`. Retirer un lieu emporte ses salles par cascade ; les séances qui s'y tenaient **ne disparaissent pas**, elles retournent au panneau « à placer ». `sessions_detached` les compte, **avant** l'ordre de suppression : après, le lien n'existe plus et le chiffre serait toujours zéro. Le corps de la requête est **ignoré** — l'édition vient de l'ascendance du lieu.
         */
        delete: operations["admin_lieu_supprimer"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/auth/login": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description `LoginPayload` → `LoginResult`. **Les six issues sortent en 200.** */
        post: operations["login"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/auth/logout": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Réussit même sans session : se déconnecter deux fois n'est pas une erreur.
         * @description Ferme la session portée par le cookie. **Réussit même sans session.**
         */
        post: operations["logout"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/auth/me": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * **Pas de 401.** Le store du site appelle cette route à chaque navigation, y
         *     compris déconnecté ; un statut d'erreur y ferait afficher un écran en panne
         *     au lieu d'un état déconnecté. Aucun identifiant n'est accepté du client :
         *     c'est la session qui dit qui parle (FR-034).
         * @description `Person | null`. **Jamais 401** : le site appelle cette route déconnecté.
         */
        get: operations["me"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/auth/password-reset": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * **Réponse invariable** (FR-036) : adresse connue ou non, la réponse est la
         *     même. Seul le courriel diffère, et il n'arrive que si le compte existe.
         * @description `PasswordResetRequestResult`. **Réponse invariable.**
         */
        post: operations["request_password_reset"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/auth/password-reset/check": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Contrôle **avant** d'afficher le formulaire : il ne consomme rien, et ne vaut
         *     aucune garantie — le jeton est revérifié à l'envoi (FR-042).
         * @description Contrôle du lien **sans le consommer**, avant d'afficher le formulaire.
         */
        get: operations["check_password_reset_token"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/auth/password-reset/confirm": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * **Deux statuts, et ils ne disent pas la même chose.** Un jeton refusé sort en
         *     200 avec son discriminant : l'écran propose de redemander un lien. Un mot de
         *     passe refusé sort en 422 sur le champ `password` : le formulaire se corrige
         *     sur place, sans repasser par la boîte aux lettres.
         * @description Le jeton est **revérifié ici**, pas seulement au contrôle. Révoque toutes les sessions.
         */
        post: operations["reset_password"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/auth/refresh": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** @description Rotation du jeton de session. */
        post: operations["refresh"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/auth/register": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * **Aucune session n'est ouverte par l'inscription** : l'adresse n'est pas
         *     encore vérifiée, et une adresse non vérifiée ne se connecte pas (FR-024).
         * @description `RegisterPayload` → `RegisterResult`. **Réponse invariable**, adresse libre ou prise.
         */
        post: operations["register"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/auth/verify-email": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Les trois refus sortent en **200** avec leur discriminant : le site les
         *     distingue pour ne pas envoyer redemander un courriel à qui a déjà cliqué.
         * @description `VerifyEmailResult` — « déjà utilisé » avant « périmé ».
         */
        post: operations["verify_email"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/auth/verify-email/resend": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * **Réponse invariable** (FR-036) : adresse inconnue, déjà vérifiée ou en
         *     attente, la réponse est la même.
         * @description `ResendVerificationResult`. **Réponse invariable.**
         */
        post: operations["resend_verification"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/event-series": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les séries, avec leur décompte d'éditions.
         * @description `EventSeries[]` — les séries avec leur **genre** et leur décompte d'éditions. C'est `kind` qui distingue une COP d'un cycle de webinaires, jamais une liste d'adresses recopiée dans un composant. Le décompte est joint **par la gauche** : une série sans édition reste visible, à zéro.
         */
        get: operations["series_devenements"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Le sélecteur d'édition du back-office.
         * @description `EventEdition[]` — les éditions que l'appelant administre, pour le sélecteur du back-office. **Filtrée par le périmètre, et non refusée** : un périmètre vide rend une **liste vide**, et c'est le store qui décide de l'écran. C'est la seule route de ce module où périmètre vide n'est pas un refus, parce que le contrat du front le veut ainsi — toutes les autres lectures du back-office rendent 403. Décroissante sur la date de début.
         */
        get: operations["editions_du_perimetre"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/public": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les éditions publiques.
         * @description `PublicEditionRow[]` — la ligne de `event.v_public_editions`, et **non** `EventEdition`, la ligne nue de la table : elle porte en plus la série et le pays résolus, les trois déclinaisons d'image, l'état temporel, l'appel résolu et le volume du programme publié. Les éditions publiques, décroissantes sur la date de début. **Le critère de publicité vient du modèle** : ni brouillon, ni annulée. Il n'est recopié dans aucun écran, ce qui referme l'écart n° 26 — une édition **annoncée** dont le programme n'est pas publié en fait partie, car sa page existe et c'est là qu'on dépose un dossier. Chaque ligne porte sa série et son pays résolus, ses **trois déclinaisons d'image**, son état temporel, son appel résolu et le volume de son programme publié. **Ce chemin est déclaré AVANT `/events/{slug}`** : sans cela, `public` serait lu comme une adresse d'URL.
         */
        get: operations["editions_publiques"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/incidents": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * @description `ActiveIncident[]` — les messages actifs de l'édition, **le plus grave en tête**, dans l'ordre où `live.active_incidents_for_event()` les rend. Les cinq portées y remontent : édition, journée, activité, organisation **qui y anime**, et les messages globaux.
         *
         *     **Chaque ligne porte `target_label` déjà résolu** par le modèle — « Atelier de négociation », « Journée finance », le nom légal d'une organisation : le bandeau nomme son sujet, et un message de portée `session` reste lisible sur une page qui parle de trente activités.
         *
         *     **Aucune garde**, et **jamais 404** : une édition inconnue rend une liste vide. Le site n'en affiche que trois, le reste replié en « +N » — c'est la règle des pastilles de la charte ; l'API, elle, rend tout.
         */
        get: operations["evenement_incidents_actifs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{event_id}/sessions/{slug}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Le détail d'une séance publiée.
         * @description `{ session, speakers, organizations }` — la séance **publiée** désignée par son adresse d'URL dans son édition, avec ses intervenants et ses organisations. **Une adresse inconnue et une séance non publiée rendent le même 404** : distinguer les deux dirait au public qu'une séance existe sans être encore annoncée.
         */
        get: operations["programmation_seance_publique"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{id}/call": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * L'appel à propositions d'une édition — **zéro ou un**.
         * @description `PublicCall | null` — **zéro ou un, jamais un tableau** : `ux_calls_one_per_event` tient la cardinalité, et l'annulé est exclu. Zéro pour une COP sans pavillon, où l'IFDD n'envoie qu'un représentant. Porte sa GRILLE D'ÉVALUATION (`criteria`) : elle est publique par nature — une organisation qui prépare un dossier doit savoir sur quoi il sera jugé —, et la servir à part coûtait une seconde vague d'appels à la page qui l'affiche.
         */
        get: operations["appel_public"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{id}/channels": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les canaux de diffusion d'une édition, **et ceux de la plateforme**.
         * @description `BroadcastChannel[]` — les canaux de l'édition **et** les canaux généraux de la plateforme, comme le front les compose déjà. Un canal général sert les diffusions dont l'événement n'a pas le sien ; le taire ferait croire qu'aucun canal n'existe.
         */
        get: operations["canaux_publics"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{id}/days": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Le calendrier d'une édition.
         * @description `EventDay[]` — le calendrier d'une édition, une ligne par jour, croissant. Une journée spéciale n'est **pas** un jour du calendrier : elle vit dans les fils de programmation.
         */
        get: operations["journees_publiques"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{id}/images": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les trois déclinaisons d'image — **vouée à disparaître**.
         * @description `Record<EditionImageRole, AttachedImage | null>` — les trois déclinaisons résolues par `media.attached_image()`. **Livrée pour ne pas casser un écran déjà en place, et vouée à disparaître** : `GET /events/{slug}` porte désormais ces mêmes images, et cet aller-retour n'a plus de raison d'être. Son retrait est inscrit aux obligations de B7 (écart n° 25). Les trois clés sont toujours présentes, à `null` tant que rien n'a été téléversé.
         */
        get: operations["images_de_ledition"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{id}/rooms": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les salles de tous les lieux d'une édition.
         * @description `Room[]` — les salles de **tous les lieux** de l'édition : une salle ne porte pas l'édition, elle la tient de son lieu. `is_virtual` n'est pas un détail d'inventaire — une salle virtuelle accepte les créneaux simultanés.
         */
        get: operations["salles_publiques"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{id}/tracks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les fils de programmation **publiés**.
         * @description `ProgrammeTrack[]` — les fils dont la page publique est **ouverte**, et eux seuls. Un fil sans page ouverte n'existe pas pour le public : le filtre est `published_at IS NOT NULL`, la colonne même que le modèle indexe pour cet usage.
         */
        get: operations["fils_publics"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{id}/venues": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les lieux d'une édition.
         * @description `Venue[]` — les lieux d'une édition. Ce sont eux qui donnent un **sujet nommable** à un conflit de créneaux : sans salle en base, la détection ne peut dire que « deux activités à 14 h ».
         */
        get: operations["lieux_publics"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/events/{slug}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * La page d'une édition, par son adresse d'URL.
         * @description `EventEdition | null` — **une requête, deux vues**. L'édition, sa série, son pays, ses **trois déclinaisons d'image** résolues, son état temporel, son appel et l'échéance **effective** (prolongation comprise), plus le volume du programme publié, joint **par la gauche** : une édition annoncée sans aucune séance publiée reste visible. `null` pour un brouillon, une annulée ou une adresse inconnue — **les trois sont indiscernables**, sans quoi l'adresse d'une édition en préparation se devinerait.
         */
        get: operations["edition_publique"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/health": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Santé d'exploitation. Ce sont les chiffres de la plateforme entière, et il
         *     n'existe aucune édition à laquelle les rapporter — mais la permission n'est
         *     PAS exigée en portée globale pour autant : ce que ces indicateurs révèlent —
         *     des courriels en rebond, un outbox en retard — touche d'abord les rappels des
         *     activités d'un administrateur détaché, qui doit pouvoir les voir. La portée
         *     commande ce qu'on lit, pas la nature de ce qu'on regarde.
         * @description État d'exploitation, depuis `analytics.v_operational_health` : outbox en retard, travaux en échec, courriels en rebond, partitions manquantes. Protégé comme une donnée.
         */
        get: operations["health"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/home": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * La vitrine de l'accueil.
         * @description `Pick<HomeScreen, 'hero'>` — les diapositives du bandeau d'ouverture, **dans l'ordre de défilement** (`sort_order`, puis `id`). Le reste de l'accueil est servi par les modules qui en répondent : les éditions par `GET /events/public`, les prochaines séances par `GET /schedule`, et les chiffres du programme voyagent avec chaque ligne d'édition. **Le filtre de diffusion vient du modèle** : `content.v_showcase` ne rend qu'une diapositive publiée et dans sa fenêtre — il n'est recopié dans aucun écran. Tableau **vide** possible : la page d'accueil reste entière et s'ouvre alors sur l'appel à propositions.
         */
        get: operations["vitrine_de_l_accueil"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/internal/mail-events": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Ingérer un lot d'annonces.
         * @description `{ applied, ignored }` — les annonces de remise, de rebond et de plainte du fournisseur mettent la trace d'expédition à jour.
         *
         *     **Une annonce rejouée est IGNORÉE, jamais dupliquée** : le fournisseur rejoue volontiers, et rendre une erreur le ferait recommencer sans fin. Une annonce dont la trace est introuvable est ignorée de la même façon — l'identifiant du fournisseur est la seule chose qui les relie, et une trace effacée par la purge de partition n'est pas un incident.
         *
         *     **Un rebond dur ou une plainte inscrivent l'adresse sur la liste de suppression** : c'est le seul geste qui protège la réputation du domaine sans intervention humaine. Un rebond souple, non — une boîte pleine n'est pas une adresse morte.
         *
         *     **Hors session**, authentifiée par un jeton porteur. Non montée si le jeton n'est pas configuré : elle rend alors 404, comme un module éteint.
         */
        post: operations["engagement_ingerer_les_retours_de_courriel"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/media/assets": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Le dépôt — métadonnées puis fichier, en un seul geste.
         * @description `Asset` — le dépôt. Corps **composite** (`multipart/form-data`), seule route de l'API qui ne parle pas JSON.
         *
         *     **Les champs de métadonnées précèdent le fichier**, ce qui permet de refuser un type, un poids ou un droit sans avoir lu un octet. Champs acceptés : `filename`, `mime_type`, `byte_size`, `owner_schema`, `owner_table`, `owner_id`, `role`, `alt_text`, `caption`, `credit`, `license_code`, `visibility` ; puis la partie `file`.
         *
         *     **L'empreinte est calculée pendant la réception**, jamais reçue du client sans être recalculée. Si le contenu est déjà connu du dépôt de stockage, **aucun second objet n'est écrit** et l'objet existant est rendu — c'est le succès de la déduplication, et la réponse porte alors `deduplicated: true`.
         *
         *     **Le texte alternatif est exigé pour une image**, avant lecture : la base interdit à une image d'être servie sans lui, et accepter le dépôt produirait un objet bloqué en traitement pour toujours.
         */
        post: operations["media_deposer"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/media/assets/precheck": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * L'annonce préalable — **elle n'écrit rien**.
         * @description `UploadVerdict` — ce que le dépôt ferait de ce fichier, **sans qu'un octet soit envoyé**. Accepté, refusé pour son type, refusé pour son poids, refusé faute d'espace, ou **l'objet existant** si une empreinte est fournie et déjà connue.
         *
         *     **Tous les refus sortent en 200** : une annonce est une question, pas une tentative, et un refus y est une réponse. Le seul refus qui sorte en erreur est celui du **droit d'écrire sur l'entité visée** — il ne se distingue pas d'une entité inexistante, et n'a donc rien de plus à dire.
         *
         *     **Rien n'est réservé** : ni espace, ni clé, ni identifiant. Sans envoi qui suive, il ne reste aucune trace.
         */
        post: operations["media_annoncer"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/media/assets/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Un objet, avec l'adresse composée de son original et ses déclinaisons prêtes.
         * @description `Asset` **+** `url` et `sources`. L'adresse est **composée** par la base depuis le point d'accès courant : aucune clé de stockage nue ne sort d'ici, et c'est ce qui rend une migration de fournisseur indolore.
         *
         *     **Un objet non servable n'est pas absent** : en traitement, en échec ou en quarantaine, il est rendu **en 200 avec son état** — « en cours » et « en échec » se lisent tous les deux « pas encore là », et les distinguer demande que l'API le dise. `sources` est alors un objet **vide mais présent**, et `url` porte déjà l'original : l'écran affiche l'image, pas un trou.
         *
         *     Seule la suppression rend 404.
         */
        get: operations["media_objet"];
        put?: never;
        post?: never;
        /**
         * Supprimer un objet.
         * @description `{ scheduled_purge_at }` — l'objet cesse d'être servi, la consommation baisse **immédiatement**, et il reste récupérable jusqu'à cet instant. La disparition du stockage, elle, appartient au travail récurrent de purge.
         *
         *     **Refusée si l'objet est encore rattaché** (`MEDIA_ASSET_IN_USE`), en disant combien de fiches l'utilisent. La déduplication traverse les propriétaires : le même fichier déposé par deux organisations ne donne **qu'une** ligne, et sans ce refus la première ferait disparaître l'image de la seconde (écart n° 128).
         */
        delete: operations["media_supprimer"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/media/assets/{id}/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * L'avancement du traitement d'un objet.
         * @description `AssetProgress` : état, verdict d'analyse et **moteur**, dimensions relevées, déclinaisons prêtes sur déclinaisons attendues.
         *
         *     **Sans cette route, un écran ne sait pas distinguer « en cours » de « en échec »** : les deux se lisent « pas encore là ». Un objet en échec ou en quarantaine rend donc son état ici, en **200** — il est simplement absent des lectures publiques.
         *
         *     **Le nombre attendu se compte, il ne s'annonce pas** : une image plus petite que la plus petite taille configurée n'en produit aucune, et annoncer trois attendues laisserait l'avancement bloqué à zéro sur trois pour toujours. Tant que le relevé n'a pas eu lieu, il vaut zéro.
         *
         *     **Le verdict `unsupported` n'est pas une absence de verdict** : c'est « aucun moteur ne sait analyser ceci », et `scan_engine` dit alors qui a répondu — `none` quand aucun moteur n'est branché.
         */
        get: operations["media_avancement"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/media/attachments": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les médias d'une entité.
         * @description `AttachedImage[]` **+** `attachment_id`, `role`, `sort_order` et `status` — trois champs sans lesquels l'écran qui **gère** les médias d'une entité ne peut rien faire : on ne sait pas quoi détacher, ni où ranger la ligne, ni comment réordonner une galerie.
         *
         *     **Cette lecture ne masque pas les objets encore en traitement**, contrairement aux lectures publiques : un fichier déposé il y a trois secondes est parfaitement valide et pas encore servable, et le masquer ferait croire que le téléversement a échoué. `sources` y est alors **vide mais présent**, et `url` porte déjà l'original. Un objet **supprimé**, lui, n'est pas rendu.
         *
         *     Ordonné par rôle puis par ordre de tri déclaré. `role` facultatif : sans lui, tous les rôles.
         *
         *     **La garde est celle de l'écriture** : ce que l'on peut changer, on peut le lire. Les pages publiques, elles, lisent par les routes de leur module.
         */
        get: operations["media_rattachements"];
        /**
         * L'écriture de remplacement, en lot.
         * @description `AttachedMedia[]` — **une liste d'affectations, appliquées en UNE transaction.** C'est elle que le formulaire d'édition appelle pour ses trois déclinaisons, et c'est ce qui referme l'obligation laissée par B3 : le rattachement s'écrit dans `media.attachments`, sans qu'une ligne du module Événements change.
         *
         *     **Chaque rôle nommé est vidé puis regarni**, dans l'ordre où ses affectations apparaissent. Un rôle **absent de la liste n'est pas touché**, et `asset_id: null` vide le sien **sans toucher aux autres**.
         *
         *     Ce même mécanisme réordonne une galerie : renvoyer la même liste dans un autre ordre suffit, et aucune route de réordonnancement n'a besoin d'exister.
         *
         *     **La transaction unique n'est pas un confort** : trois images enregistrées à moitié laisseraient une édition avec un bandeau neuf et une vignette ancienne, sans que rien ne le signale.
         */
        put: operations["media_remplacer_rattachements"];
        /**
         * Ajouter un objet à un rôle.
         * @description `AttachedMedia` — ajoute un objet à un rôle **multiple**. Sur un rôle exclusif déjà pourvu, le refus est explicite : c'est un remplacement qu'il faut demander, et `PUT /media/attachments` le fait.
         *
         *     **Les quatre contrôles de forme tombent AVANT l'écriture** — type, poids, cadrage, servabilité — non pour remplacer `tg_validate_attachment`, qui garde le dernier mot, mais pour savoir lequel de ses cinq refus nommer : il les lève sans nom de contrainte, et trois partagent le même état d'erreur.
         *
         *     **Le refus de forme cite ses quatre nombres** : dimensions reçues, rapport obtenu, rapport attendu, tolérance. « Les dimensions ne correspondent pas » n'apprend rien à qui doit recadrer.
         *
         *     `alt_text_override` vit sur le **rattachement** et ne touche jamais l'objet : un même fichier sert plusieurs fiches, et le texte pertinent n'y est pas le même.
         */
        post: operations["media_rattacher"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/media/attachments/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /**
         * Détacher.
         * @description `{ asset_kept: true }` — retire le **rattachement**. **L'objet stocké demeure**, et le champ le dit parce que c'est la question qu'on se pose en lisant la réponse : un même fichier illustre souvent plusieurs entités, la déduplication le garantissant.
         *
         *     Pour supprimer réellement un objet, `DELETE /media/assets/{id}` — qui refuse tant qu'il est encore rattaché.
         */
        delete: operations["media_detacher"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/media/roles": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les règles d'une entité.
         * @description `AttachableRoleRule[]` **+** `expected_aspect_ratio` et `aspect_ratio_tolerance` — les deux champs que le modèle déclare et que le contrat du front ne porte pas encore. **Sans eux, l'écran ne peut pas annoncer la forme attendue** : il l'apprend par le refus, après que le fichier a traversé le réseau. Leur ajout côté front est inscrit aux obligations de B7.
         *
         *     Le rapport est le quotient largeur ÷ hauteur — `3.5556` pour un 32:9, `1.0000` pour un carré — et il traverse **en texte** : `numeric(6,4)` n'a pas de représentant flottant exact, et un rapport affiché doit l'être tel qu'il est déclaré.
         *
         *     **Les rôles inactifs sont rendus, avec leur drapeau** : les masquer laisserait croire qu'un rôle n'a jamais existé, là où il a été fermé.
         */
        get: operations["media_roles"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/memberships/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /**
         * Retirer un membre, ou quitter une organisation.
         * @description `{ status: 'revoked' | 'last_manager' }`. Un référent retire un membre, ou une personne quitte l'organisation. Le retrait du **dernier référent actif** est refusé — contournable par `org.organization.manage`.
         */
        delete: operations["adhesion_revoquer"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/memberships/{id}/decision": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * La décision d'un référent sur une **demande**.
         * @description `Membership | null`. **Ne porte que sur une demande** (`invited_at` nul) : sur une invitation, `ORG_MEMBERSHIP_IS_INVITATION`. Un refus **révoque**, il ne supprime pas.
         */
        put: operations["adhesion_decision"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/notification-preferences": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les préférences, catalogue compris.
         * @description `NotificationPreferenceRow[]` — **le catalogue croisé avec les arbitrages**, canal par canal, jamais les seuls arbitrages : une liste vide ferait croire qu'aucun avis n'est servi, alors que l'absence de ligne signifie « les canaux par défaut du type ».
         *
         *     **`is_overridable` est le champ qui compte.** Une préférence posée sur un type **critique** — sécurité du compte, annulation de séance — est enregistrée telle quelle, mais elle n'oppose rien. Sans ce champ, l'écran afficherait un interrupteur éteint pour un avis qui part quand même, et la personne croirait s'être désabonnée.
         */
        get: operations["engagement_preferences_de_notification"];
        /**
         * Écrire un lot d'arbitrages.
         * @description `NotificationPreferenceRow[]` — le lot est écrit, et **la liste entière est rendue** : l'écran affiche l'état d'après sans second appel, et une préférence sans effet se voit immédiatement.
         *
         *     **Une préférence sur un type critique est enregistrée**, jamais refusée : refuser laisserait l'écran sans réponse à donner, et l'interrupteur reviendrait à sa position sans explication. C'est la lecture qui dit qu'elle n'oppose rien.
         *
         *     Un **type inconnu** est refusé ici, alors que l'envoi le refuse en silence : une ligne orpheline ne serait jamais relue, et la personne croirait avoir coupé quelque chose.
         */
        put: operations["engagement_ecrire_preferences_de_notification"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/notifications": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Le fil, **et le compte de non lues**.
         * @description `NotificationFeed` — les lignes **et** le nombre de non lues, dans la même réponse. Deux appels donneraient deux chiffres mesurés à deux instants, et un badge qui contredit la liste qu'il coiffe.
         *
         *     Le compte porte sur **toutes** les non lues, pas sur la page : un badge qui ne compterait que la page afficherait « 30 » pour toujours.
         *
         *     Une ligne peut porter un `group_count` supérieur à un : trois faits de même nature sur la même cible forment **une** ligne tant qu'elle n'est pas lue.
         */
        get: operations["engagement_fil_de_notifications"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/notifications/archive": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Archiver.
         * @description `{ archived }` — la liste d'identifiants est **exigée** : « tout archiver » n'est pas un geste qu'on fait par mégarde. Archiver marque aussi lu, une notification rangée n'ayant plus à peser sur le badge.
         */
        post: operations["engagement_archiver_notifications"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/notifications/read": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Marquer lues.
         * @description `{ marked }` — sans `ids`, **toutes** les non lues de la personne. Les siennes, et uniquement : le filtre porte sur le compte de l'appelant, jamais sur la seule liste d'identifiants reçue.
         */
        post: operations["engagement_marquer_notifications_lues"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/organizations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * La liste ouverte, **bornée**. Seule la page de guide de style l'appelle ;
         *     elle est livrée pour ne pas la casser.
         * @description `Organization[]` — bornée : défaut 50, maximum 200, fiches vivantes, triées par nom légal.
         */
        get: operations["organisations_liste"];
        put?: never;
        /**
         * Créer une organisation. **Rend 200 pour les deux issues** : une fiche créée,
         *     ou le nom déjà pris — qui n'est pas une erreur mais un refus prévu par le
         *     contrat, portant la fiche en cause.
         * @description `CreateOrganizationResult` — deux issues, toutes deux en 200. La fiche naît `candidate`, jamais `active`, et son créateur en devient référent actif. `name_taken` porte la fiche en conflit sous forme de `SimilarOrganization` : de quoi la rejoindre. **Une simple ressemblance ne bloque jamais.**
         */
        post: operations["organisation_creer"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/organizations/by-email-domain": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Ce que révèle le domaine de **son** adresse.
         * @description `EmailDomainMatch | null`. Le paramètre `email` est **ignoré** : le domaine vient de la session. `null` sur messagerie grand public ou domaine inconnu — l'écran ne propose rien, il ne devine pas.
         */
        get: operations["organisation_par_domaine"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/organizations/invitations/accept": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Accepter une invitation par son jeton. **N'exige pas de session** : le jeton
         *     est la preuve d'adresse, comme pour la vérification d'adresse de B1.
         * @description `AcceptInvitation` → `{ status, membership, organization }`. **Aucune session exigée** : le jeton est la preuve d'adresse. Si une session existe, elle doit désigner la même personne (`ORG_INVITATION_NOT_YOURS`). L'adresse est marquée vérifiée : le lien vient de la prouver. **`job_title` est exigée** — l'adhésion devient active, et une adhésion active porte toujours sa fonction : c'est la personne invitée qui la déclare, pas le référent qui l'a invitée.
         */
        post: operations["invitation_accepter"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/organizations/similar": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * **La première des deux lectures de recherche** (écart n° 23).
         * @description `SimilarOrganization[]` — **lecture filtrée** : seules les fiches entrées par une ressemblance de dénomination. Le domaine de l'appelant alimente le score sans faire entrer une fiche sans rapport. L'autre lecture, `/admin/organizations/similar`, ne filtre rien. Un terme sous deux caractères rend une liste vide, pas une erreur.
         */
        get: operations["organisations_similaires"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/organizations/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Une fiche, **telle quelle**, absorbée comprise : elle porte alors
         *     `merged_into_id`, et l'appelant sait quoi en faire. Les anciennes adresses
         *     continuent de mener quelque part, c'est la promesse de la fusion.
         * @description `Organization | null` — rendue telle quelle, absorbée comprise.
         */
        get: operations["organisation_fiche"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/organizations/{id}/editions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les éditions sur lesquelles cette organisation a déposé.
         * @description `EventEdition[] | null`, de la plus récente à la plus ancienne. Une organisation fidèle en a plusieurs, et sa liste de dossiers les groupe : un dossier de la COP30 ne se lit pas comme un dossier en cours. **Adhésion active exigée** — à défaut, `null` en 200, jamais une liste vide : « aucun dossier » et « ce n'est pas votre espace » ne se confondent pas.
         */
        get: operations["propositions_editions_de_lorganisation"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/organizations/{id}/invitations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Inviter quelqu'un par son adresse. **Référent actif de cette organisation.**
         * @description `InviteMemberResult` — trois issues. Crée la personne si l'adresse est inconnue, **sans compte et sans nom déduit de l'adresse**. `already_invited` propose de relancer, jamais d'émettre une seconde invitation.
         */
        post: operations["organisation_inviter"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/organizations/{id}/members": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Rejoindre une organisation. **Trois issues**, et `pending` n'est pas un
         *     échec : c'est le fonctionnement normal quand le domaine ne prouve rien.
         * @description `JoinOrganizationResult` — trois issues. L'organisation visée est **résolue** : rejoindre une fiche absorbée mène à la fiche vivante.
         */
        post: operations["organisation_rejoindre"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/organizations/{id}/workspace": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * La page d'accueil de l'espace.
         * @description `WorkspaceOverview` — l'organisation, l'adhésion de la personne connectée, ses dossiers avec leur journal et leurs demandes de correction ouvertes, ses membres, ce qui attend une action **de sa part**, et l'appel en cours. **Composition propre au soumissionnaire, jamais la vue de pilotage du comité** : ni note, ni note pondérée, ni rang, ni nom de membre du comité, ni inscrit nommé (FR-076, FR-077). **Gardée par l'adhésion active**, jamais par un périmètre d'administration : une organisation n'administre rien. Sans adhésion active, la réponse est `null` en 200, et non 404 : l'indiscernabilité voulue — inexistante et non-membre donnent la même réponse — ne demandait pas un statut d'erreur, et l'écran affichait « une erreur est survenue » là où il faut lire « vous n'avez pas d'espace ici ». Les séances programmées et leurs rappels partent **vides** jusqu'à B5 et B6 — un champ absent ferait échouer l'écran, un champ vide dit qu'il n'y a rien.
         */
        get: operations["propositions_espace_organisation"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/people": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * La liste des personnes. Elle n'est pas bornée par le périmètre
         *     d'administration : c'est `/admin/users` qui porte la liste du back-office et
         *     son filtrage. Ici, la permission — quelle que soit sa portée — ouvre la
         *     lecture, et rien de plus qu'une fiche publique n'en sort.
         * @description `Person[]`.
         */
        get: operations["lister"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/people/lookup": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * La personne qui porte cette adresse, si la plateforme la connaît.
         * @description `PersonLookup`, ou `null`. **La clé est l'adresse, et rien d'autre** — aucun appel de ce module ne rend l'annuaire. Même intention que la recherche d'organisations similaires : ne pas créer une seconde fiche pour quelqu'un qui existe déjà, ce qui est le défaut n° 1 de la v1 transposé de l'organisation à l'intervenant, et bien moins visible. `has_account` commande le verrouillage d'identité côté formulaire.
         */
        get: operations["depot_chercher_intervenant"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/people/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description `Person | null` — soi-même, ou `identity.person.read`. */
        get: operations["fiche"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/people/{id}/administered-events": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * **Jamais nul, toujours une valeur pleine** : les trois cas du périmètre se
         *     lisent sans ambiguïté côté site, et « aucun droit » ne se confond pas avec
         *     « réponse absente ».
         * @description `AdministeredEvents` — **jamais nul, toujours une valeur pleine**.
         */
        get: operations["perimetre"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/people/{id}/memberships": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les adhésions d'une personne. **Soi-même**, ou la permission de consultation
         *     des utilisateurs — décidée par la session, jamais par le paramètre.
         * @description `Membership[]` — adhésions vivantes : actives et en attente. Soi-même, ou `identity.person.read`.
         */
        get: operations["personne_adhesions"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/people/{id}/permissions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description `EffectivePermission[]`. */
        get: operations["permissions"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/people/{id}/roles": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description `RoleAssignmentView[]` — attributions en cours, avec le libellé du rôle, la cible de la portée résolue et qui l'a confiée. **Ce n'est pas `RoleAssignment`**, la ligne nue de la table : une pastille ne porte jamais un rôle sans sa PORTÉE, et la résoudre côté site demanderait une lecture par attribution. */
        get: operations["roles"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/platform/feature-flags": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description `ResolvedFeatureFlag[]` — chaque drapeau et son verdict POUR L'APPELANT. Le déploiement progressif est tranché par `platform.is_feature_enabled()`, jamais par le site : une seconde implémentation du calcul divergerait, et rendre `enabled_for` publierait les identifiants des personnes visées. Sans session, seul un déploiement à 100 % ouvre. */
        get: operations["platform_drapeaux"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposal-comments/{id}/resolution": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Marquer une demande de correction résolue.
         * @description `ResolveCommentPayload` → `ProposalComment`. **Qui peut le faire n'est écrit nulle part dans le modèle** — `resolved_by` est une simple clé étrangère —, et l'écart n° 35 a été tranché en A5 : le **déposant pose**, c'est lui qui sait qu'il a corrigé. Le verbe porte le sens ; le champ `resolved` du contrat est redondant et n'est pas cru. Rien n'est émis : l'état visible est le compteur de demandes ouvertes, relu à chaque affichage.
         */
        post: operations["propositions_resoudre_une_demande"];
        /**
         * Rouvrir une demande de correction — **le comité seul**.
         * @description `ResolveCommentPayload` → `ProposalComment`. **Le comité garde la main pour retirer** : un déposant qui pourrait retirer sa propre résolution ne changerait rien d'utile, mais un déposant qui retirerait celle du comité effacerait un arbitrage. C'est une règle d'autorisation, elle appartient à la permission et non à un formulaire (écart n° 35).
         */
        delete: operations["propositions_rouvrir_une_demande"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les dossiers d'une organisation, brouillons compris.
         * @description `Proposal[]` — les dossiers dont cette organisation est **porteuse principale**, brouillons compris. Deux voies d'accès, distinctes : adhésion **active** — tous les dossiers, toutes éditions confondues, une organisation n'administrant rien —, ou lecture générale, **bornée au périmètre d'administration**. Une personne sans l'une ni l'autre reçoit le refus d'une ressource inexistante. **Par la voie de l'organisation, les notes ne sortent pas** : moyenne, note pondérée et élimination partent vides (FR-077, écart n° 104).
         */
        get: operations["propositions_de_lorganisation"];
        put?: never;
        /**
         * Le premier enregistrement — **celui qui crée la ligne et attribue le numéro**.
         * @description `SaveDraftPayload` → `SaveDraftResult`. **Le dossier naît toujours en brouillon**, quel que soit l'état demandé : le garde d'état n'est posé que sur la mise à jour de `status`, et une insertion lui échappe (écart n° 96). Le numéro de dossier est attribué **à l'insertion** par le déclencheur, et l'écran peut donc l'annoncer dès la première frappe — c'est le même qui figurera sur la confirmation de dépôt. L'adresse d'URL est **dérivée par le service**, repliée quand le titre est vide et suffixée sur collision : le contrat ne la porte pas, et sans elle le tout premier enregistrement échouerait (écart n° 95).
         */
        post: operations["depot_creer_brouillon"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/assignments": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Confier une sélection à un membre du comité.
         * @description `AssignReviewerPayload` → `BulkResult`. Gardé par `event.call.manage` **sur l'édition de chaque dossier** — composer le comité et répartir sa charge sont le même geste, celui de qui tient la campagne (écart n° 48). Trois écarts nommés : **déjà confié**, **déporté** — le lui réattribuer effacerait une déclaration d'impartialité —, **introuvable**, qui couvre aussi le hors-périmètre et le hors-permission sans les distinguer. **Un événement `programme.review.assigned` par dossier**, jamais un pour le lot : un consommateur qui reçoit un lot doit le déplier lui-même, et son échec porterait alors sur douze effets au lieu d'un.
         */
        post: operations["propositions_confier_en_groupe"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/committee": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Qui peut recevoir une affectation, et ce qu'il porte déjà.
         * @description `ProposalFacet[]` — la composition du comité de l'appel : la valeur est la personne, le libellé son nom, **le décompte sa charge courante** sur cet appel, déports exclus. On ne confie pas douze dossiers de plus à quelqu'un qui en porte déjà vingt. Une édition sans appel rend une liste vide : il n'y a alors aucun comité, ce qui est un fait et non une erreur.
         */
        get: operations["propositions_comite"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/dashboard": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les lignes seules.
         * @description `ProposalDashboardRow[]` — la vue de pilotage **telle quelle**, sans facettes ni non-lus. Le titre y voyage **deux fois** : `title`, document multilingue brut résolu à l'affichage, et `title_text`, sa résolution française réservée au tri, au filtrage et à l'export. Les confondre rendrait une chaîne vide sans erreur. Les dossiers effacés sont exclus par la vue.
         */
        get: operations["propositions_pilotage"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/draft": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Le brouillon en cours de la personne, pour reprendre où elle s'est arrêtée.
         * @description `SaveDraftResult`, ou `null`. Le **plus récent** des brouillons de la personne : rien n'interdit d'en avoir deux — un par organisation —, et le contrat n'en rend qu'un. Ne rend jamais un dossier déposé : reprendre un dossier existant passe par la route de recomposition.
         */
        get: operations["depot_mon_brouillon"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/form-context": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Où l'on dépose aujourd'hui, et ce que l'organisation a déjà déposé.
         * @description `ProposalFormContext` — l'appel **réellement ouvert** de la plateforme (statut ET fenêtre, par `event.is_call_open()`), son édition, et le décompte du plafond de l'organisation, **ce brouillon exclu**. Le formulaire ne choisit pas son édition : il y en a au plus une qui reçoit. Le décompte reprend exactement les trois états que le déclencheur de recevabilité écarte — brouillon, retiré, non retenu —, sans quoi l'écran annoncerait un plafond que la base ne tient pas. Rend des champs nuls quand aucun appel n'est ouvert : l'écran l'annonce et s'arrête.
         */
        get: operations["depot_contexte_du_formulaire"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/list": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Tout l'écran de la liste, en une réponse.
         * @description `ProposalListScreen` — les lignes de `programme.v_proposal_dashboard`, **les sept facettes comptées sur ces mêmes lignes**, les dossiers que la personne connectée n'a jamais ouverts, le fuseau de l'édition, sa ville, l'échéance effective de l'appel et le nombre de revues attendues. Demandées à part, les facettes seraient mesurées à un autre instant et le « Retenu (17) » du filtre finirait par ne plus correspondre aux lignes affichées. **Ni pagination, ni tri, ni filtre serveur** : le contrat du front les garde à l'écran jusqu'au raccordement. **Périmètre vide → refus explicite**, jamais une liste vide ; **édition hors périmètre → le même refus qu'une édition inexistante**.
         */
        get: operations["propositions_ecran_de_liste"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Changer l'état d'une sélection.
         * @description `ChangeStatusPayload` → `BulkResult`. **L'autorisation est évaluée dossier par dossier** : une sélection peut traverser deux éditions, et le périmètre s'applique à chacune. Chaque dossier qui n'a pas suivi ressort avec son numéro et sa raison — transition non offerte, motif manquant, introuvable. Répondre « 6 dossiers traités » sans dire ce qu'il est advenu des six autres serait le défaut classique des actions de masse. **Un dossier hors périmètre rend le même écart qu'un dossier inexistant** : le refus ne dit pas à qui forge une sélection que le dossier existe ailleurs. Aucun événement n'est émis par le service : le déclencheur d'état les émet déjà, **un par dossier**.
         */
        post: operations["propositions_changer_letat_en_groupe"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/transitions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * La table des règles, telle quelle.
         * @description `ProposalTransitionRule[]` — les quatorze lignes de `programme.proposal_transitions_allowed`, **rendues telles quelles**. La machine à états est une DONNÉE : l'écran n'affiche que les actions déclarées, avec leur permission et leur exigence de motif, et ajouter un chemin en base ajoute une action sans toucher au code. **Globale et sans dossier** : ce qui est offert à une personne sur un dossier donné est une autre question, et une autre route.
         */
        get: operations["propositions_regles_de_transition"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Un dossier.
         * @description `Proposal` — le dossier tel que la table le porte, la colonne de recherche exclue. **Deux voies d'accès, un seul refus** : adhésion active à l'organisation porteuse, ou lecture générale dans le périmètre de l'édition ; inexistant, effacé, hors périmètre et organisation étrangère rendent tous le même 404. **Par la voie de l'organisation, les notes ne sortent pas** (FR-077, écart n° 104). `decision_reason` porte le motif de la **dernière** transition et rien de plus — une transition suivante l'écrase, et une transition sans motif l'efface : le motif d'une décision se lit dans le journal (écart n° 97).
         */
        get: operations["propositions_fiche"];
        /**
         * Les enregistrements suivants — **sans jamais toucher à l'état**.
         * @description `SaveDraftPayload` → `SaveDraftResult`. **Corriger n'est pas déposer** : `status` n'est pas dans la mise à jour, et le garde d'état n'est donc pas réveillé — un dossier en évaluation ne repart pas au comité parce qu'on a rectifié une faute de frappe. L'adresse d'URL **suit le titre tant que le dossier est en brouillon**, et se fige au dépôt : une adresse déjà communiquée ne change pas sous une correction. L'organisation porteuse vient de **la base**, jamais du corps.
         */
        put: operations["depot_enregistrer_brouillon"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/{id}/available-transitions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Ce qui est encore ouvert **à ce lecteur, sur ce dossier**.
         * @description Les transitions offertes **pour ce lecteur**, en une requête (R7). Une règle est retenue quand le lecteur est **porteur** — adhésion active — et que la règle l'y autorise, **ou** quand elle nomme une permission qu'il détient **sur l'édition du dossier**. La portée est celle de l'édition, pas la portée globale : c'est ce qui fait qu'un responsable détaché sur un webinaire ne décide pas sur la COP31. Le croisement se fait **au même instant que la lecture de l'état** — deux requêtes séparées offriraient une transition depuis un état déjà changé. Ce chemin existe parce que `/proposals/{id}/transitions` est déjà celui du journal (écart n° 101).
         */
        get: operations["propositions_transitions_offertes"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/{id}/comments": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Le fil des échanges, **filtré par visibilité à la source**.
         * @description `ProposalComment[]` — **filtré à la source, jamais après coup** : ce qui n'est pas envoyé ne peut pas fuiter. Côté comité, les messages du comité, ceux adressés au déposant et **ses propres** notes personnelles ; côté organisation, ce qui lui est adressé et rien d'autre. Le filtre est le même des deux côtés — l'écrire deux fois serait écrire deux filtres, et le second finirait par diverger.
         */
        get: operations["propositions_fil"];
        put?: never;
        /**
         * Écrire un message sur un dossier — **des deux côtés**.
         * @description `PostCommentPayload` ou `ReplyToCommentPayload` → `ProposalComment`. **Une seule route, deux appelants** : une réponse du déposant est **toujours** partagée et **jamais** une demande de correction — une organisation ne se demande pas des corrections à elle-même ; un message du comité porte sa visibilité, et une demande de correction y est **forcée en partagé** (écart n° 99), sans quoi elle bloquerait le dossier sans que le déposant sache pourquoi. **Seul un message partagé émet** `programme.comment.shared` : un message de comité ne sort pas du comité, par définition.
         */
        post: operations["propositions_ecrire_un_message"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/{id}/decision": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Décider — retenir, rejeter, remettre en évaluation, annuler.
         * @description `DecisionPayload` → `DecisionResult`. **Le service tente, il ne rejoue pas le graphe** : `programme.proposal_transitions_allowed` porte quatorze lignes, et le déclencheur en est l'arbitre — il refuse ce qui n'est pas déclaré, exige le motif quand la règle le dit, date la décision, journalise **et émet l'événement de domaine**. Le service n'émet donc rien : émettre à son tour produirait deux avis par décision, et le doublon ne se verrait qu'en production. **Les deux refus sortent en 200**, avec leur discriminant. `decision_reason` porte le motif de la dernière transition et rien de plus : le motif d'une décision se lit dans le journal.
         */
        post: operations["propositions_decider"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/{id}/documents": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les pièces du dossier.
         * @description `ProposalDocumentEntry[]` — chaque pièce, son objet stocké et son **adresse composée en base**. **L'adresse est nulle quand l'objet n'est pas servi** — quarantaine, purge, téléversement inachevé —, et c'est cette nullité qui commande l'avertissement plutôt que le bouton : le comité doit savoir qu'une pièce manque à son dossier, pas cliquer sur un lien mort.
         */
        get: operations["propositions_pieces"];
        put?: never;
        /**
         * Rattacher un objet **déjà stocké**.
         * @description Rattachement d'un objet **déjà stocké** → `ProposalDocument`. Le téléversement du fichier appartient à B6 : ce module reçoit un identifiant d'objet, jamais un fichier. Un objet inconnu ou supprimé rend `PROPOSAL_UNKNOWN_REFERENCE` **en nommant le champ** — la clé étrangère refuserait aussi, mais son message ne dirait pas lequel. Le titre par défaut est le nom du fichier d'origine : une pièce sans titre s'affiche « Document » dans une liste, et personne ne sait laquelle ouvrir.
         */
        post: operations["propositions_rattacher_une_piece"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/{id}/documents/{document_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /**
         * Détacher une pièce — **l'objet stocké demeure**.
         * @description Détachement d'une pièce. **L'objet stocké n'est pas détruit** : `media.assets` a son propre cycle de vie — suppression logique, date de purge, worker de purge — et un même objet peut être rattaché ailleurs. Le détruire ici effacerait la pièce d'un autre dossier sans le savoir. Le module ne détruit pas ce qu'il n'a pas créé.
         */
        delete: operations["propositions_detacher_une_piece"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/{id}/draft": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Rouvrir un dossier tel qu'il a été saisi.
         * @description `EditableProposal` — le dossier **recomposé en brouillon**, pas un `SELECT`. Trois conversions comptent : le créneau redevient une **heure murale dans le fuseau de l'ÉDITION** — saisi à 14:30 à Belém, il se rouvrirait à 11:30 pour qui corrige depuis Dakar, sans qu'aucune erreur ne soit levée ; les textes multilingues sont ramenés à leur français, **les textes provisoires effacés** — le formulaire n'affiche jamais « Dossier sans titre » (écart n° 102) ; chaque intervenant retrouve son **verrouillage d'identité** — une personne qui possède un compte détient sa fiche (écart n° 31). **Une seule implémentation pour les deux écrans** : deux recompositions divergeraient au premier champ ajouté.
         */
        get: operations["depot_rouvrir_un_dossier"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/{id}/file": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Le dossier vu par son déposant.
         * @description `ProposalFile` — le suivi du dossier, le **fil partagé** et l'historique champ par champ. Le fil est filtré **à la source** sur la visibilité partagée : les délibérations du comité n'y sont jamais, et les notes personnelles encore moins. C'est le **même** filtre que celui du comité — l'écrire deux fois serait écrire deux filtres, et le second finirait par diverger. `null` en 200 pour un dossier inexistant **ou porté par une organisation dont on n'est pas membre** : indiscernables, et ce n'est pas une panne.
         */
        get: operations["propositions_dossier_du_deposant"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/{id}/history": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * L'historique champ par champ.
         * @description `ProposalHistoryEntry[]` — par `programme.proposal_history()`, qui **écarte déjà les colonnes recalculées** : date de mise à jour, vecteur de recherche, compteur de vues. Les refaire ici ferait apparaître une modification à chaque affichage. **Réservé au back-office** : le déposant lit son propre historique par la route de son espace.
         */
        get: operations["propositions_historique"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/{id}/organizations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les organisations associées au dossier.
         * @description `ProposalOrganization[]` — **porteur compris**, dans l'ordre où le dossier les range. Une co-organisation dont `confirmed_at` est nulle est **annoncée, pas acquise** : elle engage un tiers, et le back-office l'affiche « en attente ».
         */
        get: operations["propositions_organisations"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/{id}/recusal": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Se déporter.
         * @description `RecusalPayload` → `ReviewAssignment`. **Le motif est obligatoire, et c'est le sujet** : la colonne existe pour tracer l'impartialité du comité, et un déport sans motif ne se relit pas six mois plus tard, quand une organisation conteste. **Le déport n'efface pas l'affectation, il la date** : la ligne demeure, et c'est elle qui interdit une réattribution silencieuse. Une seconde demande sur une affectation déjà déportée rend la même ligne, sans rien réécrire.
         */
        post: operations["propositions_se_deporter"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/{id}/resubmit": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Renvoyer un dossier corrigé.
         * @description `SaveDraftPayload` → `SubmitProposalResult`. **Ce n'est pas un dépôt** (écart n° 38) : la **fenêtre de l'appel ne s'applique pas** — le comité demande une correction à huit jours de la clôture, l'organisation répond après l'échéance, et lui opposer la clôture serait lui reprocher un délai qu'elle n'a pas choisi. Le déclencheur du modèle le sait déjà : il ne vérifie la fenêtre qu'au premier dépôt. **Le plafond, lui, s'applique** : il compte les dossiers en course, et un renvoi en remet un. Le geste est porté par le **chemin**, jamais déduit de l'état — déduire ferait franchir la clôture à un dossier corrigé par la route de dépôt, sans que personne l'ait décidé.
         */
        post: operations["depot_renvoyer_un_dossier"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/{id}/review-desk": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Toute la fiche, en une réponse.
         * @description `ReviewDeskScreen` — onze tables en une réponse : le dossier, son édition, son appel, sa grille, ses organisations avec leur historique de participation, ses intervenants, ses pièces, son journal, son historique champ par champ, l'avancement nominatif du comité, ma revue, les échanges que **ce** lecteur a le droit de voir, et les revues des pairs **quand j'ai le droit de les lire**. **Le voile de l'aveugle n'est pas un filtre** : quand il est baissé — appel en aveugle, lecteur affecté, sa revue non déposée —, la requête qui lit les revues des pairs **n'est pas exécutée**. Le décompte l'est : compter n'ancre pas, lire si. **Cette lecture écrit** : elle pose l'accusé de lecture, et `first_visit` dit l'état d'AVANT la visite.
         */
        get: operations["propositions_fiche_devaluation"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/{id}/reviews": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Enregistrer ou déposer sa revue.
         * @description `SaveReviewPayload` → `SaveReviewResult`. **Noter exige une affectation non déportée** : rien ne lie la permission à l'affectation en base, et un membre du comité pourrait sinon noter n'importe quel dossier de son édition. Lire, en revanche, reste permis — les deux règles sont décorrélées. **Une note absente n'est pas une note à zéro** : zéro sur un critère éliminatoire disqualifie le dossier. **La consolidation est appelée dans la même transaction**, et les agrégats rendus sont **relus en base** — sans cet appel, le classement du comité serait faux sans qu'aucune erreur ne le signale.
         */
        put: operations["propositions_noter"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/{id}/speakers": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les intervenants annoncés.
         * @description `ProposalSpeaker[]`, dans l'ordre annoncé. **Les deux instantanés voyagent** — fonction et organisation **au moment de cette activité** : une personne change d'employeur, et l'archive d'une COP passée ne doit pas être réécrite.
         */
        get: operations["propositions_intervenants"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/{id}/submit": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Le dépôt.
         * @description `SaveDraftPayload` → `SubmitProposalResult`. **Les trois refus de recevabilité sortent en 200**, avec leur valeur : l'échéance pour un appel clos, le plafond pour un quota atteint. Ils sont classés **avant** l'écriture parce que le déclencheur ne les rend que dans une phrase française, et parce qu'un même code d'erreur PostgreSQL sert aux quatre causes possibles. Le déclencheur reste le dernier mot : une course est **reclassée**, jamais lue au texte. Le brouillon est enregistré **avant** toute décision — si l'appel a fermé entre le chargement et le clic, l'organisation ne perd pas en plus ce qu'elle venait de saisir. La réponse porte le nombre de revues attendues et la date d'annonce, **lus sur l'appel**.
         */
        post: operations["depot_deposer"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/{id}/themes": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les thématiques du dossier, **prêtes à afficher**.
         * @description Les pastilles de `reference.term_badges()` — libellé traduit et couleur venus de `reference.taxonomy_terms`, **où un administrateur les modifie**. N'exposer que les codes forcerait l'écran à recharger la taxonomie : c'est ainsi que les libellés se sont retrouvés figés dans le frontend de la v1.
         */
        get: operations["propositions_thematiques"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/proposals/{id}/transitions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Le journal d'un dossier — **c'est lui qui porte chaque motif**.
         * @description `ProposalTransition[]` — le journal, du plus récent au plus ancien. **C'est lui qu'un écran doit lire pour un motif** : la colonne `decision_reason` du dossier ne garde que celui de la dernière transition, et une transition suivante l'écrase — y compris quand elle n'en demande aucun, auquel cas elle l'efface (écart n° 97). Un écran qui lirait la colonne afficherait « motif de la décision » sur un dossier remis en course. **Accès au dossier** : adhésion active à l'organisation porteuse, ou lecture générale dans le périmètre — deux voies distinctes, un seul refus.
         */
        get: operations["propositions_journal"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/ready": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Vivacité. **Le pool est réellement sollicité** : un processus qui répond
         *     pendant que sa base est injoignable n'est pas prêt, et le dire en 200 ferait
         *     router du trafic vers un serveur qui ne peut rien servir.
         * @description Vivacité : le processus répond et son pool de connexions est ouvert. Aucune autorisation, aucune divulgation — c'est l'orchestrateur qui la lit.
         */
        get: operations["ready"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/reference/countries": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * **Les pays inactifs sont écartés** : ils ne sont pas supprimés — des fiches
         *     anciennes les référencent — mais on ne les propose plus au choix.
         * @description `Country[]` — les pays actifs, ordonnés par leur nom français. Sans session : un formulaire d'inscription en a besoin avant qu'un compte existe. Les libellés partent en `i18n_text` entier, le site les résout dans sa locale.
         */
        get: operations["reference_pays"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/reference/locales": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description `Locale[]` — les langues actives, dans leur ordre d'affichage. */
        get: operations["reference_langues"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/reference/taxonomies/{code}/terms": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * **Une taxonomie inconnue rend une liste vide, pas 404.** L'écran qui demande
         *     « organization_type » attend une liste de choix : lui rendre une erreur le
         *     ferait afficher une panne pour un vocabulaire qu'un administrateur n'a
         *     simplement pas encore garni.
         * @description `TaxonomyTerm[]` — les termes ACTIFS d'une taxonomie, dans leur ordre d'affichage. Les libellés et les couleurs viennent de la base, **où un administrateur les modifie** : les figer dans le site est le défaut n° 1 de la v1.
         */
        get: operations["reference_termes"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/registrations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * La liste **nominative** des inscrits d'une séance.
         * @description `RegistrationRow[]` — la liste **nominative**, avec la personne et son organisation. Elle exige `programme.registration.manage` **sur l'édition de la séance** : le rôle de programmation ne la détient pas, et une chargée de programmation compose donc la grille sans pouvoir ouvrir cette liste. Ce n'est pas une fatalité du code — c'est une ligne de la table des droits, modifiable au back-office.
         */
        get: operations["inscriptions_liste_nominative"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/registrations/mine": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * « Mes inscriptions », annulations comprises.
         * @description `Registration[]` — ce à quoi la personne **connectée** est inscrite, annulations comprises. L'identifiant de personne que le front envoie encore est **ignoré** : l'API lit sa propre session.
         */
        get: operations["inscriptions_les_miennes"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/registrations/{id}/cancel": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Annuler son inscription.
         * @description `{ reason? }` → `CancelRegistrationResult`. L'annulation **promeut exactement le nombre de places libérées** — zéro ou une —, dans la même transaction et sous le même verrou : le contrôle de capacité de la base ne porte que sur l'insertion, et promouvoir davantage ferait dépasser la jauge sans un mot. Annuler une inscription **en attente** ne promeut personne : elle n'occupait aucune place. Elle est ouverte à **l'inscrit lui-même** ou à qui gère les inscriptions de l'édition.
         */
        post: operations["inscriptions_annuler"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/registrations/{id}/join": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Rejoindre — la **première** présence.
         * @description `{ joined_at }` — la **première** présence, écrite une seule fois par la fonction du modèle : un second clic ne l'écrase pas, et c'est ce qui donne un taux de participation réel. Réservée à **l'inscrit lui-même**.
         */
        post: operations["inscriptions_rejoindre"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/schedule": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * La programmation d'une édition.
         * @description `PublicScheduleRow[]` — `programme.v_public_schedule`, **telle quelle**, et **sans session**. Une ligne = un bloc du calendrier : salle, organisation avec son sigle et son pays, journées spéciales, thématiques avec libellé et couleur, image de couverture — celle de la séance, **à défaut celle du dossier d'origine** —, état temporel calculé en base, nombre d'inscrits. Une édition dont le programme n'est pas paru rend une liste **vide**, jamais une erreur. **`event_id` est facultative** : absente, ce sont les séances `upcoming` et `ongoing` de TOUTES les éditions, dans l'ordre du temps — ce que compose l'accueil, qui n'a pas d'édition à nommer. La lecture est alors plafonnée.
         */
        get: operations["programmation_publique"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/sessions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les séances d'une édition, placées ou non.
         * @description `PlannerSession[]` — les séances de l'édition, **placées ou non**. Une séance dont la salle est nulle est au panneau « à placer » ; c'est la seule chose qui l'y range, et elle existe bel et bien. Chaque ligne porte tout ce qu'un bloc affiche, **déjà joint** : salle, organisation avec son sigle et son code pays, numéro de dossier, note consolidée, durée et créneau souhaités, contraintes de programmation, journées spéciales, thématiques avec leur libellé et leur couleur, nombre d'intervenants.
         */
        get: operations["seances_liste"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/sessions/conflicts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Les chevauchements — **signalés, jamais bloqués**.
         * @description `ScheduleConflict[]` — `programme.detect_conflicts()`, **telle quelle**, sans filtrer ni requalifier les gravités. `blocking` : matériellement impossible — un seul stand par édition, un seul direct sur la plateforme, une salle physique à la fois. `warning` : gênant mais possible — un intervenant attendu à deux endroits, une organisation programmée deux fois ; l'équipe juge. **Aucun de ces conflits n'empêche une écriture** : le seul garde-fou dur est la publication du programme.
         */
        get: operations["seances_conflits"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/sessions/{id}/broadcast": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Marquer une séance diffusée, avec son canal.
         * @description `SessionBroadcastPayload` → `PlannerMutationResult`. **Le canal EST saisissable** quand la diffusion est activée : la base ne pose le canal par défaut de l'édition que lorsque la colonne est nulle — elle complète, elle n'écrase jamais —, et l'écran laisse le choix quand l'édition a plusieurs canaux. **Retirer la diffusion en désignant un canal est refusé** : c'est le seul cas où la base efface une valeur choisie sans le dire. Deux directs simultanés **s'écrivent** et remontent en gravité bloquante : la règle « un seul direct » est signalée, jamais imposée à l'écriture.
         */
        put: operations["seances_diffusion"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/sessions/{id}/organizations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description `SessionOrganization[]` — le porteur principal et ses co-organisations. La ligne du porteur est posée **par déclencheur** et jamais par le service. */
        get: operations["seances_organisations"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/sessions/{id}/registration-form": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Le formulaire applicable à une séance.
         * @description `{ form, fields }` — le formulaire **applicable** : celui de la séance, à défaut celui de son édition, à défaut celui de la plateforme. **Lecture publique** : l'écran d'inscription s'ouvre avant qu'on se connecte. Seuls les champs **actifs** sont rendus, dans leur ordre d'affichage, et les options d'un champ adossé à une taxonomie sont **résolues avec leur libellé traduit** — n'exposer que les codes forcerait l'écran à recharger la taxonomie, et c'est ainsi que les libellés se sont retrouvés figés dans le frontend de la v1.
         */
        get: operations["inscriptions_formulaire"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/sessions/{id}/registrations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * S'inscrire à une séance.
         * @description `SessionRegisterPayload` → `RegistrationResult`. **Six issues, toutes en 200** : inscrit, placé en liste d'attente avec sa position, déjà inscrit, complet avec le nombre de places, clos avec son échéance, pas encore ouvert avec sa date. Ce sont des issues normales d'une tentative bien formée — une personne peut arriver une minute après la clôture. Les réponses sont validées contre le formulaire **résolu**, avant toute écriture, et une clé inconnue est **refusée** plutôt qu'ignorée. Une réponse à un champ marqué sensible exige un consentement, dont la preuve est écrite dans la même transaction. **Sans session**, l'inscription n'aboutit que si le formulaire admet l'anonyme, et l'identité vient de champs dédiés — jamais des réponses.
         */
        post: operations["inscriptions_sinscrire"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/sessions/{id}/reminder-rule": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * La règle applicable à une séance, avec son origine.
         * @description `ApplicableReminderRule | null` — la règle qui gouverne **effectivement** cette séance, et `null` quand aucune ne s'applique.
         *
         *     **Une règle de séance REMPLACE celle de son édition**, sans cumul. La réponse porte donc l'**origine** — `session` ou `event` — et l'identifiant dont elle vient : sans elle, une règle de séance à deux décalages ne se distingue pas d'une règle d'édition qu'on aurait tronquée, et la non-cumulation cesse d'être vérifiable de l'extérieur.
         *
         *     Même garde que le calendrier : adhésion active, ou droit de gérer les inscriptions de l'édition.
         */
        get: operations["engagement_regle_de_rappel_applicable"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/sessions/{id}/reminders": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Le calendrier des rappels d'une séance.
         * @description `{ slots, has_rule }` — **une ligne par (décalage, canal), et pas un nom.** Quarante inscrits et quatre décalages rendent **quatre** lignes portant chacune quarante destinataires, jamais cent soixante : l'organisation qui anime a droit au NOMBRE de destinataires, pas à leur identité. La garantie est portée par la signature de la fonction du modèle, pas par la discipline d'un appelant.
         *
         *     Les lignes sont rangées **du décalage le plus lointain au plus proche**, en minutes : `'1 day'` et `'24 hours'` sont le même intervalle pour la base et deux chaînes différentes pour un écran, ce qui suffirait à afficher deux fois le même rappel.
         *
         *     L'état d'une ligne est celui de la ligne **la moins avancée** du groupe : une seule personne qui attend encore son courriel suffit à dire « en attente ». « Parti » ne se dit pas tant qu'il reste quelqu'un.
         *
         *     **`has_rule` distingue « aucune règle » de « tout est parti »** : une liste vide muette se confond avec un envoi réussi, et les deux situations demandent des mots différents à l'écran.
         *
         *     Gardé par l'**adhésion active** à l'organisation qui anime, ou par `programme.registration.manage` sur l'édition de la séance — jamais par un périmètre d'administration, une organisation n'administrant rien.
         */
        get: operations["engagement_calendrier_des_rappels"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/sessions/{id}/schedule": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Placer, déplacer, redimensionner, retirer.
         * @description `ScheduleSessionPayload` → `PlannerMutationResult`. **Une seule écriture pour quatre gestes** : la base n'en distingue pas, ce sont `room_id`, `starts_at` et `ends_at`. Une salle nulle **renvoie la séance au panneau** — ce n'est pas une suppression. **Jamais refusée pour chevauchement** : poser deux séances sur le même créneau réussit, et la réponse porte le conflit. La journée de rattachement est facultative : non fournie, elle est **remise à nul** pour que la base la redéduise, sans quoi une séance déplacée du 12 au 14 novembre resterait rangée au 12, en silence. La réponse porte les conflits de **toute l'édition** : un déplacement peut résoudre le conflit d'un bloc situé à l'autre bout de la semaine.
         */
        put: operations["seances_placer"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/sessions/{id}/speakers": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description `SessionSpeaker[]` — les intervenants du jour, recopiés du dossier à la programmation puis modifiables : ceux qui étaient annoncés ne sont pas toujours ceux qui viennent. */
        get: operations["seances_intervenants"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/sessions/{id}/tracks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description `SessionTrack[]` — les journées spéciales auxquelles la séance est rattachée, **avec qui les a posées**. La composition d'un fil est un choix éditorial qu'il arrive d'expliquer à une organisation qui s'étonne de ne pas y figurer. */
        get: operations["seances_fils"];
        /**
         * Rattacher aux journées spéciales.
         * @description `SessionTracksPayload` → `PlannerMutationResult`. **Manuel et indépendant de la date** : toutes les activités du 12 novembre ne relèvent pas de la « Journée finance durable ». La liste envoyée **remplace** la précédente, et la base retient qui a rattaché quoi. Un fil d'une **autre édition** est refusé par un déclencheur du modèle, traduit ici en code stable.
         */
        put: operations["seances_rattacher"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        AnnulationDemandee: {
            reason?: string | null;
        };
        /**
         * ApiError
         * @description Corps d'erreur unique de l'API. Le front branche sur `code`, jamais sur `message`.
         *
         *     Catalogue des codes stables :
         *
         *     - `VALIDATION_FAILED` (422) — La requête contient une valeur invalide.
         *     - `UNAUTHENTICATED` (401) — Votre session a expiré. Veuillez vous reconnecter.
         *     - `FORBIDDEN` (403) — Vous n'avez pas les droits nécessaires pour cette action.
         *     - `NOT_FOUND` (404) — La ressource demandée est introuvable.
         *     - `CONFLICT` (409) — Cette action entre en conflit avec l'état actuel de la donnée.
         *     - `PAYLOAD_TOO_LARGE` (413) — La requête dépasse la taille autorisée.
         *     - `INTERNAL` (500) — Une erreur interne est survenue. L'incident a été enregistré.
         *     - `SERVICE_UNAVAILABLE` (503) — Le service est momentanément indisponible.
         *     - `IDENTITY_SESSION_EXPIRED` (401) — Votre session a expiré. Veuillez vous reconnecter.
         *     - `IDENTITY_SESSION_REVOKED` (401) — Cette session a été fermée. Veuillez vous reconnecter.
         *     - `IDENTITY_REFRESH_REUSED` (401) — Par sécurité, toutes vos sessions ont été fermées. Veuillez vous reconnecter.
         *     - `IDENTITY_ORIGIN_REJECTED` (403) — Requête refusée : origine non autorisée.
         *     - `IDENTITY_PASSWORD_TOO_WEAK` (422) — Le mot de passe doit compter au moins 8 caractères, dont une majuscule et une minuscule.
         *     - `IDENTITY_EMAIL_ALREADY_USED` (409) — Cette adresse est déjà utilisée par une autre personne.
         *     - `IDENTITY_ACCOUNT_ALREADY_EXISTS` (409) — Cette personne a déjà un compte avec mot de passe.
         *     - `IDENTITY_ROLE_WINDOW_INVALID` (422) — La date de fin doit être postérieure à la prise d'effet.
         *     - `IDENTITY_ROLE_SCOPE_MISMATCH` (422) — Une portée globale ne vise aucune cible ; une portée ciblée en exige une.
         *     - `IDENTITY_ROLE_REVOCATION_INVALID` (422) — Un motif de retrait ne peut pas être posé sur une attribution en cours.
         *     - `IDENTITY_UNKNOWN_REFERENCE` (422) — La valeur choisie n'existe pas.
         *     - `IDENTITY_PRIVACY_WRONG_ACTION` (422) — L'anonymisation ne répond qu'à une demande d'effacement.
         *     - `ORG_NOT_MANAGER` (403) — Seul un référent de cette organisation peut effectuer cette action.
         *     - `ORG_MEMBERSHIP_IS_INVITATION` (422) — Cette adhésion est une invitation : elle attend la réponse de la personne, pas la vôtre.
         *     - `ORG_MEMBERSHIP_NOT_PENDING` (422) — Cette adhésion n'attend plus de décision.
         *     - `ORG_LAST_MANAGER` (422) — Cette organisation n'aurait plus aucun référent. Désignez un remplaçant d'abord.
         *     - `ORG_MERGE_FIELD_NOT_ARBITRABLE` (422) — L'adresse de la fiche absorbée ne peut pas être reprise : elle reste la sienne, et c'est ce qui fait que ses anciens liens continuent de fonctionner.
         *     - `ORG_MERGE_GLOBAL_SCOPE_REQUIRED` (403) — La fusion de deux organisations exige des droits sur l'ensemble de la plateforme.
         *     - `ORG_MERGE_SAME_ORGANIZATION` (422) — Une organisation ne peut pas être fusionnée avec elle-même.
         *     - `ORG_DOMAIN_VERIFICATION_REQUIRED` (422) — Un rattachement automatique exige un domaine vérifié.
         *     - `ORG_NAME_IS_DERIVED` (422) — Le nom légal et le sigle suivent la fiche : ils ne se retirent pas à la main.
         *     - `ORG_UNKNOWN_REFERENCE` (422) — La valeur choisie n'existe pas.
         *     - `ORG_INVITATION_NOT_YOURS` (403) — Cette invitation ne vous est pas adressée.
         *     - `EVENT_GLOBAL_SCOPE_REQUIRED` (403) — La création d'une édition exige des droits sur l'ensemble de la plateforme.
         *     - `EVENT_CRITERION_HAS_SCORES` (422) — Ce critère porte déjà des notes : le retirer effacerait l'argumentaire des évaluations rendues.
         *     - `EVENT_UNKNOWN_REFERENCE` (422) — La valeur choisie n'existe pas.
         *     - `PROPOSAL_NOT_EDITABLE` (422) — Ce dossier n'est plus modifiable. Vous pouvez en déposer un nouveau.
         *     - `PROPOSAL_SPEAKER_IDENTITY_LOCKED` (422) — Cette personne possède un compte : son identité lui appartient et ne se modifie pas depuis un dossier.
         *     - `PROPOSAL_REVIEW_NOT_ASSIGNED` (403) — Ce dossier ne vous est pas confié : vous pouvez le lire, pas le noter.
         *     - `PROPOSAL_UNKNOWN_TERM` (422) — Cette thématique n'existe pas.
         *     - `PROPOSAL_TEXT_TOO_LONG` (422) — Ce texte dépasse la longueur autorisée.
         *     - `PROPOSAL_UNKNOWN_REFERENCE` (422) — La valeur choisie n'existe pas.
         *     - `SESSION_DERIVED_FIELD` (422) — Cette valeur est déduite par le système : elle ne se saisit pas.
         *     - `SESSION_UNKNOWN_REFERENCE` (422) — La valeur choisie n'existe pas, ou n'appartient pas à cette édition.
         *     - `SESSION_TRACK_EVENT_MISMATCH` (422) — Cette journée spéciale appartient à une autre édition.
         *     - `REGISTRATION_NOT_ACCEPTED` (422) — Cette séance ne prend pas d'inscription.
         *     - `REGISTRATION_ANSWER_INVALID` (422) — Cette réponse n'est pas valide.
         *     - `REGISTRATION_CONSENT_REQUIRED` (422) — Cette question porte une donnée personnelle sensible : votre accord est nécessaire pour y répondre.
         *     - `REGISTRATION_ACCOUNT_REQUIRED` (401) — L'inscription à cette séance demande un compte. Veuillez vous connecter.
         *     - `REGISTRATION_LOCKED` (422) — Cette inscription ne peut plus être modifiée. Elle ne vous engage plus à rien.
         *     - `MEDIA_QUOTA_EXCEEDED` (422) — L'espace de stockage de cette organisation est atteint.
         *     - `MEDIA_MIME_NOT_ALLOWED` (422) — Ce type de fichier n'est pas accepté pour ce rôle.
         *     - `MEDIA_TOO_LARGE` (413) — Ce fichier dépasse la taille acceptée pour ce rôle.
         *     - `MEDIA_ASPECT_RATIO` (422) — Les dimensions de cette image ne correspondent pas à la forme attendue.
         *     - `MEDIA_ROLE_NOT_DECLARED` (422) — Ce rôle n'est pas prévu pour ce type de contenu.
         *     - `MEDIA_ROLE_EXCLUSIVE` (409) — Ce rôle n'accepte qu'un seul fichier ; remplacez celui qui s'y trouve.
         *     - `MEDIA_ASSET_NOT_SERVABLE` (422) — Ce fichier n'est pas exploitable : il est supprimé ou en quarantaine.
         *     - `MEDIA_ALT_TEXT_REQUIRED` (422) — Décrivez cette image en une phrase : elle ne pourra pas s'afficher sans.
         *     - `MEDIA_ASSET_IN_USE` (409) — Ce fichier est encore utilisé ; il ne peut pas être supprimé.
         *     - `MEDIA_UPLOAD_INCOMPLETE` (400) — L'envoi du fichier s'est interrompu.
         *     - `MEDIA_STORAGE_UNAVAILABLE` (503) — Le stockage des fichiers est momentanément indisponible.
         *     - `ENGAGEMENT_REMINDER_OFFSETS_INVALID` (422) — Les délais de rappel doivent être compris entre un et huit valeurs, toutes positives.
         *     - `ENGAGEMENT_REMINDER_SCOPE_INVALID` (422) — Une règle de rappel vise une édition ou une séance, jamais les deux.
         *     - `ENGAGEMENT_TEMPLATE_VARIABLE_UNKNOWN` (422) — Ce modèle utilise une variable que ce type de message ne fournit pas.
         *     - `ENGAGEMENT_TEMPLATE_VERSION_UNKNOWN` (404) — Cette révision de modèle n'existe pas.
         *     - `ENGAGEMENT_NOTIFICATION_TYPE_UNKNOWN` (422) — Ce type de notification n'existe pas ou n'est plus actif.
         *     - `LIVE_INCIDENT_SCOPE_TARGET_MISMATCH` (422) — La portée choisie et la cible renseignée ne correspondent pas : une portée vise exactement une cible, et la portée globale n'en vise aucune.
         *     - `LIVE_INCIDENT_WINDOW_INVALID` (422) — La fin d'affichage doit être postérieure au début.
         *     - `LIVE_INCIDENT_NOT_PUBLISHED` (422) — Ce message n'a jamais été publié : il n'y a rien à retirer.
         */
        ApiError: {
            /**
             * @description Code stable. Le renommer est un changement majeur.
             * @enum {string}
             */
            code: "VALIDATION_FAILED" | "UNAUTHENTICATED" | "FORBIDDEN" | "NOT_FOUND" | "CONFLICT" | "PAYLOAD_TOO_LARGE" | "INTERNAL" | "SERVICE_UNAVAILABLE" | "IDENTITY_SESSION_EXPIRED" | "IDENTITY_SESSION_REVOKED" | "IDENTITY_REFRESH_REUSED" | "IDENTITY_ORIGIN_REJECTED" | "IDENTITY_PASSWORD_TOO_WEAK" | "IDENTITY_EMAIL_ALREADY_USED" | "IDENTITY_ACCOUNT_ALREADY_EXISTS" | "IDENTITY_ROLE_WINDOW_INVALID" | "IDENTITY_ROLE_SCOPE_MISMATCH" | "IDENTITY_ROLE_REVOCATION_INVALID" | "IDENTITY_UNKNOWN_REFERENCE" | "IDENTITY_PRIVACY_WRONG_ACTION" | "ORG_NOT_MANAGER" | "ORG_MEMBERSHIP_IS_INVITATION" | "ORG_MEMBERSHIP_NOT_PENDING" | "ORG_LAST_MANAGER" | "ORG_MERGE_FIELD_NOT_ARBITRABLE" | "ORG_MERGE_GLOBAL_SCOPE_REQUIRED" | "ORG_MERGE_SAME_ORGANIZATION" | "ORG_DOMAIN_VERIFICATION_REQUIRED" | "ORG_NAME_IS_DERIVED" | "ORG_UNKNOWN_REFERENCE" | "ORG_INVITATION_NOT_YOURS" | "EVENT_GLOBAL_SCOPE_REQUIRED" | "EVENT_CRITERION_HAS_SCORES" | "EVENT_UNKNOWN_REFERENCE" | "PROPOSAL_NOT_EDITABLE" | "PROPOSAL_SPEAKER_IDENTITY_LOCKED" | "PROPOSAL_REVIEW_NOT_ASSIGNED" | "PROPOSAL_UNKNOWN_TERM" | "PROPOSAL_TEXT_TOO_LONG" | "PROPOSAL_UNKNOWN_REFERENCE" | "SESSION_DERIVED_FIELD" | "SESSION_UNKNOWN_REFERENCE" | "SESSION_TRACK_EVENT_MISMATCH" | "REGISTRATION_NOT_ACCEPTED" | "REGISTRATION_ANSWER_INVALID" | "REGISTRATION_CONSENT_REQUIRED" | "REGISTRATION_ACCOUNT_REQUIRED" | "REGISTRATION_LOCKED" | "MEDIA_QUOTA_EXCEEDED" | "MEDIA_MIME_NOT_ALLOWED" | "MEDIA_TOO_LARGE" | "MEDIA_ASPECT_RATIO" | "MEDIA_ROLE_NOT_DECLARED" | "MEDIA_ROLE_EXCLUSIVE" | "MEDIA_ASSET_NOT_SERVABLE" | "MEDIA_ALT_TEXT_REQUIRED" | "MEDIA_ASSET_IN_USE" | "MEDIA_UPLOAD_INCOMPLETE" | "MEDIA_STORAGE_UNAVAILABLE" | "ENGAGEMENT_REMINDER_OFFSETS_INVALID" | "ENGAGEMENT_REMINDER_SCOPE_INVALID" | "ENGAGEMENT_TEMPLATE_VARIABLE_UNKNOWN" | "ENGAGEMENT_TEMPLATE_VERSION_UNKNOWN" | "ENGAGEMENT_NOTIFICATION_TYPE_UNKNOWN" | "LIVE_INCIDENT_SCOPE_TARGET_MISMATCH" | "LIVE_INCIDENT_WINDOW_INVALID" | "LIVE_INCIDENT_NOT_PUBLISHED";
            /** @description Message français, affichable tel quel. */
            message: string;
            /** @description Champ fautif, quand le refus en désigne un. */
            field?: string;
            /** @description Identifiant de requête, à citer dans un signalement d'incident. */
            request_id?: string;
        };
        /**
         * @description Ce qu'un archivage vise. La liste est **exigée** : « tout archiver » n'est
         *     pas un geste qu'on fait par mégarde.
         */
        ArchivagePayload: {
            ids: string[];
        };
        AttachmentAssignment: {
            role: string;
            /**
             * Format: uuid
             * @description **Nul = retirer.** Le rôle est vidé, et l'objet stocké demeure.
             */
            asset_id?: string | null;
            alt_text_override?: unknown;
        };
        /**
         * @description L'écriture de remplacement, en lot — `AttachmentAssignment[]`.
         *
         *     # Ce que « remplacement » veut dire, exactement
         *
         *     **Chaque rôle nommé dans la liste est vidé puis regarni**, dans l'ordre où
         *     ses affectations apparaissent. Un rôle **absent** de la liste n'est pas
         *     touché.
         *
         *     C'est ce qui permet aux trois déclinaisons d'une édition de partir en un
         *     geste, à une valeur nulle d'en retirer une **sans toucher aux deux autres**,
         *     et à une galerie de se réordonner par un simple renvoi de la même liste dans
         *     un autre ordre — sans qu'aucune route de réordonnancement existe.
         */
        AttachmentBatch: {
            owner_schema: string;
            owner_table: string;
            /** Format: uuid */
            owner_id: string;
            assignments: components["schemas"]["AttachmentAssignment"][];
        };
        /** @description Ce qu'un ajout déclare — `AttachmentPayload`. */
        AttachmentPayload: {
            owner_schema: string;
            owner_table: string;
            /** Format: uuid */
            owner_id: string;
            role: string;
            /** Format: uuid */
            asset_id: string;
            /**
             * Format: int32
             * @description L'ordre voulu dans un rôle multiple. Absent : à la suite.
             */
            sort_order?: number | null;
            /**
             * @description Le texte alternatif **propre à cet usage**. Il prime sur celui de
             *     l'objet et **ne le modifie pas** : un objet dédupliqué sert plusieurs
             *     fiches, et le texte pertinent n'y est pas le même (FR-040).
             */
            alt_text_override?: unknown;
        };
        /**
         * @description À qui l'annonce s'adresse.
         *
         *     **Deux périmètres et pas un de plus** : toute la plateforme, ou les inscrits
         *     d'une édition. Un troisième — « les référents d'organisation », « les
         *     négociateurs » — demanderait une définition que rien ne porte aujourd'hui, et
         *     l'inventer produirait une liste que personne n'aurait validée.
         */
        Audience: {
            /** @enum {string} */
            kind: "all";
        } | {
            /** Format: uuid */
            event_id: string;
            /** @enum {string} */
            kind: "event";
        };
        BroadcastPayload: {
            /** @description Multilingue, comme tout texte que l'écran affiche. */
            title: unknown;
            body?: unknown;
            /** @description **Chemin relatif**, jamais une adresse absolue. */
            link_path?: string | null;
            audience: components["schemas"]["Audience"];
        };
        /**
         * @description **`from_event_id` est le seul champ du corps qui ne soit pas une colonne** :
         *     c'est l'édition **depuis laquelle** on agit, et donc l'ancre du contrôle de
         *     périmètre.
         */
        CreateIncidentPayload: components["schemas"]["IncidentPayload"] & {
            /** Format: uuid */
            from_event_id: string;
        };
        HealthIndicator: {
            code: string;
            label: string;
            domain: string;
            /** Format: int64 */
            value: number;
            /** Format: int64 */
            warning_threshold: number;
            /** Format: int64 */
            critical_threshold: number;
            severity: string;
            detail: Record<string, never>;
        };
        /**
         * @description Le corps commun aux deux écritures de contenu.
         *
         *     `publish` est **à part du reste** : enregistrer et publier sont deux actes
         *     distincts en base — `live.publish_incident()` horodate, attribue et émet. Un
         *     brouillon se relit avant de parler à toute une COP.
         */
        IncidentPayload: {
            scope: string;
            /** Format: uuid */
            event_id?: string | null;
            /** Format: uuid */
            event_day_id?: string | null;
            /** Format: uuid */
            session_id?: string | null;
            /** Format: uuid */
            organization_id?: string | null;
            incident_kind_code: string;
            severity: string;
            title?: unknown;
            message: unknown;
            action_url?: string | null;
            is_dismissible: boolean;
            /** Format: date-time */
            display_from: string;
            /** Format: date-time */
            display_until?: string | null;
            publish: boolean;
        };
        Invite: {
            email: string;
            first_name: string;
            last_name: string;
            civility?: string | null;
        };
        /** @description Ce que le site remonte de ce que le fournisseur a dit d'un courriel. */
        MailEvent: {
            /**
             * @description **L'identifiant que l'API a remis au site avec le message** — c'est lui
             *     qui relie l'annonce à sa trace. Le contrat d'envoi du noyau ne rapporte
             *     aucun identifiant de fournisseur : s'y reposer laisserait toute annonce
             *     sans trace à mettre à jour.
             */
            message_id: string;
            /** @description Celui du fournisseur, conservé pour corréler ses propres journaux. */
            provider_message_id?: string | null;
            /** @description `delivered`, `bounced`, `complained`, `failed`. */
            status: string;
            /** @description `hard`, `soft`, `block` — seulement pour un rebond. */
            bounce_kind?: string | null;
            detail?: string | null;
        };
        /** @description Ce qu'un marquage vise. Sans `ids` : tout. */
        MarquagePayload: {
            ids?: string[] | null;
        };
        /** @description L'écriture d'une préférence. */
        NotificationPreferencePayload: {
            type_code: string;
            channel: string;
            is_enabled: boolean;
        };
        OperationalHealth: {
            /** @description La pire gravité rencontrée : `ok`, `attention` ou `critique`. */
            status: string;
            /** Format: date-time */
            measured_at: string;
            indicators: components["schemas"]["HealthIndicator"][];
        };
        PlafondPayload: {
            /** Format: int64 */
            max_bytes: number;
            /** Format: int32 */
            max_files: number;
            note?: string | null;
        };
        /** @description Ce qu'un aperçu demande. */
        PreviewPayload: {
            /**
             * Format: int32
             * @description Absente : la révision servie, ou la plus récente si aucune n'est publiée
             *     — un brouillon doit se relire avant d'être publié, c'est même son objet.
             */
            version?: number | null;
            variables?: {
                [key: string]: string;
            };
        };
        Readiness: {
            status: string;
        };
        /** @description Ce qu'une écriture de règle déclare — `ReminderRulePayload`. */
        ReminderRulePayload: {
            /**
             * Format: uuid
             * @description **Exactement l'un des deux.** Le modèle l'exige
             *     (`ck_reminder_rules_scope`), et le refus sort sur le champ `scope`.
             */
            event_id?: string | null;
            /** Format: uuid */
            session_id?: string | null;
            /**
             * @description En minutes, **cumulés**. Absent : le défaut du modèle — 2 jours, 1 jour,
             *     1 heure, 30 minutes.
             */
            offsets?: number[] | null;
            /** @description Absent : `email`, le seul canal que ce jalon sait servir. */
            channels?: string[] | null;
            type_code?: string | null;
            /** Format: uuid */
            template_id?: string | null;
            /**
             * @description Absent : active. **Couper sans supprimer** se fait ici ; supprimer
             *     annule en plus les rappels encore à traiter.
             */
            is_active?: boolean | null;
        };
        /**
         * @description Placer, déplacer, redimensionner, retirer — `ScheduleSessionPayload`.
         *
         *     **Une seule écriture pour les quatre gestes** : la base n'en distingue pas,
         *     ce sont les colonnes `room_id`, `starts_at` et `ends_at`. Quatre routes
         *     auraient donné quatre occasions de diverger sur la détection des conflits,
         *     qui est justement ce que l'écran doit rendre identique dans les quatre.
         *
         *     `room_id` nul **renvoie la séance au panneau** ; ce n'est pas une suppression.
         */
        ScheduleSessionPayload: {
            /**
             * Format: uuid
             * @description Envoyé par le front, **ignoré** : l'identifiant qui fait foi est celui de
             *     l'adresse.
             */
            session_id?: string | null;
            /** Format: uuid */
            room_id?: string | null;
            /** Format: date-time */
            starts_at: string;
            /** Format: date-time */
            ends_at: string;
            /**
             * Format: uuid
             * @description Journée de rattachement, **facultative**. Non fournie, elle est remise à
             *     nul pour que la base la redéduise (R9, écart n° 113).
             */
            event_day_id?: string | null;
            /**
             * @description Deux valeurs **déduites** que le contrat ne porte pas et qu'un client
             *     pourrait envoyer : elles sont refusées en nommant leur champ.
             */
            time_range?: unknown;
            enforce_room_exclusivity?: boolean | null;
        };
        /**
         * @description La diffusion et son canal — `SessionBroadcastPayload`.
         *
         *     **Le canal EST saisissable** quand la diffusion est activée : le déclencheur
         *     ne pose le canal par défaut que lorsque la colonne est nulle, il complète et
         *     n'écrase jamais. L'écran laisse le choix quand l'édition a plusieurs canaux
         *     (R8, écart n° 111).
         */
        SessionBroadcastPayload: {
            /** Format: uuid */
            session_id?: string | null;
            is_streamed: boolean;
            /** Format: uuid */
            broadcast_channel_id?: string | null;
        };
        /**
         * @description Ce qu'une tentative d'inscription porte — `SessionRegisterPayload`.
         *
         *     **Pas `RegisterPayload`** : ce nom est celui de l'ouverture de compte, servi
         *     par le module Identité. Deux formes sans rapport sous un même nom faisaient
         *     coexister dans le contrat engendré une inscription à une séance et une
         *     création de compte, et le garde-fou de contrat validait l'une contre l'autre.
         */
        SessionRegisterPayload: {
            /**
             * @description Clés = `code` des champs **actifs** du formulaire applicable. Une clé
             *     inconnue est refusée : une réponse mal orthographiée qui disparaît sans
             *     un mot est une réponse perdue.
             */
            answers?: unknown;
            /** @description Langue des envois ultérieurs ; défaut, la langue négociée de la requête. */
            locale?: string | null;
            guest?: null | components["schemas"]["Invite"];
            /** @description Exigé dès qu'une réponse est donnée à un champ marqué sensible. */
            sensitive_data_consent?: boolean;
            /**
             * Format: uuid
             * @description Organisation déclarée par l'inscrit, quand il y en a une.
             */
            organization_id?: string | null;
        };
        /**
         * @description La liste des journées spéciales — `SessionTracksPayload`.
         *
         *     **Manuel et indépendant de la date** : toutes les activités du 12 novembre ne
         *     relèvent pas de la « Journée finance durable » (règle métier n° 7). La liste
         *     envoyée **remplace** la précédente.
         */
        SessionTracksPayload: {
            /** Format: uuid */
            session_id?: string | null;
            track_ids: string[];
        };
        /** @description Les valeurs saisissables — une par colonne éditable, plus les thématiques. */
        ShowcaseFormValues: {
            /** Format: uuid */
            id?: string | null;
            placement: string;
            status: string;
            nature_code: string;
            /** Format: int32 */
            sort_order: number;
            title: unknown;
            quote?: unknown;
            body?: unknown;
            /** Format: uuid */
            person_id?: string | null;
            author_name?: string | null;
            author_title?: unknown;
            /** Format: uuid */
            organization_id?: string | null;
            organization_label?: string | null;
            /** Format: uuid */
            country_id?: string | null;
            /** Format: uuid */
            event_id?: string | null;
            /** Format: uuid */
            session_id?: string | null;
            link_url?: string | null;
            link_label?: unknown;
            background_color_hex?: string | null;
            /** Format: date-time */
            starts_at?: string | null;
            /** Format: date-time */
            ends_at?: string | null;
            theme_codes?: string[];
        };
        ShowcaseReorderPayload: {
            /** Format: uuid */
            id: string;
            direction: string;
        };
        ShowcaseStatusPayload: {
            /** Format: uuid */
            id: string;
            status: string;
        };
        SuppressionPayload: {
            email: string;
            /** @description `hard_bounce`, `complaint`, `unsubscribe`, `invalid_address`, `manual`. */
            reason: string;
            detail?: string | null;
            /**
             * Format: date-time
             * @description Nulle : définitive. Une valeur lève la suppression toute seule le moment
             *     venu — une boîte pleine n'est pas une adresse morte.
             */
            expires_at?: string | null;
        };
        /**
         * @description L'écriture d'une révision. Le numéro n'est pas reçu : il est **posé** par le
         *     service, à la suite du dernier — deux administrateurs qui enregistrent en
         *     même temps ne doivent pas se disputer un numéro.
         */
        TemplateVersionPayload: {
            subject: unknown;
            body_html: unknown;
            body_text?: unknown;
        };
        /**
         * @description Le retrait, et son motif. **Ce n'est pas une suppression** : la ligne
         *     demeure, avec son instant, son auteur et ce motif, et reparaît à
         *     l'historique de la liste.
         */
        UnpublishIncidentPayload: {
            /** Format: uuid */
            incident_id?: string | null;
            reason?: string | null;
        };
        /**
         * @description Même corps, plus l'identifiant du message corrigé. Il est **redondant avec
         *     le chemin** et le contrat du site le porte : c'est le chemin qui fait foi.
         */
        UpdateIncidentPayload: components["schemas"]["IncidentPayload"] & {
            /** Format: uuid */
            from_event_id: string;
            /** Format: uuid */
            incident_id?: string | null;
        };
        /**
         * @description Ce qu'une annonce déclare — `UploadDeclaration`.
         *
         *     **Aucun octet.** L'annonce est une question, pas une tentative : elle n'écrit
         *     rien, ne réserve ni espace, ni clé, ni identifiant (FR-016).
         */
        UploadDeclaration: {
            filename: string;
            mime_type: string;
            /** Format: int64 */
            byte_size: number;
            /**
             * @description L'entité que le fichier illustrera. Facultative : un objet peut être
             *     déposé sans rôle visé, et rattaché plus tard.
             */
            owner_schema?: string | null;
            owner_table?: string | null;
            /** Format: uuid */
            owner_id?: string | null;
            role?: string | null;
            /**
             * @description L'empreinte, **si le client sait la calculer**. Elle évite le transfert
             *     entier quand le contenu est déjà connu. Le contrat du front ne la porte
             *     pas encore ; la route l'accepte quand même (FR-011).
             */
            checksum_sha256?: string | null;
        };
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    admin_appel_creer: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description CallSaveResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition inexistante **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Retrait d'un critère porteur de notes (EVENT_CRITERION_HAS_SCORES) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_appel_grille_par_defaut: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description EditionCriterion[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_appel_modifier: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'appel */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description CallSaveResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Appel inexistant **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Retrait d'un critère porteur de notes (EVENT_CRITERION_HAS_SCORES) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_comite_enregistrer: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'appel */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description CommitteeSaveResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Appel inexistant **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Personne inconnue (EVENT_UNKNOWN_REFERENCE) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_canal_creer: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description EditionTabResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition inexistante **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_canal_modifier: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du canal */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description EditionTabResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Canal inexistant **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_canal_supprimer: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du canal */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description EditionTabResult — `deactivated` est un succès */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Canal inexistant **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_tableau_de_bord: {
        parameters: {
            query: {
                /** @description Édition mesurée */
                event_id: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description AdminDashboard */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Périmètre d'administration vide, ou `analytics.dashboard.read` absente sur l'édition */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition inexistante **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    engagement_liste_de_suppression: {
        parameters: {
            query?: {
                /** @description Fragment d'adresse */
                q?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description EmailSuppression[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    engagement_supprimer_une_adresse: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SuppressionPayload"];
            };
        };
        responses: {
            /** @description EmailSuppression */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description VALIDATION_FAILED */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    engagement_retirer_une_suppression: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description L'adresse à libérer */
                email: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description { removed } */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_editions_lister: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description EditionListScreen */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_edition_creer: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description EditionSaveResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Portée globale exigée (EVENT_GLOBAL_SCOPE_REQUIRED), ou périmètre vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Série, pays ou fuseau inconnus (EVENT_UNKNOWN_REFERENCE) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_editions_options_de_formulaire: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description EditionFormOptions */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_edition_detail: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'édition */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description EditionDetail */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition inexistante **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_edition_modifier: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'édition */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description EditionSaveResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition inexistante **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Série, pays ou fuseau inconnus (EVENT_UNKNOWN_REFERENCE) */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_journees_generer: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'édition */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description EditionTabResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition inexistante **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_journees_plan: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'édition */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description DayGenerationPlan | null */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition inexistante **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_journee_habiller: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'édition */
                id: string;
                /** @description Identifiant de la journée */
                dayId: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description EditionTabResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Journée inexistante **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_incidents_lister: {
        parameters: {
            query: {
                /** @description Édition dont on veut l'écran */
                event_id: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description IncidentListScreen */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition inexistante **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_incidents_creer: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateIncidentPayload"];
            };
        };
        responses: {
            /** @description IncidentWriteResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description `from_event_id` inexistant **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_incidents_gabarit_debordement: {
        parameters: {
            query: {
                /** @description Activité qui déborde */
                session_id: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description OverrunTemplate */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Activité inexistante **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_incidents_relire: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du message */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ManagedIncident */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Message inexistant **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_incidents_corriger: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du message */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateIncidentPayload"];
            };
        };
        responses: {
            /** @description IncidentWriteResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description `from_event_id` inexistant **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_incidents_publier: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du message */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description IncidentWriteResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_incidents_depublier: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du message */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: {
            content: {
                "application/json": components["schemas"]["UnpublishIncidentPayload"];
            };
        };
        responses: {
            /** @description IncidentWriteResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    media_orphelins: {
        parameters: {
            query?: {
                /** @description Ancienneté minimale en jours */
                min_age_days?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description OrphanAsset[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    media_quotas: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description QuotaRow[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    media_relever_le_plafond: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description L'organisation dont on relève le plafond */
                organizationId: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PlafondPayload"];
            };
        };
        responses: {
            /** @description QuotaRow */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Organisation inexistante */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description VALIDATION_FAILED */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    engagement_modeles_de_messages: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description MessageTemplateRow[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de gérer les modèles absente */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    engagement_modele_de_message: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du modèle */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description TemplateDetail */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de gérer les modèles absente */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Modèle inexistant */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    engagement_apercu_de_modele: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du modèle */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PreviewPayload"];
            };
        };
        responses: {
            /** @description { fr, en } */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de gérer les modèles absente */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Modèle ou révision inexistante */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    engagement_ecrire_revision_de_modele: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du modèle */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["TemplateVersionPayload"];
            };
        };
        responses: {
            /** @description TemplateVersion */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de gérer les modèles absente */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Modèle inexistant */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description VALIDATION_FAILED */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    engagement_publier_revision_de_modele: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du modèle */
                id: string;
                /** @description Numéro de la révision */
                version: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description TemplateDetail */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de gérer les modèles absente */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description ENGAGEMENT_TEMPLATE_VERSION_UNKNOWN */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description ENGAGEMENT_TEMPLATE_VARIABLE_UNKNOWN · ENGAGEMENT_NOTIFICATION_TYPE_UNKNOWN */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    engagement_diffuser_une_annonce: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["BroadcastPayload"];
            };
        };
        responses: {
            /** @description { recipients, emailed } */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de diffuser absente */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description VALIDATION_FAILED */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_organisations_liste: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description OrganizationListScreen */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_file_des_doublons: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description DuplicateQueueScreen */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Permission de fusion absente, ou détenue sur une portée qui n'est pas globale */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_decision_de_doublon: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Paire */
                pairId: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description DuplicateDecisionResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Permission de fusion absente, ou portée non globale */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Décision hors des deux valeurs recevables */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_fusionner: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description MergeResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description ORG_MERGE_GLOBAL_SCOPE_REQUIRED */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description ORG_MERGE_FIELD_NOT_ARBITRABLE ou ORG_MERGE_SAME_ORGANIZATION */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_organisations_similaires: {
        parameters: {
            query: {
                /** @description Terme cherché */
                name: string;
                /** @description Pays */
                country_id?: string;
                /** @description Adresse dont le domaine fait entrer la fiche */
                email?: string;
                /** @description Site dont le domaine fait entrer la fiche */
                website?: string;
                /** @description Défaut 10, maximum 50 */
                limit?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description SimilarOrganization[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_organisation_fiche: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'organisation */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description OrganizationDetail | null */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_organisation_domaine: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organisation */
                id: string;
                /** @description Domaine */
                domainId: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description OrganizationWriteResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Permission de gestion absente, ou périmètre vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description ORG_DOMAIN_VERIFICATION_REQUIRED */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_apercu_de_fusion: {
        parameters: {
            query: {
                /** @description Fiche **absorbante** */
                target_id: string;
                /** @description Paire de la file d'où vient la fusion */
                pair_id?: string;
            };
            header?: never;
            path: {
                /** @description Fiche **absorbée** */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description MergePreview | null */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description ORG_MERGE_GLOBAL_SCOPE_REQUIRED */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_organisation_denomination: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organisation */
                id: string;
                /** @description Dénomination */
                nameId: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description OrganizationWriteResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Permission de gestion absente, ou périmètre vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description ORG_NAME_IS_DERIVED */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_organisation_verification: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organisation */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description OrganizationWriteResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Permission de gestion absente, ou périmètre vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    seances_ecran_du_planificateur: {
        parameters: {
            query: {
                /** @description Édition dont on compose la grille */
                event_id: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description PlannerScreen */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de planifier absente, ou périmètre vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition inexistante **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    planificateur_publier: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description PublishProgrammeResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de planifier absente, ou périmètre vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition inexistante **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    planificateur_controle_de_publication: {
        parameters: {
            query: {
                /** @description Identifiant de l'édition */
                event_id: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description PublicationReadinessIssue[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de planifier absente, ou périmètre vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition inexistante **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    file: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description PrivacyQueueScreen */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Portée globale exigée — un administrateur d'édition est refusé */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    traiter: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la demande */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description PrivacyWriteResult — saved, anonymized, wrong_type, not_found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission ou portée insuffisante */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Requête invalide */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_deduire_les_transitions_v1: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Dossiers traités et lignes semées */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Portée globale exigée */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    engagement_regles_de_rappel: {
        parameters: {
            query: {
                /** @description L'édition dont on lit les règles */
                event_id: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ReminderRule[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition inexistante ou hors périmètre */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    engagement_ecrire_regle_de_rappel: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ReminderRulePayload"];
            };
        };
        responses: {
            /** @description ReminderRule */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition ou séance inexistante, ou hors périmètre */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description ENGAGEMENT_REMINDER_OFFSETS_INVALID · ENGAGEMENT_REMINDER_SCOPE_INVALID */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    engagement_supprimer_regle_de_rappel: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la règle */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description { cancelled_reminders } */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Règle inexistante, ou édition hors périmètre */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_salle_creer: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description EditionTabResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Lieu inexistant **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_salle_modifier: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la salle */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description EditionTabResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Salle inexistante **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_salle_supprimer: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la salle */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description EditionTabResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Salle inexistante **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_vitrine_lister: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ShowcaseListScreen */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_vitrine_creer: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ShowcaseFormValues"];
            };
        };
        responses: {
            /** @description ShowcaseWriteResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_vitrine_formulaire_vierge: {
        parameters: {
            query?: {
                /** @description Emplacement d'arrivée — un seul existe */
                placement?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ShowcaseFormScreen */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_vitrine_seances: {
        parameters: {
            query: {
                /** @description Édition dont on veut les séances */
                event_id: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ShowcaseSessionOption[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition hors périmètre */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_vitrine_valeurs: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la diapositive */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ShowcaseFormValues */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, périmètre vide, ou contenu de plateforme hors portée globale */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Diapositive inexistante **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_vitrine_modifier: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la diapositive */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ShowcaseFormValues"];
            };
        };
        responses: {
            /** @description ShowcaseWriteResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, périmètre vide, ou contenu de plateforme hors portée globale */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Diapositive inexistante **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_vitrine_dupliquer: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la diapositive à copier */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ShowcaseWriteResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, périmètre vide, ou contenu de plateforme hors portée globale */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Diapositive inexistante **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_vitrine_formulaire: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la diapositive */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ShowcaseFormScreen */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, périmètre vide, ou contenu de plateforme hors portée globale */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Diapositive inexistante **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_vitrine_ordonner: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la diapositive */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ShowcaseReorderPayload"];
            };
        };
        responses: {
            /** @description ShowcaseWriteResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, périmètre vide, ou contenu de plateforme hors portée globale */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Diapositive inexistante **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_vitrine_statut: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la diapositive */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ShowcaseStatusPayload"];
            };
        };
        responses: {
            /** @description ShowcaseWriteResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, périmètre vide, ou contenu de plateforme hors portée globale */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Diapositive inexistante **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_fil_creer: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description EditionTabResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition inexistante **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_fil_modifier: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du fil */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description EditionTabResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Fil inexistant **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_fil_supprimer: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du fil */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description EditionTabResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Fil inexistant **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_utilisateur_liste: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description UserListScreen */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Sans la permission, ou sur périmètre vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_utilisateur_options_dattribution: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description RoleAssignmentOptions */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission ou portée insuffisante */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_utilisateur_retirer_role: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'attribution */
                assignment_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description RoleWriteResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission ou portée insuffisante */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Requête invalide */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_utilisateur_fiche: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la personne */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description UserDetail | null */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission ou portée insuffisante */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_utilisateur_permissions_effectives: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la personne */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description EffectivePermissionsView */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission ou portée insuffisante */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_utilisateur_attribuer_role: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la personne */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description RoleWriteResult — granted, duplicate, scope_not_allowed, forbidden_scope, not_found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission ou portée insuffisante */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Requête invalide */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_utilisateur_changer_le_statut: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la personne */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description PersonWriteResult — saved, missing_deadline, not_found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission ou portée insuffisante */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Requête invalide */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_lieu_creer: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description EditionTabResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition inexistante **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_lieu_modifier: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du lieu */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description EditionTabResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Lieu inexistant **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    admin_lieu_supprimer: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du lieu */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description EditionTabResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission absente, ou périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Lieu inexistant **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    login: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description LoginResult — authenticated, mfa_required, invalid_credentials, locked, suspended, email_unverified */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Requête invalide */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    logout: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description { status: "signed_out" } */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    me: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Person | null — corps null hors session */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    request_password_reset: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description PasswordResetRequestResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Requête invalide */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    check_password_reset_token: {
        parameters: {
            query: {
                /** @description Le jeton reçu par courriel */
                token: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description TokenCheckResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Requête invalide */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    reset_password: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description PasswordResetResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Requête invalide */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    refresh: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description { status: "renewed" | "expired" } */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    register: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description RegisterResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Requête invalide */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    verify_email: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description VerifyEmailResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Requête invalide */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    resend_verification: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description ResendVerificationResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Requête invalide */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    series_devenements: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description EventSeries[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    editions_du_perimetre: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description EventEdition[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    editions_publiques: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description PublicEditionRow[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    evenement_incidents_actifs: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Édition affichée */
                event_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ActiveIncident[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    programmation_seance_publique: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Édition */
                event_id: string;
                /** @description Adresse d'URL de la séance */
                slug: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Séance publiée, intervenants et organisations */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Adresse inconnue **ou séance non publiée** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    appel_public: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'édition */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description PublicCall | null */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    canaux_publics: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'édition */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description BroadcastChannel[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    journees_publiques: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'édition */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description EventDay[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    images_de_ledition: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'édition */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Record<EditionImageRole, AttachedImage | null> */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    salles_publiques: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'édition */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Room[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    fils_publics: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'édition */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ProgrammeTrack[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    lieux_publics: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'édition */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Venue[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    edition_publique: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Adresse d'URL de l'édition */
                slug: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description EventEdition | null */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    health: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Les indicateurs et leurs seuils */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["OperationalHealth"];
                };
            };
            /** @description Aucune session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description `analytics.dashboard.read` absente, quelle que soit la portée */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    vitrine_de_l_accueil: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Pick<HomeScreen, 'hero'> */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    engagement_ingerer_les_retours_de_courriel: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["MailEvent"][];
            };
        };
        responses: {
            /** @description { applied, ignored } */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Jeton porteur absent ou faux */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Jeton non configuré : la route n'est pas montée */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    media_deposer: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Asset, avec `deduplicated` */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description MEDIA_UPLOAD_INCOMPLETE — flux rompu, ou poids reçu différent du poids annoncé */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Entité porteuse inexistante ou hors périmètre */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description MEDIA_TOO_LARGE — plafond du rôle, ou plafond absolu du dépôt */
            413: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description MEDIA_ALT_TEXT_REQUIRED · MEDIA_MIME_NOT_ALLOWED · MEDIA_QUOTA_EXCEEDED · MEDIA_ROLE_NOT_DECLARED */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description MEDIA_STORAGE_UNAVAILABLE — le stockage n'a pas répondu */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    media_annoncer: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UploadDeclaration"];
            };
        };
        responses: {
            /** @description UploadVerdict */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Entité porteuse inexistante, hors périmètre, ou dont le rôle n'a pas de garde */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    media_objet: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'objet */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Asset */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Objet inexistant ou supprimé */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    media_supprimer: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'objet */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description { scheduled_purge_at } */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description L'objet n'appartient pas à l'acteur */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Objet inexistant ou déjà supprimé */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description MEDIA_ASSET_IN_USE */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    media_avancement: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'objet */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description AssetProgress */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Objet inexistant ou supprimé */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    media_rattachements: {
        parameters: {
            query: {
                /** @description Schéma de l'entité porteuse */
                owner_schema: string;
                /** @description Table de l'entité porteuse */
                owner_table: string;
                /** @description Identifiant de l'entité porteuse */
                owner_id: string;
                /** @description Rôle, facultatif */
                role?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description AttachedMedia[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Entité inexistante, hors périmètre, ou sans garde déclarée */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    media_remplacer_rattachements: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AttachmentBatch"];
            };
        };
        responses: {
            /** @description AttachedMedia[] — tous les médias de l'entité après l'écriture */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Entité ou objet inexistant, ou hors périmètre */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description MEDIA_ROLE_EXCLUSIVE — deux objets demandés pour un rôle qui n'en accepte qu'un */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description MEDIA_TOO_LARGE */
            413: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description MEDIA_ROLE_NOT_DECLARED · MEDIA_MIME_NOT_ALLOWED · MEDIA_ASPECT_RATIO · MEDIA_ASSET_NOT_SERVABLE */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    media_rattacher: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AttachmentPayload"];
            };
        };
        responses: {
            /** @description AttachedMedia */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Entité ou objet inexistant, ou hors périmètre */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description MEDIA_ROLE_EXCLUSIVE — le rôle n'accepte qu'un seul objet */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description MEDIA_TOO_LARGE — poids dépassé pour ce rôle */
            413: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description MEDIA_ROLE_NOT_DECLARED · MEDIA_MIME_NOT_ALLOWED · MEDIA_ASPECT_RATIO · MEDIA_ASSET_NOT_SERVABLE */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    media_detacher: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du rattachement, pas de l'objet */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description { asset_kept } */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Rattachement inexistant, ou entité hors périmètre */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    media_roles: {
        parameters: {
            query: {
                /** @description Schéma de l'entité porteuse */
                owner_schema: string;
                /** @description Table de l'entité porteuse */
                owner_table: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description AttachableRoleRule[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    adhesion_revoquer: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Adhésion */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description { status } */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description ORG_NOT_MANAGER */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Adhésion inexistante */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    adhesion_decision: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Adhésion */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description Membership | null */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description ORG_NOT_MANAGER */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description ORG_MEMBERSHIP_IS_INVITATION ou ORG_MEMBERSHIP_NOT_PENDING */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    engagement_preferences_de_notification: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description NotificationPreferenceRow[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    engagement_ecrire_preferences_de_notification: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["NotificationPreferencePayload"][];
            };
        };
        responses: {
            /** @description NotificationPreferenceRow[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description VALIDATION_FAILED · ENGAGEMENT_NOTIFICATION_TYPE_UNKNOWN */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    engagement_fil_de_notifications: {
        parameters: {
            query?: {
                /** @description Ne rendre que les non lues */
                unread_only?: boolean;
                /** @description Taille de page, bornée à 100 */
                limit?: number;
                /** @description Pagination : avant cet instant */
                before?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description NotificationFeed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    engagement_archiver_notifications: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ArchivagePayload"];
            };
        };
        responses: {
            /** @description { archived } */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    engagement_marquer_notifications_lues: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["MarquagePayload"];
            };
        };
        responses: {
            /** @description { marked } */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    organisations_liste: {
        parameters: {
            query?: {
                /** @description Défaut 50, maximum 200 */
                limit?: number;
                /** @description Décalage */
                offset?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Organization[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    organisation_creer: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description CreateOrganizationResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Valeur refusée par le modèle — nom trop court, sigle hors bornes, pays inconnu */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    organisation_par_domaine: {
        parameters: {
            query?: {
                /** @description **Ignoré.** Le domaine vient de la session. */
                email?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description EmailDomainMatch | null */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    invitation_accepter: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description Acceptation, ou l'un des trois refus de jeton */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description ORG_INVITATION_NOT_YOURS */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    organisations_similaires: {
        parameters: {
            query: {
                /** @description Ce qui a été tapé : nom complet, sigle, fragment */
                name: string;
                /** @description Pays du profil — bonus de 10 */
                country_id?: string;
                /** @description Adresse : son domaine vaut 40, sauf messagerie grand public */
                email?: string;
                /** @description Site saisi au formulaire — même usage */
                website?: string;
                /** @description Défaut 10, maximum 50 */
                limit?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description SimilarOrganization[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    organisation_fiche: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'organisation */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Organization | null */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_editions_de_lorganisation: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'organisation */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description EventEdition[] | null */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    organisation_inviter: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organisation */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description InviteMemberResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description ORG_NOT_MANAGER — pas référent actif de cette organisation */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    organisation_rejoindre: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organisation visée */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description JoinOrganizationResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Organisation inexistante */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_espace_organisation: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'organisation */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description WorkspaceOverview | null */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    lister: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Person[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission ou portée insuffisante */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    depot_chercher_intervenant: {
        parameters: {
            query: {
                /** @description Adresse électronique exacte */
                email: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description PersonLookup ou null */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    fiche: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la personne */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Person | null */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission ou portée insuffisante */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    perimetre: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la personne */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description AdministeredEvents */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission ou portée insuffisante */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    personne_adhesions: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la personne */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Membership[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Ni soi-même, ni la permission */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    permissions: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la personne */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description EffectivePermission[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission ou portée insuffisante */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    roles: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la personne */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description RoleAssignmentView[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission ou portée insuffisante */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    platform_drapeaux: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ResolvedFeatureFlag[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    propositions_resoudre_une_demande: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du message */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description ProposalComment */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Message inexistant **ou hors d'accès** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Le message n'est pas une demande de correction */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_rouvrir_une_demande: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du message */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description ProposalComment */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Le déposant ne rouvre pas une demande de correction */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Message inexistant **ou hors d'accès** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_de_lorganisation: {
        parameters: {
            query: {
                /** @description Organisation porteuse */
                organization_id: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Proposal[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Organisation étrangère **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    depot_creer_brouillon: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description SaveDraftResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de soumettre absente */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Adhésion inactive à l'organisation porteuse — indiscernable d'un dossier inexistant */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Texte trop long, thématique inconnue, identité verrouillée, bornes de l'appel */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_confier_en_groupe: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description BulkResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Membre du comité inconnu — PROPOSAL_UNKNOWN_REFERENCE */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_comite: {
        parameters: {
            query: {
                /** @description Édition dont on lit le comité */
                event_id: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ProposalFacet[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition inexistante **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_pilotage: {
        parameters: {
            query: {
                /** @description Édition dont on liste les dossiers */
                event_id: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ProposalDashboardRow[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition inexistante **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    depot_mon_brouillon: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description SaveDraftResult ou null */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    depot_contexte_du_formulaire: {
        parameters: {
            query?: {
                /** @description Organisations de la personne, séparées par des virgules */
                organization_ids?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ProposalFormContext */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_ecran_de_liste: {
        parameters: {
            query: {
                /** @description Édition dont on liste les dossiers */
                event_id: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ProposalListScreen */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition inexistante **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_changer_letat_en_groupe: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description BulkResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Périmètre d'administration vide — refus explicite, jamais une liste vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_regles_de_transition: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ProposalTransitionRule[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_fiche: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Proposal */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier inexistant **ou hors d'accès** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    depot_enregistrer_brouillon: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description SaveDraftResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de soumettre absente */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier inexistant **ou porté par une organisation dont vous n'êtes pas membre actif** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier clos (PROPOSAL_NOT_EDITABLE), texte trop long, thématique inconnue, identité verrouillée */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_transitions_offertes: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Les transitions offertes, avec leur exigence de motif */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier inexistant **ou hors d'accès** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_fil: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ProposalComment[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier inexistant **ou hors d'accès** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_ecrire_un_message: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description ProposalComment */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier hors d'accès **ou inexistant** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Corps vide */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_decider: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description DecisionResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier inexistant **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_pieces: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ProposalDocumentEntry[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier inexistant **ou hors d'accès** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_rattacher_une_piece: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description ProposalDocument */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier inexistant **ou hors d'accès** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Objet stocké inconnu — PROPOSAL_UNKNOWN_REFERENCE */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_detacher_une_piece: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
                /** @description Identifiant de la pièce */
                document_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Pièce détachée ; l'objet stocké demeure */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Pièce inexistante, d'un autre dossier, **ou hors d'accès** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    depot_rouvrir_un_dossier: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description EditableProposal */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier inexistant **ou hors d'accès** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_dossier_du_deposant: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ProposalFile | null */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_historique: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ProposalHistoryEntry[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier inexistant **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_organisations: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ProposalOrganization[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier inexistant **ou hors d'accès** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_se_deporter: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description ReviewAssignment */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Aucune affectation à quitter — PROPOSAL_REVIEW_NOT_ASSIGNED */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier inexistant **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Motif manquant */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    depot_renvoyer_un_dossier: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description SubmitProposalResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier inexistant **ou porté par une organisation dont on n'est pas membre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Le dossier n'attend aucune correction, ou l'édition est terminée */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_fiche_devaluation: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ReviewDeskScreen */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Périmètre d'administration vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier inexistant **ou hors périmètre** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_noter: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description SaveReviewResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Noter sans affectation, ou après un déport — PROPOSAL_REVIEW_NOT_ASSIGNED */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier inexistant **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Note au-dessus du maximum de son critère, ou critère étranger à la grille */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_intervenants: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ProposalSpeaker[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier inexistant **ou hors d'accès** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    depot_deposer: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": Record<string, never>;
            };
        };
        responses: {
            /** @description SubmitProposalResult — submitted, call_closed, quota_reached */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de soumettre absente */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier inexistant **ou hors de vos organisations** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier incomplet, ou bornes d'intervenants de l'appel */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_thematiques: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Les pastilles de thématique */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier inexistant **ou hors d'accès** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    propositions_journal: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant du dossier */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ProposalTransition[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Dossier inexistant **ou hors d'accès** — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    ready: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Le processus est prêt à servir */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Readiness"];
                };
            };
            /** @description Base injoignable */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    reference_pays: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Country[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    reference_langues: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Locale[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    reference_termes: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Code de la taxonomie, ex. `activity_theme` */
                code: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description TaxonomyTerm[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    inscriptions_liste_nominative: {
        parameters: {
            query: {
                /** @description Séance dont on liste les inscrits */
                session_id: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description RegistrationRow[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de gérer les inscriptions absente */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Séance inexistante **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    inscriptions_les_miennes: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Registration[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    inscriptions_annuler: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'inscription */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AnnulationDemandee"];
            };
        };
        responses: {
            /** @description CancelRegistrationResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Inscription inexistante **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Le déclencheur refuse la modification — séance annulée, ou question devenue obligatoire */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    inscriptions_rejoindre: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de l'inscription */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description { joined_at } */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Inscription inexistante, ou celle de quelqu'un d'autre — indiscernables */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    programmation_publique: {
        parameters: {
            query?: {
                /** @description Édition dont on lit le programme. Absente : les séances à venir de toutes les éditions */
                event_id?: string;
                /** @description Plafond du nombre de lignes (défaut 50, maximum 200) */
                limit?: number;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description PublicScheduleRow[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
        };
    };
    seances_liste: {
        parameters: {
            query: {
                /** @description Édition dont on liste les séances */
                event_id: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description PlannerSession[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de planifier absente, ou périmètre vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition inexistante **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    seances_conflits: {
        parameters: {
            query: {
                /** @description Édition dont on recense les chevauchements */
                event_id: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ScheduleConflict[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de planifier absente, ou périmètre vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Édition inexistante **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    seances_diffusion: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la séance */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SessionBroadcastPayload"];
            };
        };
        responses: {
            /** @description PlannerMutationResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de planifier absente, ou périmètre vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Séance inexistante **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Canal désigné sans diffusion, canal désactivé ou d'une autre édition */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    seances_organisations: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la séance */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description SessionOrganization[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de planifier absente, ou périmètre vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Séance inexistante **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    inscriptions_formulaire: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la séance */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Formulaire applicable et champs actifs */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Séance inexistante */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    inscriptions_sinscrire: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la séance */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SessionRegisterPayload"];
            };
        };
        responses: {
            /** @description RegistrationResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Le formulaire n'admet pas l'inscription sans compte */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Séance inexistante */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Réponse invalide, consentement manquant, ou séance ne prenant pas d'inscription */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    engagement_regle_de_rappel_applicable: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la séance */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ApplicableReminderRule | null */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Ni adhésion active, ni droit de gérer les inscriptions */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Séance inexistante */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    engagement_calendrier_des_rappels: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la séance */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description { slots, has_rule } */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Ni adhésion active, ni droit de gérer les inscriptions */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Séance inexistante */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    seances_placer: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la séance */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ScheduleSessionPayload"];
            };
        };
        responses: {
            /** @description PlannerMutationResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de planifier absente, ou périmètre vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Séance inexistante **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Valeur déduite envoyée, créneau invalide, ou salle d'une autre édition */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    seances_intervenants: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la séance */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description SessionSpeaker[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de planifier absente, ou périmètre vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Séance inexistante **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    seances_fils: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la séance */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description SessionTrack[] */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de planifier absente, ou périmètre vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Séance inexistante **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
    seances_rattacher: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Identifiant de la séance */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SessionTracksPayload"];
            };
        };
        responses: {
            /** @description PlannerMutationResult */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": Record<string, never>;
                };
            };
            /** @description Aucune session, ou session close */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Permission de planifier absente, ou périmètre vide */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Séance inexistante **ou hors périmètre** */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
            /** @description Journée spéciale d'une autre édition, ou inexistante */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiError"];
                };
            };
        };
    };
}
