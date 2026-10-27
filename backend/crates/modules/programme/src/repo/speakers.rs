//! Les intervenants d'un dossier, et **leurs deux instantanés**.
//!
//! # Ce que le modèle distingue, et que le code ne doit pas confondre
//!
//! `job_title_snapshot` et `organization_snapshot` sont la fonction et
//! l'organisation **au moment de cette activité**, explicitement distinctes de
//! la fiche de la personne : « une personne change d'employeur, l'archive de la
//! COP28 ne doit pas être réécrite pour autant ».
//!
//! Ils restent donc **modifiables même quand l'identité est verrouillée** — un
//! déposant peut corriger « Directrice » en « Directrice générale » sur son
//! dossier sans toucher au profil de quiconque (écart n° 31).
//!
//! La biographie suit le même régime : `proposal_speakers.bio` appartient au
//! dossier, pas à la personne. Le contrat du front la range parmi les champs
//! d'identité ; le **modèle** la range sur la ligne d'intervenant, et c'est le
//! modèle qui fait autorité.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::ids::ProposalId;

/// Un intervenant à écrire, la personne déjà résolue.
///
/// Sans personne (D16, intervenant sans adresse), la ligne porte elle-même
/// civilité, prénom et nom ; avec une personne, ces trois champs restent nuls,
/// `ck_proposal_speakers_identity` l'exige.
pub struct Intervenant<'a> {
    pub person_id: Option<Uuid>,
    pub civility: Option<&'a str>,
    pub first_name: Option<&'a str>,
    pub last_name: Option<&'a str>,
    pub role: &'a str,
    pub job_title_snapshot: Option<&'a str>,
    pub organization_snapshot: Option<&'a str>,
    pub organization_id: Option<Uuid>,
    pub bio: Option<serde_json::Value>,
    pub sort_order: i16,
}

/// Remplacer les intervenants d'un dossier, **exactement ceux-là**, dans
/// **cet** ordre.
///
/// L'unicité est `(dossier, personne, rôle)` : la même personne peut intervenir
/// deux fois avec deux rôles — modératrice d'un panel et intervenante d'un
/// autre —, et c'est ce que la clé autorise. Le conflit met donc à jour les
/// instantanés plutôt que d'échouer.
///
/// Une ligne sans personne (D16) se reconnaît à son prénom, son nom et son
/// rôle : mise à jour sur place, jamais retirée puis réécrite, car sa photo
/// s'accroche à elle (`media.attachments`, rôle `avatar`).
pub async fn remplacer(
    conn: &mut PgConnection,
    dossier: ProposalId,
    intervenants: &[Intervenant<'_>],
) -> Result<()> {
    let personnes: Vec<Option<Uuid>> = intervenants.iter().map(|i| i.person_id).collect();
    let roles: Vec<String> = intervenants.iter().map(|i| i.role.to_owned()).collect();
    let civilites: Vec<Option<String>> = intervenants
        .iter()
        .map(|i| i.civility.map(str::to_owned))
        .collect();
    let prenoms: Vec<Option<String>> = intervenants
        .iter()
        .map(|i| i.first_name.map(str::to_owned))
        .collect();
    let noms: Vec<Option<String>> = intervenants
        .iter()
        .map(|i| i.last_name.map(str::to_owned))
        .collect();

    // Le retrait porte sur le COUPLE personne–rôle, pas sur la personne seule :
    // retirer par personne effacerait la ligne « modératrice » de qui reste
    // « intervenante ».
    sqlx::query!(
        r#"DELETE FROM programme.proposal_speakers s
            WHERE s.proposal_id = $1
              AND NOT EXISTS (
                  SELECT 1 FROM unnest($2::uuid[], $3::text[], $4::text[], $5::text[])
                                AS g(personne, role, prenom, nom)
                   WHERE g.role::programme.speaker_role = s.role
                     AND (g.personne = s.person_id
                          OR (g.personne IS NULL AND s.person_id IS NULL
                              AND g.prenom = s.first_name AND g.nom = s.last_name))
              )"#,
        dossier.as_uuid(),
        &personnes as &[Option<Uuid>],
        &roles,
        &prenoms as &[Option<String>],
        &noms as &[Option<String>]
    )
    .execute(&mut *conn)
    .await?;

    if intervenants.is_empty() {
        return Ok(());
    }

    let fonctions: Vec<Option<String>> = intervenants
        .iter()
        .map(|i| i.job_title_snapshot.map(str::to_owned))
        .collect();
    let organisations: Vec<Option<String>> = intervenants
        .iter()
        .map(|i| i.organization_snapshot.map(str::to_owned))
        .collect();
    let org_ids: Vec<Option<Uuid>> = intervenants.iter().map(|i| i.organization_id).collect();
    let bios: Vec<Option<serde_json::Value>> = intervenants.iter().map(|i| i.bio.clone()).collect();
    let rangs: Vec<i16> = intervenants.iter().map(|i| i.sort_order).collect();

    sqlx::query!(
        r#"WITH e AS (
               SELECT * FROM unnest($2::uuid[], $3::text[], $4::text[], $5::text[],
                                    $6::uuid[], $7::jsonb[], $8::smallint[],
                                    $9::text[], $10::text[], $11::text[])
                      AS e(personne, role, fonction, organisation, org_id, bio, rang,
                           civilite, prenom, nom)
           ),
           sans_personne AS (
               UPDATE programme.proposal_speakers s
                  SET civility = e.civilite,
                      job_title_snapshot = e.fonction,
                      organization_snapshot = e.organisation,
                      organization_id = e.org_id,
                      bio = e.bio::jsonb::platform.i18n_text,
                      sort_order = e.rang
                 FROM e
                WHERE s.proposal_id = $1 AND s.person_id IS NULL AND e.personne IS NULL
                  AND s.role = e.role::programme.speaker_role
                  AND s.first_name = e.prenom AND s.last_name = e.nom
               RETURNING s.role, s.first_name, s.last_name
           )
           INSERT INTO programme.proposal_speakers
               (proposal_id, person_id, role, job_title_snapshot, organization_snapshot,
                organization_id, bio, sort_order, civility, first_name, last_name)
           SELECT $1, e.personne, e.role::programme.speaker_role,
                  e.fonction, e.organisation, e.org_id,
                  e.bio::jsonb::platform.i18n_text, e.rang,
                  e.civilite, e.prenom, e.nom
             FROM e
            WHERE e.personne IS NOT NULL
               OR NOT EXISTS (SELECT 1 FROM sans_personne d
                               WHERE d.role = e.role::programme.speaker_role
                                 AND d.first_name = e.prenom AND d.last_name = e.nom)
           ON CONFLICT (proposal_id, person_id, role) DO UPDATE
               SET job_title_snapshot = EXCLUDED.job_title_snapshot,
                   organization_snapshot = EXCLUDED.organization_snapshot,
                   organization_id = EXCLUDED.organization_id,
                   bio = EXCLUDED.bio,
                   sort_order = EXCLUDED.sort_order"#,
        dossier.as_uuid(),
        &personnes as &[Option<Uuid>],
        &roles,
        &fonctions as &[Option<String>],
        &organisations as &[Option<String>],
        &org_ids as &[Option<Uuid>],
        &bios as &[Option<serde_json::Value>],
        &rangs,
        &civilites as &[Option<String>],
        &prenoms as &[Option<String>],
        &noms as &[Option<String>]
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Le nombre d'intervenants d'un dossier — ce que les bornes de l'appel
/// comparent, et **qu'aucun déclencheur ne vérifie** (écart n° 27).
pub async fn compter(conn: &mut PgConnection, dossier: ProposalId) -> Result<i64> {
    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.proposal_speakers WHERE proposal_id = $1"#,
        dossier.as_uuid()
    )
    .fetch_one(conn)
    .await?;

    Ok(n)
}

// -----------------------------------------------------------------------------
// La lecture — les intervenants annoncés
// -----------------------------------------------------------------------------

/// Un intervenant, tel que la table le porte — `ProposalSpeaker`.
///
/// **Les deux instantanés voyagent** : fonction et organisation **au moment de
/// cette activité**. Une personne change d'employeur ; l'archive d'une COP
/// passée ne doit pas être réécrite, et c'est pourquoi le modèle les distingue
/// explicitement de la fiche de la personne.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct IntervenantLu {
    pub id: Uuid,
    pub proposal_id: Uuid,
    /// Nul pour un intervenant sans adresse (D16) : l'identité est alors sur la
    /// ligne, dans `civility`, `first_name` et `last_name`.
    pub person_id: Option<Uuid>,
    pub civility: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: String,
    pub job_title_snapshot: Option<String>,
    pub organization_snapshot: Option<String>,
    pub organization_id: Option<Uuid>,
    pub bio: Option<serde_json::Value>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub confirmed_at: Option<time::OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub confirmation_sent_at: Option<time::OffsetDateTime>,
    pub is_available_for_questions: bool,
    pub sort_order: i16,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: time::OffsetDateTime,
}

/// Les intervenants d'un dossier, dans l'ordre annoncé.
pub async fn du_dossier<'e>(
    executor: impl sqlx::PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<Vec<IntervenantLu>> {
    let lignes = sqlx::query!(
        r#"SELECT id, proposal_id, person_id, role::text AS "role!",
                  job_title_snapshot, organization_snapshot, organization_id,
                  bio, confirmed_at, confirmation_sent_at,
                  is_available_for_questions, sort_order, created_at,
                  civility, first_name, last_name
             FROM programme.proposal_speakers
            WHERE proposal_id = $1
            ORDER BY sort_order, created_at"#,
        dossier.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| IntervenantLu {
            id: l.id,
            proposal_id: l.proposal_id,
            person_id: l.person_id,
            civility: l.civility,
            first_name: l.first_name,
            last_name: l.last_name,
            role: l.role,
            job_title_snapshot: l.job_title_snapshot,
            organization_snapshot: l.organization_snapshot,
            organization_id: l.organization_id,
            bio: l.bio,
            confirmed_at: l.confirmed_at,
            confirmation_sent_at: l.confirmation_sent_at,
            is_available_for_questions: l.is_available_for_questions,
            sort_order: l.sort_order,
            created_at: l.created_at,
        })
        .collect())
}
