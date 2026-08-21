//! Ce que les tests de la partie « séances » ajoutent au terrain de B4.
//!
//! # Pourquoi un second fichier
//!
//! `commun/mod.rs` porte déjà six cents lignes, et B5 double la surface : une
//! édition complète — jours, salles, canal —, un dossier mené jusqu'à
//! l'évaluation, et de quoi relire une séance en base. Le garde-fou de mille
//! lignes vaut aussi pour les tests, et le découpage suit l'agrégat comme
//! partout ailleurs dans ce crate.
//!
//! # Ce que la fabrique enchaîne, et pourquoi elle est si longue
//!
//! Le semis **ne fournit aucune séance**, et une séance ne naît que d'un dossier
//! retenu. Le parcours le plus court passe donc par cinq modules : édition,
//! jours du calendrier, salles, canal de diffusion, appel, organisation
//! vérifiée, personne membre active, dossier déposé, dossier passé en
//! évaluation. C'est le parcours le plus long du jalon, et le seul qui les
//! traverse tous.

#![allow(dead_code)]

use uuid::Uuid;

use super::{Bac, Terrain};

/// L'édition de référence, **complète** : ses jours, ses deux salles, son canal.
///
/// Les deux salles ne sont pas un ornement : c'est leur différence qui fait la
/// détection des conflits. Une salle **physique** occupe le stand unique et
/// remonte en gravité bloquante ; une salle **virtuelle** ne l'occupe pas, et
/// deux séances y coexistent sans un mot.
pub struct Grille {
    /// Les douze jours de la COP31, du 9 au 20 novembre 2027.
    pub jours: Vec<Uuid>,
    /// Salle physique du pavillon — `is_virtual = false`.
    pub salle: Uuid,
    /// Salle en ligne — `is_virtual = true`.
    pub salle_virtuelle: Uuid,
    /// Canal **par défaut** de l'édition : celui que le déclencheur pose.
    pub canal: Uuid,
    /// Un fil de programmation publié — la « Journée finance durable ».
    pub fil: Uuid,
}

/// Poser jours, lieu, salles, canal et fil sur une édition existante.
pub async fn grille(bac: &Bac, event_id: Uuid) -> Grille {
    let jours = jours_du_calendrier(bac, event_id).await;
    let lieu = lieu(bac, event_id).await;

    Grille {
        jours,
        salle: salle(bac, lieu, "auditorium", "Auditorium", false).await,
        salle_virtuelle: salle(bac, lieu, "en-ligne", "Salle en ligne", true).await,
        canal: canal_par_defaut(bac, event_id).await,
        fil: fil_publie(bac, event_id, "finance_durable", "Journée finance durable").await,
    }
}

/// Les douze jours civils de l'édition, **dans son fuseau**.
///
/// C'est la base qui produit la série : la composer en Rust demanderait de
/// savoir où tombe minuit à Belém, ce que seule la base de fuseaux sait.
pub async fn jours_du_calendrier(bac: &Bac, event_id: Uuid) -> Vec<Uuid> {
    sqlx::query_scalar!(
        r#"INSERT INTO event.event_days (event_id, day_date, sort_order)
           SELECT $1, jour::date, (row_number() OVER (ORDER BY jour))::smallint
             FROM event.events e,
                  generate_series((e.starts_at AT TIME ZONE e.timezone)::date,
                                  (e.ends_at   AT TIME ZONE e.timezone)::date,
                                  interval '1 day') AS jour
            WHERE e.id = $1
        RETURNING id"#,
        event_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("insertion des jours")
}

/// Le pavillon de la Francophonie.
pub async fn lieu(bac: &Bac, event_id: Uuid) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO event.venues (event_id, name, kind)
           VALUES ($1, '{"fr":"Pavillon de la Francophonie"}'::jsonb, 'pavilion')
        RETURNING id"#,
        event_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du lieu")
}

/// Une salle. **`is_virtual` décide de tout** : c'est de lui que le déclencheur
/// dérive l'exclusivité de salle, et donc la gravité du chevauchement.
pub async fn salle(bac: &Bac, venue_id: Uuid, code: &str, nom: &str, virtuelle: bool) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO event.rooms (venue_id, name, code, capacity, is_virtual, has_streaming)
           VALUES ($1, jsonb_build_object('fr', $3::text), $2, 80, $4, true)
        RETURNING id"#,
        venue_id,
        code,
        nom,
        virtuelle
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la salle")
}

/// Le canal **par défaut** de l'édition. Un seul par édition
/// (`ux_broadcast_channels_default`).
pub async fn canal_par_defaut(bac: &Bac, event_id: Uuid) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO event.broadcast_channels (event_id, code, name, is_default)
           VALUES ($1, 'principal', '{"fr":"Chaîne du pavillon"}'::jsonb, true)
        RETURNING id"#,
        event_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du canal")
}

/// Un second canal, **sans être par défaut** : c'est lui qui prouve qu'un canal
/// choisi est retenu tel quel, et non remplacé par le canal d'office.
pub async fn canal_secondaire(bac: &Bac, event_id: Uuid, code: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO event.broadcast_channels (event_id, code, name, is_default)
           VALUES ($1, $2, jsonb_build_object('fr', $2::text), false)
        RETURNING id"#,
        event_id,
        code
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du canal secondaire")
}

/// Un fil de programmation **publié** : la vue publique n'agrège que ceux-là.
pub async fn fil_publie(bac: &Bac, event_id: Uuid, code: &str, titre: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO event.programme_tracks
               (event_id, code, slug, kind, title, color_hex, published_at)
           VALUES ($1, $2, platform.slugify($3)::platform.slug, 'special_day',
                   jsonb_build_object('fr', $3::text), '#00A1E4', now())
        RETURNING id"#,
        event_id,
        code,
        titre
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du fil")
}

// -----------------------------------------------------------------------------
// Un dossier prêt à être retenu
// -----------------------------------------------------------------------------

/// Un dossier **déposé et passé en évaluation**, avec ses intervenants, ses
/// co-organisations et ses thématiques — tout ce que la naissance recopie.
///
/// L'état d'arrivée est `under_review` et non `accepted` : c'est **le service**
/// qui doit franchir la dernière marche, sans quoi le test ne prouve rien.
pub struct DossierPret {
    pub id: Uuid,
    pub intervenants: Vec<Uuid>,
    /// Co-organisations **hors porteur** : celles que le service recopie.
    pub coorganisations: Vec<Uuid>,
    pub themes: Vec<String>,
}

/// Options de composition. Le défaut est le cas courant — une occurrence, un
/// créneau souhaité, une durée déclarée.
pub struct Souhaits {
    pub occurrences: i16,
    /// Créneau souhaité, en **heure murale** de l'édition (`2027-11-12 14:00`).
    pub creneau: Option<&'static str>,
    pub duree_minutes: Option<i16>,
    /// Rattacher le dossier à l'appel de l'édition, ou non (R5).
    pub avec_appel: bool,
}

impl Default for Souhaits {
    fn default() -> Self {
        Self {
            occurrences: 1,
            creneau: Some("2027-11-12 14:00"),
            duree_minutes: Some(90),
            avec_appel: true,
        }
    }
}

/// Composer le dossier, puis le mener jusqu'à l'évaluation.
pub async fn dossier_pret(
    bac: &Bac,
    terrain: &Terrain,
    titre: &str,
    slug: &str,
    souhaits: Souhaits,
) -> DossierPret {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO programme.proposals
               (call_id, event_id, organization_id, submitted_by,
                title, slug, summary, objectives, detailed_presentation, format,
                preferred_start_at, duration_minutes, requested_sessions,
                scheduling_constraints)
           SELECT CASE WHEN $10 THEN $1::uuid END, $2, $3, $4,
                  jsonb_build_object('fr', $5::text),
                  $6::text::platform.slug,
                  jsonb_build_object('fr', 'Un résumé.'),
                  '{"fr":"Objectifs du dossier."}'::jsonb,
                  '{"fr":"<p>Présentation.</p>"}'::jsonb,
                  'hybrid',
                  CASE WHEN $7::text IS NOT NULL
                       THEN ($7::text)::timestamp AT TIME ZONE e.timezone END,
                  $8, $9, 'Pas le matin.'
             FROM event.events e WHERE e.id = $2
        RETURNING id"#,
        terrain.appel,
        terrain.edition,
        terrain.organisation,
        terrain.deposante,
        titre,
        slug,
        souhaits.creneau,
        souhaits.duree_minutes,
        souhaits.occurrences,
        souhaits.avec_appel
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du dossier");

    // **Les identités sont uniques par dossier** : deux dossiers d'un même test
    // partageraient sinon la même adresse électronique, et
    // `ux_people_primary_email` refuserait la seconde.
    let intervenants = vec![
        intervenant_du_dossier(
            bac,
            id,
            &format!("moussa.ba+{slug}@example.org"),
            "Moussa",
            "Ba",
            "speaker",
        )
        .await,
        intervenant_du_dossier(
            bac,
            id,
            &format!("lea.martin+{slug}@example.org"),
            "Léa",
            "Martin",
            "moderator",
        )
        .await,
    ];

    // La ligne du porteur est posée par déclencheur : n'entre ici que ce que le
    // service devra recopier.
    let partenaire =
        super::organisation_verifiee(bac, &format!("Réseau des femmes {slug}"), "RDF").await;
    let coorganisations = vec![partenaire];
    coorganiser(bac, id, partenaire, "co_organizer").await;

    // **Deux codes qui existent réellement** dans `activity_theme` : un code
    // inconnu ne poserait aucun lien, et le test compterait un thème de moins
    // sans dire pourquoi.
    let themes = vec!["adaptation".to_owned(), "climate_finance".to_owned()];
    thematiques_du_dossier(bac, id, &themes).await;

    mener_a_levaluation(bac, id).await;

    DossierPret {
        id,
        intervenants,
        coorganisations,
        themes,
    }
}

/// Un intervenant du dossier, avec sa personne.
pub async fn intervenant_du_dossier(
    bac: &Bac,
    proposal_id: Uuid,
    email: &str,
    prenom: &str,
    nom: &str,
    role: &str,
) -> Uuid {
    let person_id = super::personne(bac, email, prenom, nom).await;

    sqlx::query!(
        r#"INSERT INTO programme.proposal_speakers
               (proposal_id, person_id, role, job_title_snapshot,
                organization_snapshot, bio, confirmed_at, sort_order)
           VALUES ($1, $2, $3::text::programme.speaker_role, 'Directrice',
                   'IFDD', jsonb_build_object('fr', 'Une notice.'), now(), 0)"#,
        proposal_id,
        person_id,
        role
    )
    .execute(bac.pool())
    .await
    .expect("insertion de l'intervenant");

    person_id
}

/// Une co-organisation du dossier.
pub async fn coorganiser(bac: &Bac, proposal_id: Uuid, organization_id: Uuid, role: &str) {
    sqlx::query!(
        "INSERT INTO programme.proposal_organizations (proposal_id, organization_id, role)
         VALUES ($1, $2, $3::text::programme.organization_role)
         ON CONFLICT (proposal_id, organization_id) DO NOTHING",
        proposal_id,
        organization_id,
        role
    )
    .execute(bac.pool())
    .await
    .expect("insertion de la co-organisation");
}

/// Les thématiques du dossier, posées directement dans le référentiel.
pub async fn thematiques_du_dossier(bac: &Bac, proposal_id: Uuid, codes: &[String]) {
    sqlx::query!(
        "INSERT INTO reference.entity_terms
             (entity_schema, entity_table, entity_id, term_id, sort_order)
         SELECT 'programme', 'proposals', $1, t.id, c.rang
           FROM unnest($2::text[]) WITH ORDINALITY AS c(code, rang)
           JOIN reference.taxonomy_terms t
             ON t.code = c.code AND t.taxonomy_code = 'activity_theme'
         ON CONFLICT DO NOTHING",
        proposal_id,
        codes
    )
    .execute(bac.pool())
    .await
    .expect("insertion des thématiques");
}

/// Déposer puis mettre en évaluation — **les deux marches d'avant la décision**.
pub async fn mener_a_levaluation(bac: &Bac, proposal_id: Uuid) {
    for etat in ["submitted", "under_review"] {
        sqlx::query!(
            "UPDATE programme.proposals
                SET status = $2::text::programme.proposal_status
              WHERE id = $1",
            proposal_id,
            etat
        )
        .execute(bac.pool())
        .await
        .unwrap_or_else(|e| panic!("passage en {etat} : {e}"));
    }
}

// -----------------------------------------------------------------------------
// Ce qu'une séance permet d'observer
// -----------------------------------------------------------------------------

/// La ligne d'une séance, telle que la base la porte. Les tests relisent en base
/// plutôt que de croire la réponse.
pub struct LigneDeSeance {
    pub id: Uuid,
    pub event_id: Uuid,
    pub sequence_number: i16,
    pub slug: String,
    pub titre_fr: String,
    pub status: String,
    pub format: String,
    pub timezone: String,
    pub room_id: Option<Uuid>,
    pub event_day_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub enforce_room_exclusivity: bool,
    pub is_streamed: bool,
    pub broadcast_channel_id: Option<Uuid>,
    pub published_at: Option<time::OffsetDateTime>,
    /// Créneau relu **en heure murale**, dans le fuseau de la séance : c'est
    /// l'aller-retour qui compte, un écart de trois heures ne se voit pas
    /// autrement.
    pub debut_mural: String,
    pub fin_murale: String,
}

/// Les séances d'un dossier, par rang d'occurrence.
pub async fn seances_du_dossier(bac: &Bac, proposal_id: Uuid) -> Vec<LigneDeSeance> {
    let lignes = sqlx::query!(
        r#"SELECT s.id, s.event_id, s.sequence_number, s.slug::text AS "slug!",
                  s.title ->> 'fr' AS "titre!", s.status::text AS "status!",
                  s.format::text AS "format!", s.timezone::text AS "timezone!",
                  s.room_id, s.event_day_id, s.organization_id,
                  s.enforce_room_exclusivity, s.is_streamed, s.broadcast_channel_id,
                  s.published_at,
                  to_char(s.starts_at AT TIME ZONE s.timezone,
                          'YYYY-MM-DD"T"HH24:MI') AS "debut!",
                  to_char(s.ends_at AT TIME ZONE s.timezone,
                          'YYYY-MM-DD"T"HH24:MI') AS "fin!"
             FROM programme.sessions s
            WHERE s.proposal_id = $1
            ORDER BY s.sequence_number"#,
        proposal_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des séances");

    lignes
        .into_iter()
        .map(|l| LigneDeSeance {
            id: l.id,
            event_id: l.event_id,
            sequence_number: l.sequence_number,
            slug: l.slug,
            titre_fr: l.titre,
            status: l.status,
            format: l.format,
            timezone: l.timezone,
            room_id: l.room_id,
            event_day_id: l.event_day_id,
            organization_id: l.organization_id,
            enforce_room_exclusivity: l.enforce_room_exclusivity,
            is_streamed: l.is_streamed,
            broadcast_channel_id: l.broadcast_channel_id,
            published_at: l.published_at,
            debut_mural: l.debut,
            fin_murale: l.fin,
        })
        .collect()
}

/// Une séance par son identifiant.
pub async fn seance(bac: &Bac, session_id: Uuid) -> LigneDeSeance {
    let dossier = sqlx::query_scalar!(
        "SELECT proposal_id FROM programme.sessions WHERE id = $1",
        session_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de la séance");

    match dossier {
        Some(p) => seances_du_dossier(bac, p)
            .await
            .into_iter()
            .find(|s| s.id == session_id)
            .expect("la séance de ce dossier"),
        None => panic!("séance sans dossier : hors du périmètre des fabriques"),
    }
}

/// Le jour civil auquel la séance est rattachée, **relu en base**.
///
/// C'est la seule mesure qui vaille pour l'écart n° 113 : croire la réponse ne
/// dirait rien d'une journée restée sur l'ancien jour.
pub async fn jour_de_rattachement(bac: &Bac, session_id: Uuid) -> Option<time::Date> {
    sqlx::query_scalar!(
        "SELECT d.day_date
           FROM programme.sessions s
           JOIN event.event_days d ON d.id = s.event_day_id
          WHERE s.id = $1",
        session_id
    )
    .fetch_optional(bac.pool())
    .await
    .expect("lecture du jour de rattachement")
}

/// Les intervenants d'une séance, par ordre d'affichage.
pub async fn intervenants_de_la_seance(bac: &Bac, session_id: Uuid) -> Vec<(Uuid, String)> {
    sqlx::query!(
        r#"SELECT person_id, role::text AS "role!"
             FROM programme.session_speakers
            WHERE session_id = $1 ORDER BY sort_order, created_at"#,
        session_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des intervenants")
    .into_iter()
    .map(|l| (l.person_id, l.role))
    .collect()
}

/// Les organisations d'une séance, porteur compris.
pub async fn organisations_de_la_seance(bac: &Bac, session_id: Uuid) -> Vec<(Uuid, String)> {
    sqlx::query!(
        r#"SELECT organization_id, role::text AS "role!"
             FROM programme.session_organizations
            WHERE session_id = $1 ORDER BY role = 'lead' DESC, sort_order"#,
        session_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des organisations")
    .into_iter()
    .map(|l| (l.organization_id, l.role))
    .collect()
}

/// Les thématiques d'une séance, **avec leur triplet d'entité**.
pub async fn thematiques_de_la_seance(
    bac: &Bac,
    session_id: Uuid,
) -> Vec<(String, String, String)> {
    sqlx::query!(
        r#"SELECT et.entity_schema, et.entity_table, t.code
             FROM reference.entity_terms et
             JOIN reference.taxonomy_terms t ON t.id = et.term_id
            WHERE et.entity_id = $1
            ORDER BY et.sort_order, t.code"#,
        session_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des thématiques")
    .into_iter()
    .map(|l| (l.entity_schema, l.entity_table, l.code))
    .collect()
}

/// Les fils auxquels une séance est rattachée, avec qui les a posés.
pub async fn fils_de_la_seance(bac: &Bac, session_id: Uuid) -> Vec<(Uuid, Option<Uuid>)> {
    sqlx::query!(
        "SELECT track_id, added_by FROM programme.session_tracks
          WHERE session_id = $1 ORDER BY sort_order",
        session_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des fils")
    .into_iter()
    .map(|l| (l.track_id, l.added_by))
    .collect()
}

// -----------------------------------------------------------------------------
// Agir sur une séance, PAR LE SERVICE
//
// Poser les colonnes à la main ne prouverait rien : c'est l'écriture du service
// qu'on éprouve, avec ses refus et sa réponse.
// -----------------------------------------------------------------------------

use kernel::error::Result;
use programme::domain::ids::{EventId, SessionId};
use programme::domain::sessions::PlannerMutationResult;
use programme::service::planner;

/// Un instant **écrit en heure murale de l'édition**, converti par la base.
///
/// L'écrire en UTC dans un test obligerait à faire le décalage à la main,
/// c'est-à-dire à réimplémenter ce que le test doit vérifier.
pub async fn instant_local(bac: &Bac, event_id: Uuid, mural: &str) -> time::OffsetDateTime {
    sqlx::query_scalar!(
        r#"SELECT (($2::text)::timestamp AT TIME ZONE e.timezone) AS "instant!"
             FROM event.events e WHERE e.id = $1"#,
        event_id,
        mural
    )
    .fetch_one(bac.pool())
    .await
    .expect("conversion de l'heure murale")
}

/// Placer, déplacer, redimensionner ou retirer — les heures sont murales, et le
/// jour est celui de la COP31 sauf mention contraire.
pub async fn placer(
    bac: &Bac,
    event_id: Uuid,
    session_id: Uuid,
    salle: Option<Uuid>,
    debut: &str,
    fin: &str,
) -> Result<PlannerMutationResult> {
    let jour = "2027-11-12";
    placer_le(bac, event_id, session_id, salle, jour, debut, fin).await
}

/// La même chose, en choisissant le jour : c'est ce qui éprouve la journée de
/// rattachement.
pub async fn placer_le(
    bac: &Bac,
    event_id: Uuid,
    session_id: Uuid,
    salle: Option<Uuid>,
    jour: &str,
    debut: &str,
    fin: &str,
) -> Result<PlannerMutationResult> {
    let starts_at = instant_local(bac, event_id, &format!("{jour} {debut}")).await;
    let ends_at = instant_local(bac, event_id, &format!("{jour} {fin}")).await;

    planner::placer(
        &bac.state,
        &bac.ctx(),
        EventId(event_id),
        SessionId(session_id),
        planner::ScheduleSessionPayload {
            session_id: Some(session_id),
            room_id: salle,
            starts_at,
            ends_at,
            event_day_id: None,
            time_range: None,
            enforce_room_exclusivity: None,
        },
    )
    .await
}

/// Marquer une séance diffusée, avec ou sans canal choisi.
pub async fn diffuser(
    bac: &Bac,
    event_id: Uuid,
    session_id: Uuid,
    diffusee: bool,
    canal: Option<Uuid>,
) -> Result<PlannerMutationResult> {
    planner::diffuser(
        &bac.state,
        &bac.ctx(),
        EventId(event_id),
        SessionId(session_id),
        planner::SessionBroadcastPayload {
            session_id: Some(session_id),
            is_streamed: diffusee,
            broadcast_channel_id: canal,
        },
    )
    .await
}

/// Rattacher une séance à des journées spéciales — la liste **remplace** la
/// précédente.
pub async fn rattacher(
    bac: &Bac,
    event_id: Uuid,
    session_id: Uuid,
    fils: Vec<Uuid>,
    acteur: Option<Uuid>,
) -> Result<PlannerMutationResult> {
    let ctx = match acteur {
        Some(personne) => bac.ctx().with_actor(personne),
        None => bac.ctx(),
    };

    planner::rattacher_les_fils(
        &bac.state,
        &ctx,
        EventId(event_id),
        SessionId(session_id),
        planner::SessionTracksPayload {
            session_id: Some(session_id),
            track_ids: fils,
        },
    )
    .await
}

// -----------------------------------------------------------------------------
// De quoi éprouver l'inscription
// -----------------------------------------------------------------------------

use programme::domain::ids::RegistrationId;
use programme::domain::registration::{AnnulationRendue, IssueDInscription};
use programme::service::registration;

/// Ouvrir une séance aux inscriptions, avec ou sans jauge.
pub async fn ouvrir_les_inscriptions(
    bac: &Bac,
    session_id: Uuid,
    jauge: Option<i32>,
    liste_dattente: bool,
) {
    sqlx::query!(
        "UPDATE programme.sessions
            SET registration_required = true,
                capacity = $2,
                waitlist_enabled = $3,
                registration_opens_at = now() - interval '1 day',
                registration_closes_at = now() + interval '30 days'
          WHERE id = $1",
        session_id,
        jauge,
        liste_dattente
    )
    .execute(bac.pool())
    .await
    .expect("ouverture des inscriptions");
}

/// S'inscrire, **par le service** : c'est lui qui valide, verrouille et traduit.
pub async fn sinscrire(
    bac: &Bac,
    session_id: Uuid,
    personne: Option<Uuid>,
    charge: registration::RegisterPayload,
) -> Result<IssueDInscription> {
    let ctx = match personne {
        Some(id) => bac.ctx().with_actor(id),
        None => bac.ctx(),
    };

    registration::sinscrire(
        &bac.state,
        &ctx,
        SessionId(session_id),
        personne,
        None,
        charge,
    )
    .await
}

/// Une charge utile **valide et minimale** pour le formulaire par défaut : le
/// pays est la seule réponse obligatoire qu'il porte.
pub fn reponses_valides() -> registration::RegisterPayload {
    registration::RegisterPayload {
        answers: serde_json::json!({ "country": "SN" }),
        locale: None,
        guest: None,
        sensitive_data_consent: false,
        organization_id: None,
    }
}

/// Annuler, **par le service** : c'est lui qui promeut, sous le même verrou.
pub async fn annuler(
    bac: &Bac,
    inscription: Uuid,
    session_id: Uuid,
    motif: Option<&str>,
) -> Result<AnnulationRendue> {
    registration::annuler(
        &bac.state,
        &bac.ctx(),
        RegistrationId(inscription),
        SessionId(session_id),
        motif,
    )
    .await
}

/// L'identifiant d'une inscription, depuis l'issue rendue.
pub fn identifiant_de(issue: &IssueDInscription) -> Uuid {
    let ligne = match issue {
        IssueDInscription::Registered { registration }
        | IssueDInscription::Waitlisted { registration, .. }
        | IssueDInscription::AlreadyRegistered { registration } => registration,
        autre => panic!("aucune inscription dans cette issue : {autre:?}"),
    };

    ligne
        .get("id")
        .and_then(|v| v.as_str())
        .and_then(|v| Uuid::parse_str(v).ok())
        .expect("l'inscription porte son identifiant")
}

/// L'état d'une inscription, **relu en base**.
pub async fn statut_dinscription(bac: &Bac, inscription: Uuid) -> String {
    sqlx::query_scalar!(
        r#"SELECT status::text AS "statut!" FROM programme.registrations WHERE id = $1"#,
        inscription
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de l'inscription")
}
