//! Ce qu'on lit d'un dossier : la table des règles, son journal, et ce qui lui
//! est encore ouvert.
//!
//! # Trois chemins, et l'un d'eux a failli en écraser un autre
//!
//! Le prompt proposait `GET /proposals/:id/transitions` pour les **transitions
//! offertes**. Ce chemin est **déjà** celui du journal dans le contrat du front
//! — la fabrique des propositions y lit les lignes de
//! `programme.proposal_transitions`. Le journal le garde ; les transitions
//! offertes prennent `available-transitions` (écart n° 101, R19).
//!
//! Elles sont exposées **deux fois, et c'est voulu** : ici pour l'espace
//! organisation et pour rafraîchir un menu après une décision groupée, et comme
//! champ de la fiche d'évaluation — où l'en-tête en a besoin sans requête de
//! plus.
//!
//! # `/proposals/transitions` et `/proposals/{id}/transitions` ne se
//! concurrencent pas
//!
//! Le préfixe diffère au premier segment : « transitions » d'un côté, un
//! identifiant de l'autre. C'est le seul chemin littéral de ce fichier qui n'ait
//! **pas** besoin de précéder son homologue paramétré — les autres, oui.

use actix_web::{web, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::ids::ProposalId;
use crate::repo::transitions as repo;
use crate::service::{detail, transition};
use crate::state::ProgrammeState;

/// L'organisation dont on liste les dossiers.
#[derive(Debug, Deserialize)]
pub struct OrganisationDemandee {
    organization_id: Uuid,
}

/// Les chemins littéraux de ce fichier.
///
/// `""` est le chemin du scope lui-même — `GET /proposals`. Il ne concurrence
/// rien : Actix distingue une ressource sans segment d'une ressource à un
/// segment.
pub fn chemins_litteraux(cfg: &mut web::ServiceConfig) {
    cfg.route("/transitions", web::get().to(regles))
        .route("", web::get().to(de_lorganisation));
}

/// Les chemins portant un identifiant de dossier.
///
/// **`GET /{id}` arrive ici, et c'est le moment que `lib.rs` annonçait** : les
/// chemins littéraux du scope sont désormais en concurrence réelle avec lui,
/// sur la même méthode. Seul leur enregistrement préalable les sauve, et
/// `crates/api/tests/routes_programme.rs` le vérifie sur la vraie application.
pub fn chemins_de_dossier(cfg: &mut web::ServiceConfig) {
    cfg.route("/{id}", web::get().to(dossier))
        .route("/{id}/transitions", web::get().to(journal))
        .route(
            "/{id}/available-transitions",
            web::get().to(transitions_offertes),
        )
        .route("/{id}/organizations", web::get().to(organisations))
        .route("/{id}/speakers", web::get().to(intervenants))
        .route("/{id}/themes", web::get().to(thematiques))
        .route("/{id}/history", web::get().to(historique))
        .route("/{id}/comments", web::get().to(fil))
        .route("/{id}/documents", web::get().to(pieces))
        .route("/{id}/documents", web::post().to(rattacher))
        .route("/{id}/documents/{document_id}", web::delete().to(detacher));
}

/// Les organisations associées au dossier.
#[utoipa::path(
    get,
    description = "`ProposalOrganization[]` — **porteur compris**, dans l'ordre où le dossier les range. Une co-organisation dont `confirmed_at` est nulle est **annoncée, pas acquise** : elle engage un tiers, et le back-office l'affiche « en attente ».",
    path = "/proposals/{id}/organizations",
    tag = "Back-office — propositions",
    operation_id = "propositions_organisations",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    responses(
        (status = 200, description = "ProposalOrganization[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors d'accès**", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn organisations(
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let dossier = ProposalId(chemin.into_inner());
    crate::service::perimeter::acces_au_dossier(state.pool(), acteur.0, dossier).await?;
    let liens = crate::repo::organizations::du_dossier(state.pool(), dossier).await?;

    Ok(HttpResponse::Ok().json(liens))
}

/// Les intervenants annoncés.
#[utoipa::path(
    get,
    description = "`ProposalSpeaker[]`, dans l'ordre annoncé. **Les deux instantanés voyagent** — fonction et organisation **au moment de cette activité** : une personne change d'employeur, et l'archive d'une COP passée ne doit pas être réécrite.",
    path = "/proposals/{id}/speakers",
    tag = "Back-office — propositions",
    operation_id = "propositions_intervenants",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    responses(
        (status = 200, description = "ProposalSpeaker[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors d'accès**", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn intervenants(
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let dossier = ProposalId(chemin.into_inner());
    crate::service::perimeter::acces_au_dossier(state.pool(), acteur.0, dossier).await?;
    let lignes = crate::repo::speakers::du_dossier(state.pool(), dossier).await?;

    Ok(HttpResponse::Ok().json(lignes))
}

/// Les thématiques du dossier, **prêtes à afficher**.
#[utoipa::path(
    get,
    description = "Les pastilles de `reference.term_badges()` — libellé traduit et couleur venus de `reference.taxonomy_terms`, **où un administrateur les modifie**. N'exposer que les codes forcerait l'écran à recharger la taxonomie : c'est ainsi que les libellés se sont retrouvés figés dans le frontend de la v1.",
    path = "/proposals/{id}/themes",
    tag = "Back-office — propositions",
    operation_id = "propositions_thematiques",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    responses(
        (status = 200, description = "Les pastilles de thématique", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors d'accès**", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn thematiques(
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let dossier = ProposalId(chemin.into_inner());
    crate::service::perimeter::acces_au_dossier(state.pool(), acteur.0, dossier).await?;
    let pastilles = crate::repo::cross::pastilles_du_dossier(state.pool(), dossier).await?;

    Ok(HttpResponse::Ok().json(pastilles))
}

/// L'historique champ par champ.
#[utoipa::path(
    get,
    description = "`ProposalHistoryEntry[]` — par `programme.proposal_history()`, qui **écarte déjà les colonnes recalculées** : date de mise à jour, vecteur de recherche, compteur de vues. Les refaire ici ferait apparaître une modification à chaque affichage. **Réservé au back-office** : le déposant lit son propre historique par la route de son espace.",
    path = "/proposals/{id}/history",
    tag = "Back-office — propositions",
    operation_id = "propositions_historique",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    responses(
        (status = 200, description = "ProposalHistoryEntry[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn historique(
    state: web::Data<ProgrammeState>,
    perimetre: kernel::auth::Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let dossier = ProposalId(chemin.into_inner());
    crate::service::perimeter::edition_dans_le_perimetre(
        state.pool(),
        &perimetre,
        crate::service::perimeter::Cible::Dossier(dossier),
    )
    .await?;
    let lignes = crate::repo::cross::historique_du_dossier(state.pool(), dossier).await?;

    Ok(HttpResponse::Ok().json(lignes))
}

/// Le fil des échanges, **filtré par visibilité à la source**.
#[utoipa::path(
    get,
    description = "`ProposalComment[]` — **filtré à la source, jamais après coup** : ce qui n'est pas envoyé ne peut pas fuiter. Côté comité, les messages du comité, ceux adressés au déposant et **ses propres** notes personnelles ; côté organisation, ce qui lui est adressé et rien d'autre. Le filtre est le même des deux côtés — l'écrire deux fois serait écrire deux filtres, et le second finirait par diverger.",
    path = "/proposals/{id}/comments",
    tag = "Back-office — évaluation",
    operation_id = "propositions_fil",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    responses(
        (status = 200, description = "ProposalComment[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors d'accès**", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn fil(
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let messages =
        crate::service::comments::fil_de(&state, acteur.0, ProposalId(chemin.into_inner())).await?;

    Ok(HttpResponse::Ok().json(messages))
}

/// Les pièces du dossier.
#[utoipa::path(
    get,
    description = "`ProposalDocumentEntry[]` — chaque pièce, son objet stocké et son **adresse composée en base**. **L'adresse est nulle quand l'objet n'est pas servi** — quarantaine, purge, téléversement inachevé —, et c'est cette nullité qui commande l'avertissement plutôt que le bouton : le comité doit savoir qu'une pièce manque à son dossier, pas cliquer sur un lien mort.",
    path = "/proposals/{id}/documents",
    tag = "Back-office — propositions",
    operation_id = "propositions_pieces",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    responses(
        (status = 200, description = "ProposalDocumentEntry[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors d'accès**", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn pieces(
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let pieces =
        crate::service::documents::lister(&state, acteur.0, ProposalId(chemin.into_inner()))
            .await?;

    Ok(HttpResponse::Ok().json(pieces))
}

/// Rattacher un objet **déjà stocké**.
#[utoipa::path(
    post,
    description = "Rattachement d'un objet **déjà stocké** → `ProposalDocument`. Le téléversement du fichier appartient à B6 : ce module reçoit un identifiant d'objet, jamais un fichier. Un objet inconnu ou supprimé rend `PROPOSAL_UNKNOWN_REFERENCE` **en nommant le champ** — la clé étrangère refuserait aussi, mais son message ne dirait pas lequel. Le titre par défaut est le nom du fichier d'origine : une pièce sans titre s'affiche « Document » dans une liste, et personne ne sait laquelle ouvrir.",
    path = "/proposals/{id}/documents",
    tag = "Back-office — propositions",
    operation_id = "propositions_rattacher_une_piece",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    request_body = Object,
    responses(
        (status = 200, description = "ProposalDocument", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors d'accès**", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Objet stocké inconnu — PROPOSAL_UNKNOWN_REFERENCE", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn rattacher(
    requete: actix_web::HttpRequest,
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
    corps: web::Json<crate::service::documents::AttachDocumentPayload>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.0);
    let piece = crate::service::documents::rattacher(
        &state,
        &ctx,
        acteur.0,
        ProposalId(chemin.into_inner()),
        corps.into_inner(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(piece))
}

/// Détacher une pièce — **l'objet stocké demeure**.
#[utoipa::path(
    delete,
    description = "Détachement d'une pièce. **L'objet stocké n'est pas détruit** : `media.assets` a son propre cycle de vie — suppression logique, date de purge, worker de purge — et un même objet peut être rattaché ailleurs. Le détruire ici effacerait la pièce d'un autre dossier sans le savoir. Le module ne détruit pas ce qu'il n'a pas créé.",
    path = "/proposals/{id}/documents/{document_id}",
    tag = "Back-office — propositions",
    operation_id = "propositions_detacher_une_piece",
    params(
        ("id" = Uuid, Path, description = "Identifiant du dossier"),
        ("document_id" = Uuid, Path, description = "Identifiant de la pièce"),
    ),
    responses(
        (status = 204, description = "Pièce détachée ; l'objet stocké demeure"),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Pièce inexistante, d'un autre dossier, **ou hors d'accès**", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn detacher(
    requete: actix_web::HttpRequest,
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse> {
    let (dossier, piece) = chemin.into_inner();
    let ctx = crate::routes::contexte_de(&requete, acteur.0);
    crate::service::documents::detacher(&state, &ctx, acteur.0, ProposalId(dossier), piece).await?;

    Ok(HttpResponse::NoContent().finish())
}

/// Les dossiers d'une organisation, brouillons compris.
#[utoipa::path(
    get,
    description = "`Proposal[]` — les dossiers dont cette organisation est **porteuse principale**, brouillons compris. Deux voies d'accès, distinctes : adhésion **active** — tous les dossiers, toutes éditions confondues, une organisation n'administrant rien —, ou lecture générale, **bornée au périmètre d'administration**. Une personne sans l'une ni l'autre reçoit le refus d'une ressource inexistante. **Par la voie de l'organisation, les notes ne sortent pas** : moyenne, note pondérée et élimination partent vides (FR-077, écart n° 104).",
    path = "/proposals",
    tag = "Espace organisation",
    operation_id = "propositions_de_lorganisation",
    params(("organization_id" = Uuid, Query, description = "Organisation porteuse")),
    responses(
        (status = 200, description = "Proposal[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Organisation étrangère **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn de_lorganisation(
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    demande: web::Query<OrganisationDemandee>,
) -> Result<HttpResponse> {
    let fiches = detail::de_lorganisation(&state, acteur.0, demande.organization_id).await?;
    Ok(HttpResponse::Ok().json(fiches))
}

/// Un dossier.
#[utoipa::path(
    get,
    description = "`Proposal` — le dossier tel que la table le porte, la colonne de recherche exclue. **Deux voies d'accès, un seul refus** : adhésion active à l'organisation porteuse, ou lecture générale dans le périmètre de l'édition ; inexistant, effacé, hors périmètre et organisation étrangère rendent tous le même 404. **Par la voie de l'organisation, les notes ne sortent pas** (FR-077, écart n° 104). `decision_reason` porte le motif de la **dernière** transition et rien de plus — une transition suivante l'écrase, et une transition sans motif l'efface : le motif d'une décision se lit dans le journal (écart n° 97).",
    path = "/proposals/{id}",
    tag = "Back-office — propositions",
    operation_id = "propositions_fiche",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    responses(
        (status = 200, description = "Proposal", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors d'accès** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn dossier(
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let fiche = detail::dossier(&state, acteur.0, ProposalId(chemin.into_inner())).await?;
    Ok(HttpResponse::Ok().json(fiche))
}

/// La table des règles, telle quelle.
#[utoipa::path(
    get,
    description = "`ProposalTransitionRule[]` — les quatorze lignes de `programme.proposal_transitions_allowed`, **rendues telles quelles**. La machine à états est une DONNÉE : l'écran n'affiche que les actions déclarées, avec leur permission et leur exigence de motif, et ajouter un chemin en base ajoute une action sans toucher au code. **Globale et sans dossier** : ce qui est offert à une personne sur un dossier donné est une autre question, et une autre route.",
    path = "/proposals/transitions",
    tag = "Back-office — propositions",
    operation_id = "propositions_regles_de_transition",
    responses(
        (status = 200, description = "ProposalTransitionRule[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn regles(
    state: web::Data<ProgrammeState>,
    _acteur: Actor,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(repo::regles(state.pool()).await?))
}

/// Le journal d'un dossier — **c'est lui qui porte chaque motif**.
#[utoipa::path(
    get,
    description = "`ProposalTransition[]` — le journal, du plus récent au plus ancien. **C'est lui qu'un écran doit lire pour un motif** : la colonne `decision_reason` du dossier ne garde que celui de la dernière transition, et une transition suivante l'écrase — y compris quand elle n'en demande aucun, auquel cas elle l'efface (écart n° 97). Un écran qui lirait la colonne afficherait « motif de la décision » sur un dossier remis en course. **Accès au dossier** : adhésion active à l'organisation porteuse, ou lecture générale dans le périmètre — deux voies distinctes, un seul refus.",
    path = "/proposals/{id}/transitions",
    tag = "Back-office — propositions",
    operation_id = "propositions_journal",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    responses(
        (status = 200, description = "ProposalTransition[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors d'accès** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn journal(
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let lignes = transition::journal_de(&state, acteur.0, ProposalId(chemin.into_inner())).await?;

    Ok(HttpResponse::Ok().json(lignes))
}

/// Ce qui est encore ouvert **à ce lecteur, sur ce dossier**.
#[utoipa::path(
    get,
    description = "Les transitions offertes **pour ce lecteur**, en une requête (R7). Une règle est retenue quand le lecteur est **porteur** — adhésion active — et que la règle l'y autorise, **ou** quand elle nomme une permission qu'il détient **sur l'édition du dossier**. La portée est celle de l'édition, pas la portée globale : c'est ce qui fait qu'un responsable détaché sur un webinaire ne décide pas sur la COP31. Le croisement se fait **au même instant que la lecture de l'état** — deux requêtes séparées offriraient une transition depuis un état déjà changé. Ce chemin existe parce que `/proposals/{id}/transitions` est déjà celui du journal (écart n° 101).",
    path = "/proposals/{id}/available-transitions",
    tag = "Back-office — propositions",
    operation_id = "propositions_transitions_offertes",
    params(("id" = Uuid, Path, description = "Identifiant du dossier")),
    responses(
        (status = 200, description = "Les transitions offertes, avec leur exigence de motif", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Dossier inexistant **ou hors d'accès** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn transitions_offertes(
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let offertes =
        transition::offertes_pour(&state, acteur.0, ProposalId(chemin.into_inner())).await?;

    Ok(HttpResponse::Ok().json(offertes))
}
