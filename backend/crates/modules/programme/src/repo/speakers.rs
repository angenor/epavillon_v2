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

use std::collections::HashSet;

use kernel::error::{ApiError, ErrorCode, Result};
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::ids::ProposalId;

/// Un intervenant à écrire, la personne déjà résolue.
///
/// Sans personne (D16, intervenant sans adresse), la ligne porte elle-même
/// civilité, prénom et nom ; avec une personne, ces trois champs restent nuls,
/// `ck_proposal_speakers_identity` l'exige.
pub struct Intervenant<'a> {
    /// La ligne que l'écran corrige, telle que la relecture l'a rendue.
    pub ligne: Option<Uuid>,
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

/// Les colonnes d'une liste d'intervenants, prêtes pour `unnest`.
struct Colonnes {
    lignes: Vec<Uuid>,
    personnes: Vec<Option<Uuid>>,
    roles: Vec<String>,
    fonctions: Vec<Option<String>>,
    organisations: Vec<Option<String>>,
    org_ids: Vec<Option<Uuid>>,
    bios: Vec<Option<serde_json::Value>>,
    rangs: Vec<i16>,
    civilites: Vec<Option<String>>,
    prenoms: Vec<Option<String>>,
    noms: Vec<Option<String>>,
}

impl Colonnes {
    fn de<'a, 'b: 'a>(
        intervenants: impl Iterator<Item = (Option<Uuid>, &'a Intervenant<'b>)>,
    ) -> Self {
        let mut c = Colonnes {
            lignes: Vec::new(),
            personnes: Vec::new(),
            roles: Vec::new(),
            fonctions: Vec::new(),
            organisations: Vec::new(),
            org_ids: Vec::new(),
            bios: Vec::new(),
            rangs: Vec::new(),
            civilites: Vec::new(),
            prenoms: Vec::new(),
            noms: Vec::new(),
        };
        for (ligne, i) in intervenants {
            if let Some(ligne) = ligne {
                c.lignes.push(ligne);
            }
            c.personnes.push(i.person_id);
            c.roles.push(i.role.to_owned());
            c.fonctions.push(i.job_title_snapshot.map(str::to_owned));
            c.organisations
                .push(i.organization_snapshot.map(str::to_owned));
            c.org_ids.push(i.organization_id);
            c.bios.push(i.bio.clone());
            c.rangs.push(i.sort_order);
            c.civilites.push(i.civility.map(str::to_owned));
            c.prenoms.push(i.first_name.map(str::to_owned));
            c.noms.push(i.last_name.map(str::to_owned));
        }
        c
    }
}

/// Remplacer les intervenants d'un dossier, **exactement ceux-là**, dans
/// **cet** ordre.
///
/// L'unicité est `(dossier, personne, rôle)` : la même personne peut intervenir
/// deux fois avec deux rôles (modératrice d'un panel et intervenante d'un
/// autre), et c'est ce que la clé autorise. Le conflit met donc à jour les
/// instantanés plutôt que d'échouer.
///
/// **Une ligne désignée par l'écran est corrigée sur place** : identité,
/// personne, rôle, instantanés, ordre. Sa photo (`media.attachments`, rôle
/// `avatar`) s'accroche à son identifiant ; la retrouver par le nom ferait
/// d'une coquille corrigée une ligne nouvelle, et la photo resterait sur
/// l'ancienne, retirée.
///
/// Sans désignation, une ligne sans personne (D16) se reconnaît à son prénom,
/// son nom et son rôle, une ligne avec personne au couple personne–rôle.
pub async fn remplacer(
    conn: &mut PgConnection,
    dossier: ProposalId,
    intervenants: &[Intervenant<'_>],
) -> Result<()> {
    let mut couples = HashSet::new();
    for i in intervenants {
        if let Some(personne) = i.person_id {
            if !couples.insert((personne, i.role)) {
                return Err(ApiError::with_message(
                    ErrorCode::ValidationFailed,
                    "Une même personne ne peut figurer qu'une fois avec le même rôle.",
                )
                .field("speakers"));
            }
        }
    }

    let existantes = sqlx::query_scalar!(
        "SELECT id FROM programme.proposal_speakers WHERE proposal_id = $1",
        dossier.as_uuid()
    )
    .fetch_all(&mut *conn)
    .await?;

    // Une ligne d'un autre dossier, ou désignée deux fois, ne désigne rien.
    let mut retenues = HashSet::new();
    let designees: Vec<Option<Uuid>> = intervenants
        .iter()
        .map(|i| {
            i.ligne
                .filter(|id| existantes.contains(id) && retenues.insert(*id))
        })
        .collect();

    let sur_place = Colonnes::de(
        designees
            .iter()
            .zip(intervenants)
            .filter(|(l, _)| l.is_some())
            .map(|(l, i)| (*l, i)),
    );
    let rapprochees = Colonnes::de(
        designees
            .iter()
            .zip(intervenants)
            .filter(|(l, _)| l.is_none())
            .map(|(_, i)| (None, i)),
    );
    let gardees = &sur_place.lignes;

    // Le retrait porte sur le COUPLE personne–rôle, pas sur la personne seule :
    // retirer par personne effacerait la ligne « modératrice » de qui reste
    // « intervenante ».
    sqlx::query!(
        r#"DELETE FROM programme.proposal_speakers s
            WHERE s.proposal_id = $1
              AND s.id <> ALL($6::uuid[])
              AND NOT EXISTS (
                  SELECT 1 FROM unnest($2::uuid[], $3::text[], $4::text[], $5::text[])
                                AS g(personne, role, prenom, nom)
                   WHERE g.role::programme.speaker_role = s.role
                     AND (g.personne = s.person_id
                          OR (g.personne IS NULL AND s.person_id IS NULL
                              AND g.prenom = s.first_name AND g.nom = s.last_name))
              )"#,
        dossier.as_uuid(),
        &rapprochees.personnes as &[Option<Uuid>],
        &rapprochees.roles,
        &rapprochees.prenoms as &[Option<String>],
        &rapprochees.noms as &[Option<String>],
        gardees
    )
    .execute(&mut *conn)
    .await?;

    if !gardees.is_empty() {
        corriger_sur_place(conn, dossier, &sur_place).await?;
    }

    if rapprochees.roles.is_empty() {
        return Ok(());
    }

    let r = &rapprochees;
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
                  AND s.id <> ALL($12::uuid[])
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
        &r.personnes as &[Option<Uuid>],
        &r.roles,
        &r.fonctions as &[Option<String>],
        &r.organisations as &[Option<String>],
        &r.org_ids as &[Option<Uuid>],
        &r.bios as &[Option<serde_json::Value>],
        &r.rangs,
        &r.civilites as &[Option<String>],
        &r.prenoms as &[Option<String>],
        &r.noms as &[Option<String>],
        gardees
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Les lignes désignées, corrigées sur place.
///
/// Une personne qui change n'emporte pas la confirmation de la précédente : la
/// confirmation est celle de quelqu'un, pas de la ligne.
async fn corriger_sur_place(
    conn: &mut PgConnection,
    dossier: ProposalId,
    c: &Colonnes,
) -> Result<()> {
    sqlx::query!(
        r#"UPDATE programme.proposal_speakers s
              SET person_id = e.personne,
                  role = e.role::programme.speaker_role,
                  civility = e.civilite,
                  first_name = e.prenom,
                  last_name = e.nom,
                  job_title_snapshot = e.fonction,
                  organization_snapshot = e.organisation,
                  organization_id = e.org_id,
                  bio = e.bio::jsonb::platform.i18n_text,
                  sort_order = e.rang,
                  confirmed_at = CASE WHEN s.person_id IS NOT DISTINCT FROM e.personne
                                      THEN s.confirmed_at END,
                  confirmation_sent_at = CASE WHEN s.person_id IS NOT DISTINCT FROM e.personne
                                              THEN s.confirmation_sent_at END
             FROM unnest($2::uuid[], $3::uuid[], $4::text[], $5::text[], $6::text[],
                         $7::uuid[], $8::jsonb[], $9::smallint[],
                         $10::text[], $11::text[], $12::text[])
                  AS e(ligne, personne, role, fonction, organisation, org_id, bio, rang,
                       civilite, prenom, nom)
            WHERE s.proposal_id = $1 AND s.id = e.ligne"#,
        dossier.as_uuid(),
        &c.lignes,
        &c.personnes as &[Option<Uuid>],
        &c.roles,
        &c.fonctions as &[Option<String>],
        &c.organisations as &[Option<String>],
        &c.org_ids as &[Option<Uuid>],
        &c.bios as &[Option<serde_json::Value>],
        &c.rangs,
        &c.civilites as &[Option<String>],
        &c.prenoms as &[Option<String>],
        &c.noms as &[Option<String>]
    )
    .execute(&mut *conn)
    .await
    .map_err(|erreur| match &erreur {
        // Seul cas restant après le contrôle des doublons : deux lignes de la
        // même personne qui échangent leurs rôles. L'unicité se vérifie ligne à
        // ligne, pas en fin d'instruction.
        sqlx::Error::Database(e) if e.constraint() == Some("ux_proposal_speakers") => {
            ApiError::with_message(
                ErrorCode::ValidationFailed,
                "Cette personne a déjà une ligne avec ce rôle : changez un rôle à la fois.",
            )
            .field("speakers")
        }
        _ => ApiError::from(erreur),
    })?;

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

/// La photo de chaque ligne d'un dossier : celle posée sur la ligne par
/// l'organisation, à défaut l'avatar de la personne.
///
/// La ligne d'abord : l'organisation choisit la photo de son activité, et elle
/// ne peut pas changer celle d'un compte qui ne lui appartient pas.
pub async fn photos_du_dossier<'e>(
    executor: impl sqlx::PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<Vec<(Uuid, Option<serde_json::Value>)>> {
    let lignes = sqlx::query!(
        r#"SELECT d.id,
                  COALESCE(media.attached_image('programme', 'proposal_speakers', d.id, 'avatar'),
                           media.attached_image('identity', 'people', d.person_id, 'avatar')) AS photo
             FROM programme.proposal_speakers d
            WHERE d.proposal_id = $1"#,
        dossier.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes.into_iter().map(|l| (l.id, l.photo)).collect())
}
