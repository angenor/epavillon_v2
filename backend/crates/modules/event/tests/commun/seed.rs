//! Les deux éditions des tests — **hors de `docs/database/`**, ce ne sont pas
//! des données du modèle.
//!
//! ## Ce que `900_seed.sql` fournit déjà, et ce qu'il ne fournit pas
//!
//! Relevé sur le fichier, pas supposé (research.md § R19) :
//!
//! - **§ 4 — quatre séries** : `cop_climate`, `cop_biodiversity`,
//!   `cop_desertification`, `ifdd_webinars`. Un test qui les compte doit
//!   s'attendre à quatre, pas à zéro. C'est le rappel de l'écart n° 86 de B2 :
//!   *le semis donne plus que prévu, et l'ignorer coûte une demi-heure.*
//! - **§ 4 bis — un canal de diffusion général**, `ifdd_principal`, **sans
//!   édition**, actif et **par défaut**. Il est le défaut de son propre groupe
//!   — celui des canaux `event_id IS NULL` — et n'entre donc en concurrence
//!   avec aucun canal d'édition : poser un défaut sur la COP31 **ne le déloge
//!   pas**, et c'est le comportement attendu (R6).
//! - **Aucune édition.** Chaque test crée la sienne, et c'est pourquoi ce
//!   fichier existe.
//!
//! ## Les deux éditions semées ici
//!
//! La **COP31** est l'édition de référence du jalon : série climat, Belém,
//! fuseau `America/Belem`, pavillon **tenu**, du 9 au 20 novembre 2027 — douze
//! journées civiles, le compte que `jours_civils_dans_le_fuseau.rs` vérifie.
//! Trois heures derrière l'UTC : si la première journée tombe le 8 ou le 10, le
//! fuseau n'a pas été appliqué.
//!
//! La seconde est **sans pavillon**, et c'est tout son intérêt : elle n'a pas
//! de sigle et ne doit pas en réclamer. C'est le cas PACO, celui qui prouve que
//! la règle du sigle ne casse pas un usage existant.

#![allow(dead_code)]

use time::Date;
use uuid::Uuid;

use super::{pays, serie, Bac};

pub struct Editions {
    /// COP31 : pavillon tenu, sigle `COP31`, Belém, 9 au 20 novembre 2027.
    pub cop31: Uuid,
    /// Sans pavillon et **sans sigle**. En ligne, donc sans pays ni ville.
    pub sans_pavillon: Uuid,
}

pub const SLUG_COP31: &str = "cop31-belem";
pub const SLUG_SANS_PAVILLON: &str = "rendez-vous-paco-2027";
pub const FUSEAU_COP31: &str = "America/Belem";
/// Douze journées, du 9 au 20 novembre inclus.
pub const JOURS_COP31: i64 = 12;

pub async fn editions(bac: &Bac) -> Editions {
    Editions {
        cop31: cop31(bac).await,
        sans_pavillon: sans_pavillon(bac).await,
    }
}

/// L'édition de référence.
///
/// Les instants sont écrits **en heure locale de Belém**, convertis par la base
/// : `timestamp AT TIME ZONE 'America/Belem'` rend le `timestamptz`
/// correspondant. Les écrire en UTC obligerait à faire le décalage à la main
/// dans le test — c'est-à-dire à réimplémenter ce que le test doit vérifier.
pub async fn cop31(bac: &Bac) -> Uuid {
    let climat = serie(bac, "cop_climate").await;
    let bresil = pays(bac, "BRA").await;

    sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (series_id, edition_label, edition_year, title, acronym, slug, description,
                status, participation_mode, timezone, starts_at, ends_at,
                country_id, city, has_pavilion)
           VALUES ($1, 'COP31', 2027,
                   '{"fr":"COP31 — Conférence des Parties","en":"COP31"}'::jsonb,
                   'COP31', $2::text::platform.slug,
                   '{"fr":"Pavillon de la Francophonie à la COP31.","en":"Francophonie pavilion at COP31."}'::jsonb,
                   'announced', 'hybrid', $3::text::platform.timezone_name,
                   timestamp '2027-11-09 09:00' AT TIME ZONE $3,
                   timestamp '2027-11-20 18:00' AT TIME ZONE $3,
                   $4, 'Belém', true)
        RETURNING id"#,
        climat,
        SLUG_COP31,
        FUSEAU_COP31,
        bresil
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la COP31")
}

/// L'édition **sans pavillon et sans sigle**. Elle est enregistrable telle
/// quelle, et c'est exactement ce que la règle du sigle doit préserver.
pub async fn sans_pavillon(bac: &Bac) -> Uuid {
    let webinaires = serie(bac, "ifdd_webinars").await;

    sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (series_id, edition_label, edition_year, title, slug, description,
                status, participation_mode, timezone, starts_at, ends_at, has_pavilion)
           VALUES ($1, 'PACO 2027', 2027,
                   '{"fr":"Rendez-vous du PACO","en":"PACO meeting"}'::jsonb,
                   $2::text::platform.slug,
                   '{"fr":"Cycle en ligne, sans pavillon.","en":"Online series, no pavilion."}'::jsonb,
                   'announced', 'online', 'Africa/Dakar'::platform.timezone_name,
                   timestamp '2027-03-02 10:00' AT TIME ZONE 'Africa/Dakar',
                   timestamp '2027-03-04 16:00' AT TIME ZONE 'Africa/Dakar',
                   false)
        RETURNING id"#,
        webinaires,
        SLUG_SANS_PAVILLON
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'édition sans pavillon")
}

// -----------------------------------------------------------------------------
// Les enfants d'une édition
//
// Six objets, un par route paramétrée qui remonte à son édition, plus la grille
// et une séance. Ils servent deux tests :
//
// - **l'URL forgée** — chacun de ces identifiants, demandé par un compte qui
//   n'administre pas leur édition, doit recevoir le refus d'un identifiant
//   inexistant ;
// - **le détail en une réponse** — les six onglets doivent être là, et leurs
//   décomptes doivent tomber juste.
//
// Ils sont écrits en SQL direct : ce sont des DONNÉES de test, pas un parcours.
// Les services qui les écriront viennent avec leurs propres phases.
// -----------------------------------------------------------------------------

pub struct Enfants {
    pub journee: Uuid,
    pub fil: Uuid,
    pub lieu: Uuid,
    pub salle: Uuid,
    pub canal: Uuid,
    pub appel: Uuid,
}

/// La date de la journée créée, et celle de la séance : **en heure locale de
/// l'édition**. Un décalage d'un jour ici ferait échouer le rattachement
/// automatique de la séance à sa journée, que le modèle dérive dans ce fuseau.
pub const JOUR_SEANCE: Date = time::macros::date!(2027 - 11 - 10);

pub async fn enfants(bac: &Bac, event_id: Uuid) -> Enfants {
    let journee = sqlx::query_scalar!(
        "INSERT INTO event.event_days (event_id, day_date, sort_order)
         VALUES ($1, $2, 1) RETURNING id",
        event_id,
        JOUR_SEANCE
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la journée");

    let fil = sqlx::query_scalar!(
        r#"INSERT INTO event.programme_tracks (event_id, code, slug, kind, title)
           VALUES ($1, 'journee_finance', 'journee-finance'::platform.slug, 'special_day',
                   '{"fr":"Journée finance durable","en":"Sustainable finance day"}'::jsonb)
        RETURNING id"#,
        event_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du fil");

    let lieu = sqlx::query_scalar!(
        r#"INSERT INTO event.venues (event_id, name, kind)
           VALUES ($1, '{"fr":"Pavillon de la Francophonie"}'::jsonb, 'pavilion')
        RETURNING id"#,
        event_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du lieu");

    let salle = sqlx::query_scalar!(
        r#"INSERT INTO event.rooms (venue_id, name, code, capacity)
           VALUES ($1, '{"fr":"Salle Baobab"}'::jsonb, 'baobab', 80)
        RETURNING id"#,
        lieu
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la salle");

    // **Par défaut et actif** : c'est ce canal-là que le modèle affecte
    // automatiquement à une séance marquée diffusée.
    let canal = sqlx::query_scalar!(
        r#"INSERT INTO event.broadcast_channels
               (event_id, code, name, provider, is_default, is_active)
           VALUES ($1, 'cop31_direct', '{"fr":"Direct COP31"}'::jsonb, 'youtube', true, true)
        RETURNING id"#,
        event_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du canal");

    // **La fenêtre encadre l'instant courant** : sans cela, le modèle refuse le
    // dépôt d'un dossier, et le décompte de l'onglet resterait à zéro sans
    // qu'on sache pourquoi.
    let appel = sqlx::query_scalar!(
        r#"INSERT INTO event.calls_for_proposals
               (event_id, code, title, status, opens_at, closes_at)
           VALUES ($1, 'cop31', '{"fr":"Appel à propositions COP31"}'::jsonb, 'open',
                   now() - interval '1 day', now() + interval '30 days')
        RETURNING id"#,
        event_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'appel");

    // La grille **du modèle**, pas une grille recopiée : six critères.
    sqlx::query!("SELECT event.seed_default_criteria($1)", appel)
        .execute(bac.pool())
        .await
        .expect("grille par défaut");

    Enfants {
        journee,
        fil,
        lieu,
        salle,
        canal,
        appel,
    }
}

/// Une séance **placée en salle, diffusée et rattachée au fil**.
///
/// Elle est ce qui rend les décomptes du détail vérifiables : sans elle, tous
/// valent zéro et le test ne prouve rien. La journée et le canal ne sont pas
/// donnés — le modèle les dérive, et c'est ce comportement-là qu'on veut voir.
pub async fn seance(bac: &Bac, event_id: Uuid, enfants: &Enfants) -> Uuid {
    let seance = sqlx::query_scalar!(
        r#"INSERT INTO programme.sessions
               (event_id, title, slug, format, starts_at, ends_at, timezone,
                room_id, is_streamed)
           VALUES ($1, '{"fr":"Table ronde sur la finance climat"}'::jsonb,
                   'table-ronde-finance'::platform.slug, 'hybrid',
                   timestamp '2027-11-10 14:00' AT TIME ZONE $2,
                   timestamp '2027-11-10 15:30' AT TIME ZONE $2,
                   $2::text::platform.timezone_name,
                   $3, true)
        RETURNING id"#,
        event_id,
        FUSEAU_COP31,
        enfants.salle
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la séance");

    sqlx::query!(
        "INSERT INTO programme.session_tracks (session_id, track_id) VALUES ($1, $2)",
        seance,
        enfants.fil
    )
    .execute(bac.pool())
    .await
    .expect("rattachement au fil");

    seance
}

/// Un dossier **déposé**, et un brouillon qui ne doit pas être compté.
///
/// Les deux ensemble : compter les dossiers reçus sans vérifier que le
/// brouillon reste dehors ne prouverait que la moitié de la règle (FR-020).
pub async fn dossiers(bac: &Bac, event_id: Uuid, call_id: Uuid) -> Uuid {
    let organisation = sqlx::query_scalar!(
        r#"INSERT INTO org.organizations (legal_name, slug, organization_type_code, status)
           VALUES ('Association test du Sahel', 'association-test-sahel'::platform.slug,
                   'ngo', 'active')
        RETURNING id"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'organisation");

    let deposant = super::personne(bac, "depot@example.org", "Fatou", "Ndiaye").await;

    for (slug, statut) in [
        ("dossier-depose", "submitted"),
        ("dossier-brouillon", "draft"),
    ] {
        sqlx::query!(
            r#"INSERT INTO programme.proposals
                   (call_id, event_id, organization_id, submitted_by, title, slug,
                    objectives, detailed_presentation, format, status, submitted_at)
               VALUES ($1, $2, $3, $4,
                       '{"fr":"Proposition de test"}'::jsonb, $5::text::platform.slug,
                       '{"fr":"Objectifs."}'::jsonb, '{"fr":"<p>Présentation.</p>"}'::jsonb,
                       'hybrid', $6::text::programme.proposal_status,
                       CASE WHEN $6 = 'submitted' THEN now() END)"#,
            call_id,
            event_id,
            organisation,
            deposant,
            slug,
            statut
        )
        .execute(bac.pool())
        .await
        .expect("insertion du dossier");
    }

    organisation
}

/// **Une note posée sur un critère** — ce qui rend le refus de suppression
/// vérifiable.
///
/// Sans elle, le test du critère porteur de notes ne prouverait rien : c'est le
/// chiffre, et non le code, qui interdit le retrait (research.md § R9).
///
/// Elle exige toute la chaîne du modèle : un dossier déposé, une revue de ce
/// dossier par une personne, et la note elle-même. La raccourcir en insérant une
/// note orpheline serait impossible — `review_scores` référence `reviews`.
pub async fn note_sur_le_critere(
    bac: &Bac,
    event_id: Uuid,
    call_id: Uuid,
    code_du_critere: &str,
) -> Uuid {
    let organisation = sqlx::query_scalar!(
        r#"INSERT INTO org.organizations (legal_name, slug, organization_type_code, status)
           VALUES ('Association notée', ('org-notee-' || gen_random_uuid())::platform.slug,
                   'ngo', 'active')
        RETURNING id"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'organisation");

    let deposant = super::personne(bac, "depot.note@example.org", "Awa", "Traoré").await;
    let evaluateur =
        super::personne(bac, "evaluateur@ifdd.francophonie.org", "Luc", "Bernard").await;

    let dossier = sqlx::query_scalar!(
        r#"INSERT INTO programme.proposals
               (call_id, event_id, organization_id, submitted_by, title, slug,
                objectives, detailed_presentation, format, status, submitted_at)
           VALUES ($1, $2, $3, $4, '{"fr":"Dossier évalué"}'::jsonb,
                   ('dossier-evalue-' || gen_random_uuid())::platform.slug,
                   '{"fr":"Objectifs."}'::jsonb, '{"fr":"<p>Présentation.</p>"}'::jsonb,
                   'hybrid', 'under_review', now())
        RETURNING id"#,
        call_id,
        event_id,
        organisation,
        deposant
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du dossier");

    let revue = sqlx::query_scalar!(
        "INSERT INTO programme.reviews (proposal_id, reviewer_id, recommendation, submitted_at)
         VALUES ($1, $2, 'accept', now()) RETURNING id",
        dossier,
        evaluateur
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la revue");

    let critere = sqlx::query_scalar!(
        "SELECT id FROM event.review_criteria WHERE call_id = $1 AND code = $2",
        call_id,
        code_du_critere
    )
    .fetch_one(bac.pool())
    .await
    .expect("le critère existe");

    sqlx::query!(
        "INSERT INTO programme.review_scores (review_id, criterion_id, score)
         VALUES ($1, $2, 4)",
        revue,
        critere
    )
    .execute(bac.pool())
    .await
    .expect("insertion de la note");

    critere
}

/// **Les trois déclinaisons d'une édition**, rattachées comme le module Média
/// les attend — 32:9 pour le bandeau, 16:9 pour la couverture, 1:1 pour la
/// vignette.
///
/// Les dimensions ne sont pas décoratives : `media.attachable_roles` déclare la
/// forme attendue par rôle et son déclencheur refuse un fichier qui ne l'a pas.
/// Un test qui poserait trois carrés verrait deux rattachements rejetés, et
/// mettrait l'échec sur le compte de la lecture.
///
/// Le statut `ready` compte tout autant : `media.attached_image()` n'expose que
/// les objets servables, et un objet resté `uploaded` rendrait `null`. Et le
/// texte alternatif est **exigé par le modèle** pour une image publique
/// (`ck_assets_alt_text_required`) : une image sans description ne se sert pas.
/// Enfin, `ck_assets_scan_before_ready` veut qu'un objet servable ait été
/// analysé : on pose donc le verdict, comme le ferait la chaîne de traitement.
pub async fn images_de_ledition(bac: &Bac, event_id: Uuid) {
    // **Un objet a toujours un propriétaire** (`ck_assets_owner_present`) : le
    // modèle refuse un fichier que personne n'a téléversé, faute de quoi le
    // quota de stockage n'aurait personne à qui s'imputer.
    let televerseur = super::personne(bac, "media@ifdd.francophonie.org", "Sarah", "Nkosi").await;

    for (role, largeur, hauteur) in [
        ("banner", 3200, 900),
        ("cover", 1600, 900),
        ("thumbnail", 800, 800),
    ] {
        let objet = sqlx::query_scalar!(
            r#"INSERT INTO media.assets
                   (object_key, checksum_sha256, mime_type, byte_size, width, height,
                    alt_text, owner_person_id, scan_verdict, scanned_at, status)
               VALUES ('2027/11/' || gen_random_uuid() || '/image.webp',
                       encode(sha256(gen_random_uuid()::text::bytea), 'hex'),
                       'image/webp', 120000, $1, $2,
                       '{"fr":"Pavillon de la Francophonie"}'::jsonb, $3,
                       'clean', now(), 'ready')
            RETURNING id"#,
            largeur,
            hauteur,
            televerseur
        )
        .fetch_one(bac.pool())
        .await
        .expect("insertion de l'objet");

        sqlx::query!(
            "INSERT INTO media.attachments (owner_schema, owner_table, owner_id, asset_id, role)
             VALUES ('event', 'events', $1, $2, $3::text::media.attachment_role)",
            event_id,
            objet,
            role
        )
        .execute(bac.pool())
        .await
        .unwrap_or_else(|e| panic!("rattachement du rôle {role} : {e}"));
    }
}
