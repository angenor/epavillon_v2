//! **La liste du comité : bornée, comptée juste, et honnête sur ce qu'elle
//! n'a pas fait.**
//!
//! Trois choses s'y prouvent, et aucune ne se déduit des deux autres :
//!
//! - **le périmètre**, dans ses trois états — global, détaché, vide —, et le
//!   fait qu'une URL forgée ne dise rien de ce qui existe ailleurs ;
//! - **les facettes**, dont chaque décompte doit correspondre *exactement* aux
//!   lignes rendues : demandées à part, elles seraient mesurées à un autre
//!   instant, et le « Retenu (17) » du filtre finirait par mentir ;
//! - **les actions groupées**, qui doivent rendre compte de chaque dossier de
//!   la sélection — appliqués **plus** écartés **égale** la sélection.

mod commun;

use commun::{Bac, Terrain};
use kernel::auth::{AdminScope, Perimeter};
use kernel::error::ErrorCode;
use programme::domain::bulk::RaisonDEcart;
use programme::domain::facets::{FLAG_LATE, FLAG_UNREAD, FLAG_UNREVIEWED};
use programme::domain::ids::EventId;
use programme::service::list::{self, AssignReviewerPayload};
use uuid::Uuid;

// -----------------------------------------------------------------------------
// Trois périmètres, trois personnes
// -----------------------------------------------------------------------------

/// Administratrice **globale** : rôle `admin` sur la portée globale.
async fn globale(bac: &Bac) -> Perimeter {
    let personne = commun::personne(bac, "globale@ifdd.francophonie.org", "Gaëlle", "Ndiaye").await;
    commun::attribuer(bac, personne, "admin", "global", None).await;
    commun::perimetre_de(bac, personne).await
}

/// Administratrice **détachée** sur une seule édition — le cas PACO.
async fn detachee(bac: &Bac, edition: Uuid, courriel: &str) -> Perimeter {
    let personne = commun::personne(bac, courriel, "Détaché", "Test").await;
    commun::attribuer(bac, personne, "admin", "event", Some(edition)).await;
    commun::perimetre_de(bac, personne).await
}

/// **Aucun droit d'administration.** Le périmètre est construit à la main :
/// le garde du noyau refuse justement de le rendre, et c'est ce refus que le
/// service doit reproduire pour son propre compte.
async fn sans_droits(bac: &Bac) -> Perimeter {
    let personne = commun::personne(bac, "quidam@example.org", "Quid", "Am").await;
    Perimeter {
        person_id: personne,
        scope: AdminScope {
            is_global: false,
            event_ids: Vec::new(),
        },
    }
}

// -----------------------------------------------------------------------------
// De quoi peupler une édition
// -----------------------------------------------------------------------------

/// Un dossier posé **directement en base, dans l'état voulu**.
///
/// L'insertion échappe au garde d'état (écart n° 96) : c'est ce qui permet de
/// composer une édition hétérogène sans jouer douze transitions. La date de
/// dépôt accompagne tout état non brouillon, sans quoi
/// `ck_proposals_submitted_at` refuserait la ligne.
async fn dossier_dans_letat(
    bac: &Bac,
    terrain: &Terrain,
    titre: &str,
    slug: &str,
    statut: &str,
) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO programme.proposals
               (call_id, event_id, organization_id, submitted_by,
                title, slug, objectives, detailed_presentation, format,
                status, submitted_at)
           VALUES ($1, $2, $3, $4,
                   jsonb_build_object('fr', $5::text),
                   $6::text::platform.slug,
                   '{"fr":"Objectifs du dossier."}'::jsonb,
                   '{"fr":"<p>Présentation.</p>"}'::jsonb,
                   'hybrid',
                   $7::text::programme.proposal_status,
                   CASE WHEN $7 = 'draft' THEN NULL ELSE now() END)
        RETURNING id"#,
        terrain.appel,
        terrain.edition,
        terrain.organisation,
        terrain.deposante,
        titre,
        slug,
        statut
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du dossier")
}

/// Rattacher une thématique — table **polymorphe**, sans clé étrangère vers
/// les propositions : aucun autre module ne peut la poser.
async fn thematique(bac: &Bac, dossier: Uuid, code: &str) {
    sqlx::query!(
        "INSERT INTO reference.entity_terms (entity_schema, entity_table, entity_id, term_id)
         SELECT 'programme', 'proposals', $1, id
           FROM reference.taxonomy_terms
          WHERE taxonomy_code = 'activity_theme' AND code = $2",
        dossier,
        code
    )
    .execute(bac.pool())
    .await
    .expect("rattachement de la thématique");
}

/// Un membre du comité de l'appel.
async fn membre_du_comite(bac: &Bac, appel: Uuid, courriel: &str, prenom: &str) -> Uuid {
    let personne = commun::personne(bac, courriel, prenom, "Comite").await;
    sqlx::query!(
        "INSERT INTO event.call_reviewers (call_id, person_id) VALUES ($1, $2)",
        appel,
        personne
    )
    .execute(bac.pool())
    .await
    .expect("inscription au comité");
    personne
}

/// Une affectation **en retard** : échéance passée, aucune revue déposée.
async fn affectation_en_retard(bac: &Bac, dossier: Uuid, membre: Uuid) {
    sqlx::query!(
        "INSERT INTO programme.review_assignments (proposal_id, reviewer_id, due_at)
         VALUES ($1, $2, now() - interval '2 days')",
        dossier,
        membre
    )
    .execute(bac.pool())
    .await
    .expect("insertion de l'affectation");
}

/// Un accusé de lecture, par la fonction du modèle.
async fn marquer_lu(bac: &Bac, dossier: Uuid, personne: Uuid) {
    sqlx::query!(
        "SELECT programme.record_proposal_read($1, $2)",
        dossier,
        personne
    )
    .execute(bac.pool())
    .await
    .expect("accusé de lecture");
}

/// L'effacement **logique** d'un dossier.
async fn effacer(bac: &Bac, dossier: Uuid) {
    sqlx::query!(
        "UPDATE programme.proposals SET deleted_at = now() WHERE id = $1",
        dossier
    )
    .execute(bac.pool())
    .await
    .expect("effacement du dossier");
}

// -----------------------------------------------------------------------------
// T089 — le périmètre, et six identifiants forgés
// -----------------------------------------------------------------------------

/// **Les trois états du périmètre ne se confondent pas.**
///
/// Global : tout. Détaché : son édition, et le refus ailleurs. Vide : un refus
/// **explicite**, jamais une liste vide — « rien à traiter » et « vous n'avez
/// aucun droit » ne se disent pas de la même façon (principe V).
#[tokio::test]
async fn les_trois_cas_du_perimetre_se_distinguent() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let autre_edition = commun::edition_secondaire(&bac).await;
    commun::dossier(&bac, &terrain, "Atelier adaptation", "atelier-adaptation").await;

    let ecran = list::ecran(&bac.state, &globale(&bac).await, EventId(terrain.edition))
        .await
        .expect("l'administratrice globale lit toutes les éditions");
    assert_eq!(ecran.rows.len(), 1);

    let detachee_ici = detachee(&bac, terrain.edition, "ici@ifdd.francophonie.org").await;
    let ecran = list::ecran(&bac.state, &detachee_ici, EventId(terrain.edition))
        .await
        .expect("l'administratrice détachée lit son édition");
    assert_eq!(ecran.rows.len(), 1);

    let refus = list::ecran(&bac.state, &detachee_ici, EventId(autre_edition))
        .await
        .expect_err("l'édition qu'elle n'administre pas doit être refusée");
    assert_eq!(refus.code, ErrorCode::NotFound);

    // **Un refus, pas une liste vide.** C'est la seule assertion de ce test qui
    // porte sur un code différent, et c'est délibéré : confondre les deux
    // afficherait « aucun dossier » à qui n'a aucun droit.
    let refus = list::ecran(
        &bac.state,
        &sans_droits(&bac).await,
        EventId(terrain.edition),
    )
    .await
    .expect_err("un périmètre vide se refuse");
    assert_eq!(refus.code, ErrorCode::Forbidden);
}

/// **Six identifiants forgés, un seul refus.**
///
/// Une administratrice **globale** franchit le contrôle de périmètre pour
/// n'importe quel identifiant : c'est donc elle qui éprouve le second contrôle,
/// l'existence de l'édition. Quatre des six désignent des objets bien réels —
/// un dossier, une organisation, une personne, un appel —, et c'est ce qui rend
/// le test discriminant : un service qui répondrait « existe » pour ceux-là
/// aurait laissé fuiter la structure de la base.
///
/// **C'est la comparaison qui compte**, pas le code pris isolément.
#[tokio::test]
async fn six_identifiants_forges_menent_au_meme_refus_quun_inexistant() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = commun::dossier(&bac, &terrain, "Atelier adaptation", "atelier-adaptation").await;
    let perimetre = globale(&bac).await;

    let inexistant = list::ecran(&bac.state, &perimetre, EventId(Uuid::now_v7()))
        .await
        .expect_err("une édition inexistante se refuse");
    assert_eq!(inexistant.code, ErrorCode::NotFound);

    for (quoi, forge) in [
        ("un dossier", dossier),
        ("une organisation", terrain.organisation),
        ("une personne", terrain.deposante),
        ("un appel", terrain.appel),
        ("un identifiant nul", Uuid::nil()),
        ("un identifiant tiré au hasard", Uuid::now_v7()),
    ] {
        let refus = list::ecran(&bac.state, &perimetre, EventId(forge))
            .await
            .err()
            .unwrap_or_else(|| {
                panic!("{quoi} : forgé en identifiant d'édition, il doit être refusé")
            });

        assert_eq!(refus.code, inexistant.code, "{quoi}");
        assert_eq!(refus.message, inexistant.message, "{quoi}");
        assert_eq!(refus.field, inexistant.field, "{quoi}");
    }
}

// -----------------------------------------------------------------------------
// T090 — les décomptes correspondent EXACTEMENT aux lignes rendues
// -----------------------------------------------------------------------------

/// **Chaque facette est recomptée sur les lignes de la réponse.**
///
/// C'est tout l'objet de R16 : les facettes ne sont pas une seconde requête,
/// elles sont un décompte sur ce que l'écran affiche. Le test ne compare donc
/// pas à des nombres écrits à la main — il **recompte**, filtre par filtre, et
/// exige l'égalité. Un jour où la composition ferait deux lectures, ce test
/// tomberait pour la seule raison qui vaille : les deux mesures auraient été
/// prises à des instants différents.
#[tokio::test]
async fn les_decomptes_des_facettes_correspondent_aux_lignes() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let perimetre = globale(&bac).await;

    let depose = dossier_dans_letat(
        &bac,
        &terrain,
        "Adaptation côtière",
        "adaptation-cotiere",
        "submitted",
    )
    .await;
    let evalue = dossier_dans_letat(
        &bac,
        &terrain,
        "Finance climat",
        "finance-climat",
        "under_review",
    )
    .await;
    let retenu = dossier_dans_letat(
        &bac,
        &terrain,
        "Forêts et carbone",
        "forets-carbone",
        "accepted",
    )
    .await;
    let brouillon = dossier_dans_letat(
        &bac,
        &terrain,
        "Note de travail",
        "note-de-travail",
        "draft",
    )
    .await;

    thematique(&bac, depose, "adaptation").await;
    thematique(&bac, evalue, "adaptation").await;
    thematique(&bac, evalue, "mitigation").await;

    let noteur =
        membre_du_comite(&bac, terrain.appel, "noteuse@ifdd.francophonie.org", "Nour").await;
    affectation_en_retard(&bac, depose, noteur).await;
    marquer_lu(&bac, retenu, perimetre.person_id).await;

    let ecran = list::ecran(&bac.state, &perimetre, EventId(terrain.edition))
        .await
        .expect("l'écran de liste");

    assert_eq!(ecran.rows.len(), 4);
    assert_eq!(ecran.timezone, commun::FUSEAU_COP31);
    assert_eq!(ecran.city.as_deref(), Some("Belém"));
    assert_eq!(ecran.required_reviews, Some(2));
    assert!(ecran.deadline.is_some(), "l'échéance effective de l'appel");

    let compte = |facettes: &[programme::domain::facets::ProposalFacet], valeur: &str| {
        facettes
            .iter()
            .find(|f| f.value == valeur)
            .map(|f| f.count)
            .unwrap_or(0)
    };

    for facette in &ecran.facets.statuses {
        let attendu = ecran
            .rows
            .iter()
            .filter(|r| r.status == facette.value)
            .count() as i64;
        assert_eq!(facette.count, attendu, "statut {}", facette.value);
    }
    for facette in &ecran.facets.formats {
        let attendu = ecran
            .rows
            .iter()
            .filter(|r| r.format == facette.value)
            .count() as i64;
        assert_eq!(facette.count, attendu, "format {}", facette.value);
    }
    for facette in &ecran.facets.themes {
        let attendu = ecran
            .rows
            .iter()
            .filter(|r| r.theme_codes.contains(&facette.value))
            .count() as i64;
        assert_eq!(facette.count, attendu, "thématique {}", facette.value);
    }
    for facette in &ecran.facets.countries {
        let attendu = ecran
            .rows
            .iter()
            .filter(|r| r.organization_country_code.as_deref() == Some(facette.value.as_str()))
            .count() as i64;
        assert_eq!(facette.count, attendu, "pays {}", facette.value);
    }
    for facette in &ecran.facets.organizations {
        let attendu = ecran
            .rows
            .iter()
            .filter(|r| r.organization_id.to_string() == facette.value)
            .count() as i64;
        assert_eq!(facette.count, attendu, "organisation {}", facette.value);
    }
    for facette in &ecran.facets.reviewers {
        let attendu = ecran
            .rows
            .iter()
            .filter(|r| {
                r.reviewer_ids
                    .iter()
                    .any(|id| id.to_string() == facette.value)
            })
            .count() as i64;
        assert_eq!(facette.count, attendu, "membre du comité {}", facette.value);
    }

    // **Les trois signaux ne sont pas des statuts.** Un dossier « non évalué »
    // peut être déposé ou en évaluation ; le brouillon en est exclu — il n'a
    // jamais été soumis à personne.
    assert_eq!(compte(&ecran.facets.flags, FLAG_UNREVIEWED), 3);
    assert_eq!(compte(&ecran.facets.flags, FLAG_LATE), 1);
    // Un seul dossier a été ouvert : les trois autres sont non lus.
    assert_eq!(compte(&ecran.facets.flags, FLAG_UNREAD), 3);
    assert_eq!(ecran.unread_ids.len(), 3);
    assert!(!ecran.unread_ids.contains(&retenu));
    let _ = brouillon;

    // L'ordre du cycle de vie, et non celui de la popularité : « déposé »
    // précède « en évaluation », qui précède « retenu ».
    let ordre: Vec<&str> = ecran
        .facets
        .statuses
        .iter()
        .map(|f| f.value.as_str())
        .collect();
    assert_eq!(
        ordre,
        vec!["draft", "submitted", "under_review", "accepted"]
    );
}

// -----------------------------------------------------------------------------
// T092 — l'effacé n'y figure pas, et les deux titres sont de deux types
// -----------------------------------------------------------------------------

/// **Un dossier effacé n'est pas une ligne grisée : il n'est pas là.**
///
/// Et le titre voyage **deux fois, sous deux types** : un document multilingue
/// à résoudre à l'affichage, une chaîne déjà résolue pour trier et exporter.
/// Une version antérieure de la vue portait les deux sous le même nom, et
/// l'utilitaire du front rendait alors une chaîne vide **sans erreur**.
#[tokio::test]
async fn un_dossier_efface_disparait_et_le_titre_porte_deux_types() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let perimetre = globale(&bac).await;

    let garde = commun::dossier(&bac, &terrain, "Atelier adaptation", "atelier-adaptation").await;
    let efface = commun::dossier(&bac, &terrain, "Dossier retiré", "dossier-retire").await;
    effacer(&bac, efface).await;

    let ecran = list::ecran(&bac.state, &perimetre, EventId(terrain.edition))
        .await
        .expect("l'écran de liste");

    assert_eq!(ecran.rows.len(), 1);
    assert_eq!(ecran.rows[0].id, garde);
    assert!(
        !ecran.unread_ids.contains(&efface),
        "la fonction des non-lus exclut l'effacé"
    );

    let ligne = &ecran.rows[0];
    assert!(
        ligne.title.is_object(),
        "`title` est le document multilingue brut, pas une chaîne"
    );
    assert_eq!(ligne.title["fr"], "Atelier adaptation");
    assert_eq!(
        ligne.title_text.as_deref(),
        Some("Atelier adaptation"),
        "`title_text` est la résolution française, réservée au tri et à l'export"
    );
}

// -----------------------------------------------------------------------------
// T091 — une sélection hétérogène rend compte de CHAQUE dossier
// -----------------------------------------------------------------------------

/// **Appliqués + écartés = taille de la sélection.** Toujours.
///
/// Le défaut classique d'une action de masse est de répondre « 6 dossiers
/// traités » sans dire ce qu'il est advenu des six autres. Ici, la sélection
/// traverse quatre cas : un dossier libre, un déjà confié, un dont le membre
/// s'est déporté, et un identifiant qui ne désigne rien.
///
/// **L'écart du déporté et celui du déjà confié ne se confondent pas** :
/// réattribuer un dossier dont on s'est retiré effacerait une déclaration
/// d'impartialité, et l'écran doit pouvoir le dire.
#[tokio::test]
async fn une_selection_heterogene_rend_compte_de_chaque_dossier() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    // `admin` sur l'édition détient `event.call.manage` : composer le comité et
    // répartir sa charge sont le même geste (écart n° 48).
    let perimetre = commun::perimetre_de(&bac, droits.decideur).await;

    let membre =
        membre_du_comite(&bac, terrain.appel, "membre@ifdd.francophonie.org", "Marie").await;

    let libre = commun::dossier(&bac, &terrain, "Dossier libre", "dossier-libre").await;
    let deja = commun::dossier(&bac, &terrain, "Dossier confié", "dossier-confie").await;
    let deporte = commun::dossier(&bac, &terrain, "Dossier déporté", "dossier-deporte").await;
    let inexistant = Uuid::now_v7();

    sqlx::query!(
        "INSERT INTO programme.review_assignments (proposal_id, reviewer_id) VALUES ($1, $2)",
        deja,
        membre
    )
    .execute(bac.pool())
    .await
    .expect("affectation préalable");

    sqlx::query!(
        "INSERT INTO programme.review_assignments
             (proposal_id, reviewer_id, recused_at, recusal_reason)
         VALUES ($1, $2, now(), 'Je siège au conseil de cette organisation.')",
        deporte,
        membre
    )
    .execute(bac.pool())
    .await
    .expect("déport préalable");

    let selection = vec![libre, deja, deporte, inexistant];
    let resultat = list::confier_en_groupe(
        &bac.state,
        &bac.ctx(),
        &perimetre,
        AssignReviewerPayload {
            proposal_ids: selection.clone(),
            reviewer_id: membre,
            due_at: None,
        },
    )
    .await
    .expect("l'action groupée aboutit, même quand rien ne suit");

    assert_eq!(
        resultat.applied.len() + resultat.skipped.len(),
        selection.len(),
        "chaque dossier de la sélection doit ressortir, appliqué ou écarté"
    );
    assert_eq!(resultat.applied, vec![libre]);

    let raison = |dossier: Uuid| {
        resultat
            .skipped
            .iter()
            .find(|e| e.proposal_id == dossier)
            .map(|e| e.reason)
    };
    assert_eq!(raison(deja), Some(RaisonDEcart::AlreadyAssigned));
    assert_eq!(raison(deporte), Some(RaisonDEcart::Recused));
    assert_eq!(raison(inexistant), Some(RaisonDEcart::NotFound));

    // Le numéro de dossier accompagne l'écart quand il existe : c'est lui que
    // l'équipe lit, pas l'identifiant technique.
    let confie = resultat
        .skipped
        .iter()
        .find(|e| e.proposal_id == deja)
        .expect("l'écart du dossier déjà confié");
    assert!(confie.reference_code.starts_with("COP31-"));
    let introuvable = resultat
        .skipped
        .iter()
        .find(|e| e.proposal_id == inexistant)
        .expect("l'écart du dossier introuvable");
    assert!(
        introuvable.reference_code.is_empty(),
        "un dossier qui n'existe pas n'a pas de numéro à rendre"
    );
}

/// **Hors périmètre et inexistant rendent le même écart.**
///
/// Sans cela, une sélection forgée dirait à qui la forge que le dossier existe
/// ailleurs — le principe IX, appliqué à l'intérieur d'une réponse 200.
#[tokio::test]
async fn un_dossier_hors_perimetre_secarte_comme_un_inexistant() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let autre_edition = commun::edition_secondaire(&bac).await;
    let dossier = commun::dossier(&bac, &terrain, "Atelier adaptation", "atelier-adaptation").await;

    let ailleurs = detachee(&bac, autre_edition, "ailleurs@ifdd.francophonie.org").await;
    let membre =
        membre_du_comite(&bac, terrain.appel, "membre@ifdd.francophonie.org", "Marie").await;
    let inexistant = Uuid::now_v7();

    let resultat = list::confier_en_groupe(
        &bac.state,
        &bac.ctx(),
        &ailleurs,
        AssignReviewerPayload {
            proposal_ids: vec![dossier, inexistant],
            reviewer_id: membre,
            due_at: None,
        },
    )
    .await
    .expect("l'action groupée aboutit");

    assert!(resultat.applied.is_empty());
    assert_eq!(resultat.skipped.len(), 2);
    for ecart in &resultat.skipped {
        assert_eq!(ecart.reason, RaisonDEcart::NotFound);
        assert!(
            ecart.reference_code.is_empty(),
            "le numéro ne sort pas non plus"
        );
    }
}

// -----------------------------------------------------------------------------
// T093 — un événement PAR DOSSIER, jamais un pour le lot
// -----------------------------------------------------------------------------

/// **Douze dossiers confiés émettent douze événements.**
///
/// Les **compter** est le seul contrôle qui dise quelque chose : vérifier leur
/// présence n'en dirait rien. Un consommateur qui recevrait un lot devrait le
/// déplier lui-même, et son échec porterait alors sur douze effets au lieu
/// d'un — la garde de rejeu est par événement.
///
/// **Et aucun événement d'état n'est émis** : confier ne change aucun état, le
/// déclencheur d'état ne s'éveille donc pas. Le décompte par dossier vaut un.
#[tokio::test]
async fn laffectation_groupee_de_douze_dossiers_emet_douze_evenements() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let perimetre = commun::perimetre_de(&bac, droits.decideur).await;
    let membre =
        membre_du_comite(&bac, terrain.appel, "membre@ifdd.francophonie.org", "Marie").await;

    let mut dossiers = Vec::new();
    for numero in 1..=12 {
        dossiers.push(
            commun::dossier(
                &bac,
                &terrain,
                &format!("Dossier {numero}"),
                &format!("dossier-{numero}"),
            )
            .await,
        );
    }

    let resultat = list::confier_en_groupe(
        &bac.state,
        &bac.ctx(),
        &perimetre,
        AssignReviewerPayload {
            proposal_ids: dossiers.clone(),
            reviewer_id: membre,
            due_at: None,
        },
    )
    .await
    .expect("l'affectation groupée");

    assert_eq!(resultat.applied.len(), 12);
    assert!(resultat.skipped.is_empty());

    let mut total = 0;
    for dossier in &dossiers {
        let emis = commun::evenements_emis(&bac, *dossier).await;
        let affectations = emis
            .iter()
            .filter(|t| t.as_str() == "programme.review.assigned")
            .count();
        assert_eq!(affectations, 1, "un événement d'affectation, et un seul");
        total += affectations;
    }
    assert_eq!(total, 12, "douze événements, jamais un pour le lot");

    // La charge du comité suit : ce que l'écran affiche avant de confier douze
    // dossiers de plus est ce qui vient d'être écrit.
    let comite = list::comite(&bac.state, &perimetre, EventId(terrain.edition))
        .await
        .expect("la composition du comité");
    let charge = comite
        .iter()
        .find(|f| f.value == membre.to_string())
        .expect("le membre du comité");
    assert_eq!(charge.count, 12);
}
