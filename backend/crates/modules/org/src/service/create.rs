//! Créer une fiche sans en fabriquer une deuxième.
//!
//! **Rien n'est bloqué, sauf le doublon exact que la base refuse** — et ce refus
//! nomme la fiche existante, sous la forme d'un résultat de recherche : de quoi
//! la rejoindre. Une simple ressemblance ne bloque jamais : l'écran l'a montrée,
//! la personne a maintenu, et c'est une revue humaine que ça mérite, pas un
//! refus.

use contracts::org as evenements;
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::events::{self, DomainEvent};
use sqlx::postgres::PgConnection;

use crate::domain::ids::PersonId;
use crate::domain::membership::MembershipRole;
use crate::domain::organization::{
    slug_ou_repli, CreateOrganization, CreateOrganizationOutcome, Organization,
};
use crate::domain::search::SimilarOrganization;
use crate::jobs;
use crate::repo::organizations::{self, NewOrganization};
use crate::repo::{memberships, search};
use crate::state::OrgState;

/// Crée une organisation.
///
/// La fiche naît **`candidate`** — jamais `active` : une fiche née d'un
/// formulaire public n'est pas une fiche de référence tant que l'IFDD ne l'a pas
/// regardée. Le créateur en devient **référent actif** : il n'a personne pour
/// l'approuver.
pub async fn create(
    state: &OrgState,
    ctx: &RequestContext,
    acteur: PersonId,
    demande: CreateOrganization,
) -> Result<CreateOrganizationOutcome> {
    let mut tx = state.db().write(ctx).await?;

    let fiche = match inserer(&mut tx, &demande, acteur).await {
        Ok(fiche) => fiche,
        Err(e) if est_nom_deja_pris(&e) => {
            // **La transaction est rendue AVANT de lire la fiche en cause.**
            // L'ordre compte : la lecture prend une seconde connexion, et sous
            // cent créations concurrentes chaque transaction perdante en
            // retiendrait deux — le pool s'épuise et les dernières échouent en
            // « service indisponible » au lieu de recevoir leur refus.
            tx.rollback().await?;

            // Le refus doit **nommer la fiche en cause** : sans elle, la
            // personne ne sait ni ce qui existe ni comment le rejoindre.
            let existante = fiche_en_conflit(state, &demande).await?;
            return Ok(CreateOrganizationOutcome::NameTaken {
                existing: Box::new(existante),
            });
        }
        Err(e) => return Err(e),
    };

    // Le créateur est **référent**, et son adhésion est active d'emblée. La
    // primauté, elle, est attribuée par la base : le service ne la calcule pas.
    let (_, adhesion) = memberships::request(
        &mut tx,
        fiche.id,
        acteur,
        MembershipRole::Manager,
        demande.job_title.as_deref(),
        true,
    )
    .await?;

    events::emit(
        &mut tx,
        DomainEvent {
            aggregate_schema: evenements::AGGREGATE_SCHEMA,
            aggregate_type: evenements::AGGREGATE_ORGANIZATION,
            aggregate_id: fiche.id.as_uuid(),
            event_type: evenements::ORGANIZATION_CREATED,
            payload: serde_json::to_value(evenements::OrganizationCreated {
                organization_id: fiche.id.as_uuid(),
                status: fiche.status.as_str().to_owned(),
                country_id: fiche.country_id,
                organization_type_code: fiche.organization_type_code.clone(),
                // Créer sans rien voir n'est pas la même faute que créer en
                // sachant : le chiffre part avec l'événement, et la revue le lit.
                acknowledged_matches: demande.acknowledged_match_ids.len() as i32,
            })
            .map_err(ApiError::internal)?,
        },
    )
    .await?;

    jobs::planifier_apres_ecriture(&mut tx, state.config(), fiche.id).await?;

    let membership_id = adhesion.id;
    tx.commit().await?;

    Ok(CreateOrganizationOutcome::Created {
        organization: Box::new(fiche),
        membership_id,
        role: MembershipRole::Manager,
    })
}

/// L'insertion, adresse d'URL comprise.
///
/// **La collision d'adresse se rejoue une fois, puis abandonne.** Deux noms
/// voisins produisent la même adresse normalisée — « Réseau climat » et
/// « réseau, climat » — et la suffixer une fois suffit en pratique. Boucler
/// indéfiniment sur une collision transformerait un défaut de données en attente
/// sans fin.
///
/// Chaque tentative est encadrée d'un point de reprise : sans lui, la première
/// violation de contrainte abandonnerait la transaction entière et la seconde
/// tentative n'aurait plus de terrain.
async fn inserer(
    tx: &mut PgConnection,
    demande: &CreateOrganization,
    acteur: PersonId,
) -> Result<Organization> {
    let base = composer_ladresse(tx, &demande.legal_name).await?;

    match nouvelle_fiche(tx, demande, acteur, &base).await {
        Err(e) if est_collision_dadresse(&e) => {
            let suffixe = format!("{base}-{}", &uuid::Uuid::now_v7().simple().to_string()[..6]);
            nouvelle_fiche(tx, demande, acteur, &suffixe)
                .await
                .map_err(|e| {
                    if est_collision_dadresse(&e) {
                        ApiError::internal(
                            "collision d'adresse d'URL après suffixe : l'aléa a échoué deux fois",
                        )
                    } else {
                        e
                    }
                })
        }
        autre => autre,
    }
}

/// Une tentative d'insertion, protégée par un point de reprise.
async fn nouvelle_fiche(
    tx: &mut PgConnection,
    demande: &CreateOrganization,
    acteur: PersonId,
    slug: &str,
) -> Result<Organization> {
    sqlx::query("SAVEPOINT insertion_fiche")
        .execute(&mut *tx)
        .await?;

    let resultat = organizations::create(
        tx,
        NewOrganization {
            legal_name: demande.legal_name.trim(),
            acronym: demande
                .acronym
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty()),
            slug,
            organization_type_code: &demande.organization_type_code,
            country_id: demande.country_id,
            city: demande.city.as_deref(),
            website: demande.website.as_deref(),
            description: demande.description.as_ref(),
            created_by: acteur,
        },
    )
    .await;

    match resultat {
        Ok(fiche) => {
            sqlx::query("RELEASE SAVEPOINT insertion_fiche")
                .execute(tx)
                .await?;
            Ok(fiche)
        }
        Err(erreur) => {
            sqlx::query("ROLLBACK TO SAVEPOINT insertion_fiche")
                .execute(tx)
                .await?;
            Err(erreur)
        }
    }
}

/// L'adresse composée par la base — `platform.slugify`, la même fonction que
/// partout ailleurs. La réécrire en Rust ferait diverger la normalisation d'une
/// virgule, et deux adresses cesseraient de se ressembler.
async fn composer_ladresse(tx: &mut PgConnection, nom: &str) -> Result<String> {
    let compose = sqlx::query_scalar!("SELECT platform.slugify($1)", nom)
        .fetch_one(&mut *tx)
        .await?;

    // Le nom est déjà connu ; l'identifiant, non — on tire donc le repli sur un
    // aléa, qui ne sert que si la normalisation a tout effacé.
    Ok(slug_ou_repli(compose, uuid::Uuid::now_v7()))
}

/// La fiche qui porte déjà ce nom dans ce pays, rendue comme un résultat de
/// recherche : c'est la forme que l'écran sait afficher, et elle porte de quoi
/// la rejoindre.
async fn fiche_en_conflit(
    state: &OrgState,
    demande: &CreateOrganization,
) -> Result<SimilarOrganization> {
    let resultats = search::brute(
        state.pool(),
        search::SearchInput {
            name: demande.legal_name.trim(),
            country_id: demande.country_id,
            email: None,
            website: demande.website.as_deref(),
            limit: 5,
        },
    )
    .await?;

    // Celle qui porte exactement le même nom normalisé dans le même pays : c'est
    // elle que la contrainte a opposée, pas la plus ressemblante.
    let exacte = sqlx::query_scalar!(
        "SELECT id FROM org.organizations
          WHERE legal_name_normalized = platform.normalize_label($1)
            AND COALESCE(country_id, '00000000-0000-0000-0000-000000000000'::uuid)
              = COALESCE($2, '00000000-0000-0000-0000-000000000000'::uuid)
            AND status IN ('candidate', 'active')
          LIMIT 1",
        demande.legal_name.trim(),
        demande.country_id
    )
    .fetch_optional(state.pool())
    .await?;

    resultats
        .into_iter()
        .find(|r| Some(r.organization_id.as_uuid()) == exacte)
        .ok_or_else(|| {
            ApiError::internal(
                "la base a refusé un doublon exact et la fiche en cause reste introuvable",
            )
        })
}

/// `ux_organizations_name_country` : le doublon exact que la base refuse.
fn est_nom_deja_pris(erreur: &ApiError) -> bool {
    contrainte_violee(erreur, "ux_organizations_name_country")
}

/// `ux_organizations_slug` : deux noms voisins ont produit la même adresse.
fn est_collision_dadresse(erreur: &ApiError) -> bool {
    contrainte_violee(erreur, "ux_organizations_slug")
}

/// Le nom de la contrainte violée se lit dans le détail technique, que le noyau
/// y dépose et qui ne franchit jamais la réponse HTTP.
fn contrainte_violee(erreur: &ApiError, nom: &str) -> bool {
    erreur.code == ErrorCode::Conflict && erreur.detail.as_deref().is_some_and(|d| d.contains(nom))
}
