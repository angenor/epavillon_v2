//! Les envois du back-office : brouillons, destinataires figés, état d'envoi.
//!
//! La liste des destinataires vient de `engagement.mailing_audience()`, qui lit
//! les participants de l'édition (programme, identity, org) : la règle de
//! frontière de `cross.rs` vaut ici, on lit, on n'écrit que dans `engagement`.
//! Les pièces jointes se LISENT dans `media` (rattachements et objets) ; seul
//! le module media les pose et les retire.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::mailing::{
    MailingAttachment, MailingAudienceEntry, MailingCounts, MailingCriterion, MailingRecipient,
    MailingSummary,
};

/// Un envoi tel que la table le porte.
#[derive(Debug, Clone)]
pub struct LigneEnvoi {
    pub id: Uuid,
    pub event_id: Uuid,
    pub status: String,
    pub subject: String,
    pub body_html: String,
    pub audience: Vec<MailingCriterion>,
    pub extra_addresses: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub reply_to: Option<String>,
    pub duplicated_from: Option<Uuid>,
    pub created_by_name: Option<String>,
    pub queued_by_name: Option<String>,
    pub queued_at: Option<OffsetDateTime>,
    pub completed_at: Option<OffsetDateTime>,
    pub cancelled_at: Option<OffsetDateTime>,
    pub echec: Echec,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub habillage: Habillage,
    /// Le dossier du courriel (01/11) ; nul : un envoi groupé.
    pub proposal_id: Option<Uuid>,
    /// Le texte type choisi (01/11).
    pub mail_text_id: Option<Uuid>,
}

/// Le dernier échec d'un envoi et ses relances (31/10).
#[derive(Debug, Clone, Default)]
pub struct Echec {
    pub failed_at: Option<OffsetDateTime>,
    pub failure_reason: Option<String>,
    pub relaunch_count: i16,
    pub relaunched_at: Option<OffsetDateTime>,
}

/// Le gabarit et ce qui l'accompagne (28/10).
#[derive(Debug, Clone)]
pub struct Habillage {
    pub template: String,
    /// Objet JSON nom de variable -> valeur de secours.
    pub variable_fallbacks: serde_json::Value,
    pub reply_button: bool,
    pub cta_label: Option<String>,
    pub cta_url: Option<String>,
    /// Gabarit personnalisé (29/10) ; `template` porte alors sa mise en page.
    pub mail_template_id: Option<Uuid>,
}

/// Ce qu'un brouillon enregistre, déjà validé et assaini.
pub struct ValeursBrouillon<'a> {
    pub subject: &'a str,
    pub body_html: &'a str,
    pub audience: serde_json::Value,
    pub extra_addresses: &'a [String],
    pub cc: &'a [String],
    pub bcc: &'a [String],
    pub reply_to: Option<&'a str>,
    pub habillage: &'a Habillage,
    pub mail_text_id: Option<Uuid>,
}

fn criteres(valeur: serde_json::Value) -> Vec<MailingCriterion> {
    serde_json::from_value(valeur).unwrap_or_default()
}

pub async fn lire(pool: &PgPool, id: Uuid) -> Result<Option<LigneEnvoi>> {
    let ligne = sqlx::query!(
        r#"SELECT m.id, m.event_id, m.status::text AS "status!", m.subject, m.body_html,
                  m.audience, m.extra_addresses::text[] AS "extra_addresses!",
                  m.cc::text[] AS "cc!", m.bcc::text[] AS "bcc!", m.reply_to::text AS reply_to,
                  m.duplicated_from,
                  (SELECT p.display_name FROM identity.people p WHERE p.id = m.created_by) AS created_by_name,
                  (SELECT p.display_name FROM identity.people p WHERE p.id = m.queued_by) AS queued_by_name,
                  m.queued_at, m.completed_at, m.cancelled_at, m.created_at, m.updated_at,
                  m.failed_at, m.failure_reason, m.relaunch_count, m.relaunched_at,
                  m.template, m.variable_fallbacks, m.reply_button, m.cta_label,
                  m.cta_url::text AS cta_url, m.mail_template_id, m.proposal_id, m.mail_text_id
             FROM engagement.mailings m
            WHERE m.id = $1"#,
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| LigneEnvoi {
        id: l.id,
        event_id: l.event_id,
        status: l.status,
        subject: l.subject,
        body_html: l.body_html,
        audience: criteres(l.audience),
        extra_addresses: l.extra_addresses,
        cc: l.cc,
        bcc: l.bcc,
        reply_to: l.reply_to,
        duplicated_from: l.duplicated_from,
        created_by_name: l.created_by_name,
        queued_by_name: l.queued_by_name,
        queued_at: l.queued_at,
        completed_at: l.completed_at,
        cancelled_at: l.cancelled_at,
        echec: Echec {
            failed_at: l.failed_at,
            failure_reason: l.failure_reason,
            relaunch_count: l.relaunch_count,
            relaunched_at: l.relaunched_at,
        },
        created_at: l.created_at,
        updated_at: l.updated_at,
        habillage: Habillage {
            template: l.template,
            variable_fallbacks: l.variable_fallbacks,
            reply_button: l.reply_button,
            cta_label: l.cta_label,
            cta_url: l.cta_url,
            mail_template_id: l.mail_template_id,
        },
        proposal_id: l.proposal_id,
        mail_text_id: l.mail_text_id,
    }))
}

/// Les envois d'une édition, ou ceux d'un de ses dossiers.
pub async fn lister(
    pool: &PgPool,
    event_id: Uuid,
    proposal_id: Option<Uuid>,
) -> Result<Vec<MailingSummary>> {
    let lignes = sqlx::query!(
        r#"SELECT m.id, m.event_id, m.status::text AS "status!", m.subject,
                  (SELECT p.display_name FROM identity.people p WHERE p.id = m.created_by) AS created_by_name,
                  (SELECT p.display_name FROM identity.people p WHERE p.id = m.queued_by) AS queued_by_name,
                  m.queued_at, m.completed_at, m.created_at, m.updated_at,
                  m.failed_at, m.failure_reason,
                  COALESCE(c.total, 0) AS "total!", COALESCE(c.queued, 0) AS "queued!",
                  COALESCE(c.sending, 0) AS "sending!", COALESCE(c.sent, 0) AS "sent!",
                  COALESCE(c.failed, 0) AS "failed!", COALESCE(c.suppressed, 0) AS "suppressed!",
                  COALESCE(c.cancelled, 0) AS "cancelled!",
                  (SELECT count(*) FROM media.attachments x
                    WHERE x.owner_schema = 'engagement' AND x.owner_table = 'mailings'
                      AND x.owner_id = m.id AND x.role = 'attachment') AS "attachment_count!",
                  m.proposal_id,
                  (SELECT t.name FROM engagement.mail_texts t WHERE t.id = m.mail_text_id) AS mail_text_name,
                  COALESCE(m.extra_addresses[1]::text,
                           (SELECT r.email::text FROM engagement.mailing_recipients r
                             WHERE r.mailing_id = m.id ORDER BY r.id LIMIT 1)) AS "to",
                  cardinality(m.cc)::int8 AS "cc_count!"
             FROM engagement.mailings m
             LEFT JOIN LATERAL (
                 SELECT count(*) AS total,
                        count(*) FILTER (WHERE r.status = 'queued') AS queued,
                        count(*) FILTER (WHERE r.status = 'sending') AS sending,
                        count(*) FILTER (WHERE r.status = 'sent') AS sent,
                        count(*) FILTER (WHERE r.status = 'failed') AS failed,
                        count(*) FILTER (WHERE r.status = 'suppressed') AS suppressed,
                        count(*) FILTER (WHERE r.status = 'cancelled') AS cancelled
                   FROM engagement.mailing_recipients r
                  WHERE r.mailing_id = m.id
             ) c ON true
            WHERE m.event_id = $1 AND ($2::uuid IS NULL OR m.proposal_id = $2)
            ORDER BY COALESCE(m.queued_at, m.updated_at) DESC"#,
        event_id,
        proposal_id
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| MailingSummary {
            id: l.id,
            event_id: l.event_id,
            status: l.status,
            shown_subject: l.subject.clone(),
            subject: l.subject,
            created_by_name: l.created_by_name,
            queued_by_name: l.queued_by_name,
            queued_at: l.queued_at,
            completed_at: l.completed_at,
            failed_at: l.failed_at,
            failure_reason: l.failure_reason,
            created_at: l.created_at,
            updated_at: l.updated_at,
            counts: MailingCounts {
                total: l.total,
                queued: l.queued,
                sending: l.sending,
                sent: l.sent,
                failed: l.failed,
                suppressed: l.suppressed,
                cancelled: l.cancelled,
            },
            attachment_count: l.attachment_count,
            proposal_id: l.proposal_id,
            mail_text_name: l.mail_text_name,
            to: l.to,
            cc_count: l.cc_count,
        })
        .collect())
}

pub async fn destinataires(pool: &PgPool, mailing_id: Uuid) -> Result<Vec<MailingRecipient>> {
    let lignes = sqlx::query!(
        r#"SELECT r.id, r.email::text AS "email!", r.person_id,
                  NULLIF(r.variables ->> 'prenom', '') AS first_name,
                  NULLIF(r.variables ->> 'nom', '') AS last_name,
                  NULLIF(r.variables ->> 'organisation', '') AS organization,
                  r.sources, r.status::text AS "status!", r.last_error, r.sent_at
             FROM engagement.mailing_recipients r
            WHERE r.mailing_id = $1
            ORDER BY r.email"#,
        mailing_id
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| MailingRecipient {
            id: l.id,
            email: l.email,
            person_id: l.person_id,
            first_name: l.first_name,
            last_name: l.last_name,
            organization: l.organization,
            sources: l.sources,
            status: l.status,
            last_error: l.last_error,
            sent_at: l.sent_at,
        })
        .collect())
}

pub fn compter(destinataires: &[MailingRecipient]) -> MailingCounts {
    let mut c = MailingCounts {
        total: destinataires.len() as i64,
        ..MailingCounts::default()
    };
    for d in destinataires {
        match d.status.as_str() {
            "queued" => c.queued += 1,
            "sending" => c.sending += 1,
            "sent" => c.sent += 1,
            "failed" => c.failed += 1,
            "suppressed" => c.suppressed += 1,
            _ => c.cancelled += 1,
        }
    }
    c
}

pub async fn creer(
    conn: &mut PgConnection,
    event_id: Uuid,
    acteur: Uuid,
    v: &ValeursBrouillon<'_>,
    duplique_de: Option<Uuid>,
    proposal_id: Option<Uuid>,
) -> Result<Uuid> {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO engagement.mailings
               (event_id, subject, body_html, audience, extra_addresses, cc, bcc, reply_to,
                duplicated_from, created_by, template, variable_fallbacks, reply_button,
                cta_label, cta_url, mail_template_id, mail_text_id, proposal_id)
           VALUES ($1, $2, $3, $4, $5::text[]::platform.email[], $6::text[]::platform.email[],
                   $7::text[]::platform.email[], $8::text::platform.email, $9, $10,
                   $11, $12, $13, $14, $15::text::platform.url, $16, $17, $18)
           RETURNING id"#,
        event_id,
        v.subject,
        v.body_html,
        v.audience,
        v.extra_addresses,
        v.cc,
        v.bcc,
        v.reply_to,
        duplique_de,
        acteur,
        v.habillage.template,
        v.habillage.variable_fallbacks,
        v.habillage.reply_button,
        v.habillage.cta_label,
        v.habillage.cta_url,
        v.habillage.mail_template_id,
        v.mail_text_id,
        proposal_id
    )
    .fetch_one(conn)
    .await?;
    Ok(id)
}

/// Rend faux si l'envoi n'est plus un brouillon.
pub async fn modifier(
    conn: &mut PgConnection,
    id: Uuid,
    event_id: Uuid,
    v: &ValeursBrouillon<'_>,
) -> Result<bool> {
    let n = sqlx::query!(
        r#"UPDATE engagement.mailings
              SET subject = $2, body_html = $3, audience = $4,
                  extra_addresses = $5::text[]::platform.email[],
                  cc = $6::text[]::platform.email[], bcc = $7::text[]::platform.email[],
                  reply_to = $8::text::platform.email, event_id = $9,
                  template = $10, variable_fallbacks = $11, reply_button = $12,
                  cta_label = $13, cta_url = $14::text::platform.url, mail_template_id = $15,
                  mail_text_id = $16
            WHERE id = $1 AND status = 'draft'"#,
        id,
        v.subject,
        v.body_html,
        v.audience,
        v.extra_addresses,
        v.cc,
        v.bcc,
        v.reply_to,
        event_id,
        v.habillage.template,
        v.habillage.variable_fallbacks,
        v.habillage.reply_button,
        v.habillage.cta_label,
        v.habillage.cta_url,
        v.habillage.mail_template_id,
        v.mail_text_id
    )
    .execute(conn)
    .await?
    .rows_affected();
    Ok(n == 1)
}

/// Supprime un envoi qui n'est pas en cours : destinataires par cascade,
/// rattachements de pièces jointes par le déclencheur du module media. Rend
/// faux si l'envoi est en cours (ou a disparu entre-temps).
pub async fn supprimer(conn: &mut PgConnection, id: Uuid) -> Result<bool> {
    let n = sqlx::query!(
        "DELETE FROM engagement.mailings WHERE id = $1 AND status <> 'queued'",
        id
    )
    .execute(conn)
    .await?
    .rows_affected();
    Ok(n == 1)
}

pub async fn audience(
    pool: &PgPool,
    event_id: Uuid,
    criteres: &serde_json::Value,
    adresses: &[String],
) -> Result<Vec<MailingAudienceEntry>> {
    let lignes = sqlx::query!(
        r#"SELECT a.email AS "email!", a.person_id, a.first_name, a.last_name, a.organization,
                  a.sources AS "sources!", a.is_suppressed AS "is_suppressed!"
             FROM engagement.mailing_audience($1, $2, $3::text[]::platform.email[]) a"#,
        event_id,
        criteres,
        adresses
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| MailingAudienceEntry {
            email: l.email,
            person_id: l.person_id,
            first_name: l.first_name,
            last_name: l.last_name,
            organization: l.organization,
            sources: l.sources,
            is_suppressed: l.is_suppressed,
        })
        .collect())
}

/// Fige les destinataires et passe l'envoi en file, dans la transaction de
/// l'appelant. Rend le nombre d'adresses qui partiront, ou `None` si l'envoi
/// n'était plus un brouillon (deux clics, deux onglets). `dossier` : les
/// valeurs des variables du dossier (objet vide hors dossier), figées sur
/// chaque destinataire avec les siennes.
pub async fn figer(
    conn: &mut PgConnection,
    id: Uuid,
    acteur: Uuid,
    dossier: &serde_json::Value,
) -> Result<Option<i64>> {
    let passe = sqlx::query!(
        "UPDATE engagement.mailings
            SET status = 'queued', queued_at = now(), queued_by = $2
          WHERE id = $1 AND status = 'draft'",
        id,
        acteur
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();
    if passe != 1 {
        return Ok(None);
    }

    sqlx::query!(
        r#"INSERT INTO engagement.mailing_recipients
               (mailing_id, email, person_id, variables, sources, status, last_error)
           SELECT m.id, a.email::platform.email, a.person_id,
                  jsonb_build_object(
                      'prenom', COALESCE(a.first_name, ''),
                      'nom', COALESCE(a.last_name, ''),
                      'organisation', COALESCE(a.organization, ''),
                      'email', a.email,
                      'edition', platform.t(e.title, 'fr')) || $2::jsonb,
                  a.sources,
                  CASE WHEN a.is_suppressed THEN 'suppressed'::engagement.mailing_recipient_status
                       ELSE 'queued'::engagement.mailing_recipient_status END,
                  CASE WHEN a.is_suppressed THEN 'adresse sur la liste de suppression' END
             FROM engagement.mailings m
             JOIN event.events e ON e.id = m.event_id
             CROSS JOIN LATERAL engagement.mailing_audience(m.event_id, m.audience, m.extra_addresses) a
            WHERE m.id = $1"#,
        id,
        dossier
    )
    .execute(&mut *conn)
    .await?;

    let partiront = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM engagement.mailing_recipients
            WHERE mailing_id = $1 AND status = 'queued'"#,
        id
    )
    .fetch_one(&mut *conn)
    .await?;
    Ok(Some(partiront))
}

/// Arrête un envoi en cours ou en échec. Rend faux s'il n'était ni l'un ni
/// l'autre.
pub async fn annuler(conn: &mut PgConnection, id: Uuid) -> Result<bool> {
    let n = sqlx::query!(
        "UPDATE engagement.mailings SET status = 'cancelled', cancelled_at = now()
          WHERE id = $1 AND status IN ('queued', 'failed')",
        id
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();
    if n != 1 {
        return Ok(false);
    }
    sqlx::query!(
        "UPDATE engagement.mailing_recipients SET status = 'cancelled'
          WHERE mailing_id = $1 AND status = 'queued'",
        id
    )
    .execute(&mut *conn)
    .await?;
    Ok(true)
}

/// Remet en file un envoi en échec, pour ses seuls destinataires encore en
/// file : ceux qui sont partis, en échec ou écartés ne sont pas repris. Rend
/// le numéro de la relance, ou `None` si l'envoi n'était pas en échec (deux
/// clics, deux onglets) ou que personne n'attend plus.
pub async fn relancer(conn: &mut PgConnection, id: Uuid) -> Result<Option<i16>> {
    let numero = sqlx::query_scalar!(
        "UPDATE engagement.mailings m
            SET status = 'queued', relaunch_count = m.relaunch_count + 1, relaunched_at = now()
          WHERE m.id = $1 AND m.status = 'failed'
            AND EXISTS (SELECT 1 FROM engagement.mailing_recipients r
                         WHERE r.mailing_id = m.id AND r.status = 'queued')
        RETURNING m.relaunch_count",
        id
    )
    .fetch_optional(conn)
    .await?;
    Ok(numero)
}

/// Le filet de `engagement.fail_stalled_mailings()` : les envois de l'édition
/// qu'aucun travail ne sert plus passent en échec. Rend ceux qui ont changé.
pub async fn basculer_les_bloques(conn: &mut PgConnection, event_id: Uuid) -> Result<Vec<Uuid>> {
    let ids = sqlx::query_scalar!(
        r#"SELECT id AS "id!" FROM engagement.fail_stalled_mailings($1) AS id"#,
        event_id
    )
    .fetch_all(conn)
    .await?;
    Ok(ids)
}

/// Y a-t-il, dans l'édition, un envoi en cours ? Évite d'ouvrir une écriture
/// à chaque relevé quand il n'y a rien à vérifier.
pub async fn a_des_envois_en_cours(pool: &PgPool, event_id: Uuid) -> Result<bool> {
    let oui = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM engagement.mailings
                           WHERE event_id = $1 AND status = 'queued') AS "oui!""#,
        event_id
    )
    .fetch_one(pool)
    .await?;
    Ok(oui)
}

/// La copie de l'équipe proposée par défaut. Un réglage absent ou illisible
/// n'en propose aucune.
pub async fn equipe_en_copie(pool: &PgPool) -> Result<Vec<String>> {
    let equipe = sqlx::query_scalar!(
        r#"SELECT ARRAY(SELECT jsonb_array_elements_text(value)) AS equipe
             FROM platform.settings
            WHERE key = 'engagement.mailing.team_cc' AND jsonb_typeof(value) = 'array'"#
    )
    .fetch_optional(pool)
    .await?;
    Ok(equipe.flatten().unwrap_or_default())
}

pub async fn adresse_de(
    pool: &PgPool,
    person_id: Uuid,
) -> Result<Option<(String, String, String, Option<String>)>> {
    let ligne = sqlx::query!(
        r#"SELECT p.primary_email::text AS "email!", p.first_name, p.last_name,
                  (SELECT COALESCE(o.acronym, o.legal_name) FROM org.organizations o
                    WHERE o.id = p.primary_organization_id) AS organisation
             FROM identity.people p WHERE p.id = $1"#,
        person_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(ligne.map(|l| (l.email, l.first_name, l.last_name, l.organisation)))
}

pub async fn titre_edition(pool: &PgPool, event_id: Uuid) -> Result<String> {
    let titre = sqlx::query_scalar!(
        r#"SELECT platform.t(title, 'fr') AS "titre!" FROM event.events WHERE id = $1"#,
        event_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(titre.unwrap_or_default())
}

// -----------------------------------------------------------------------------
// Le travail d'envoi
// -----------------------------------------------------------------------------

/// Un destinataire pris par le travail d'envoi.
pub struct Pris {
    pub id: Uuid,
    pub email: String,
    pub variables: serde_json::Value,
}

/// Prend le prochain destinataire en file et le marque `sending` **avant**
/// l'expédition : un travail rejoué après une mort brutale ne le reprendra pas,
/// et un courriel perdu se voit dans l'historique, quand un doublon parti ne se
/// rattrape pas.
pub async fn prendre(conn: &mut PgConnection, mailing_id: Uuid) -> Result<Option<Pris>> {
    let ligne = sqlx::query!(
        r#"UPDATE engagement.mailing_recipients r
              SET status = 'sending'
            WHERE r.id = (SELECT q.id FROM engagement.mailing_recipients q
                           WHERE q.mailing_id = $1 AND q.status = 'queued'
                           ORDER BY q.id LIMIT 1 FOR UPDATE SKIP LOCKED)
        RETURNING r.id, r.email::text AS "email!", r.variables"#,
        mailing_id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne.map(|l| Pris {
        id: l.id,
        email: l.email,
        variables: l.variables,
    }))
}

/// `statut` : `sent`, `failed`, `suppressed`, ou `queued` pour rendre la ligne
/// à la file quand le serveur d'envoi est injoignable.
pub async fn clore(
    conn: &mut PgConnection,
    recipient_id: Uuid,
    statut: &str,
    erreur: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        r#"UPDATE engagement.mailing_recipients
              SET status = $2::text::engagement.mailing_recipient_status,
                  last_error = $3,
                  sent_at = CASE WHEN $2 = 'sent' THEN now() END
            WHERE id = $1"#,
        recipient_id,
        statut,
        erreur
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Passe l'envoi en échec avec son motif (`engagement.fail_mailing()`). Rend
/// faux s'il n'était plus en cours.
pub async fn echouer(conn: &mut PgConnection, mailing_id: Uuid, motif: &str) -> Result<bool> {
    let passe = sqlx::query_scalar!(
        r#"SELECT engagement.fail_mailing($1, $2) AS "passe!""#,
        mailing_id,
        motif
    )
    .fetch_one(conn)
    .await?;
    Ok(passe)
}

/// Clôt l'envoi s'il ne reste personne en file. Rend vrai s'il est clos.
///
/// Appelé entre deux lots, quand aucun destinataire n'est en cours d'envoi :
/// une ligne encore `sending` vient d'un travail mort entre le marquage et la
/// réponse du serveur. Rien ne dit si le message est parti ; la ligne le dit.
pub async fn clore_si_fini(conn: &mut PgConnection, mailing_id: Uuid) -> Result<bool> {
    sqlx::query!(
        "UPDATE engagement.mailing_recipients
            SET status = 'failed', last_error = 'envoi interrompu : départ non confirmé'
          WHERE mailing_id = $1 AND status = 'sending'
            AND NOT EXISTS (SELECT 1 FROM engagement.mailing_recipients q
                             WHERE q.mailing_id = $1 AND q.status = 'queued')",
        mailing_id
    )
    .execute(&mut *conn)
    .await?;

    let n = sqlx::query!(
        "UPDATE engagement.mailings m SET status = 'sent', completed_at = now()
          WHERE m.id = $1 AND m.status = 'queued'
            AND NOT EXISTS (SELECT 1 FROM engagement.mailing_recipients r
                             WHERE r.mailing_id = m.id AND r.status IN ('queued', 'sending'))",
        mailing_id
    )
    .execute(conn)
    .await?
    .rows_affected();
    Ok(n == 1)
}

// -----------------------------------------------------------------------------
// Pièces jointes, lues dans `media`
// -----------------------------------------------------------------------------

/// Une pièce jointe et l'endroit où ses octets sont rangés.
pub struct PieceJointe {
    pub fiche: MailingAttachment,
    pub bucket: String,
    pub object_key: String,
}

pub async fn pieces_jointes(pool: &PgPool, mailing_id: Uuid) -> Result<Vec<PieceJointe>> {
    let lignes = sqlx::query!(
        r#"SELECT x.id AS attachment_id, o.id AS asset_id,
                  COALESCE(o.original_filename, 'piece-jointe') AS "filename!",
                  o.mime_type, o.byte_size, o.status::text AS "status!",
                  o.bucket, o.object_key
             FROM media.attachments x
             JOIN media.assets o ON o.id = x.asset_id
            WHERE x.owner_schema = 'engagement' AND x.owner_table = 'mailings'
              AND x.owner_id = $1 AND x.role = 'attachment' AND o.deleted_at IS NULL
            ORDER BY x.sort_order, x.created_at"#,
        mailing_id
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| PieceJointe {
            fiche: MailingAttachment {
                attachment_id: l.attachment_id,
                asset_id: l.asset_id,
                filename: l.filename,
                mime_type: l.mime_type,
                byte_size: l.byte_size,
                status: l.status,
            },
            bucket: l.bucket,
            object_key: l.object_key,
        })
        .collect())
}

/// Le poids cumulé permis pour les pièces jointes d'un envoi.
pub async fn poids_permis(pool: &PgPool) -> Result<Option<i64>> {
    let poids = sqlx::query_scalar!(
        "SELECT r.max_total_byte_size FROM media.attachable_roles r
          WHERE r.owner_schema = 'engagement' AND r.owner_table = 'mailings'
            AND r.role = 'attachment'"
    )
    .fetch_optional(pool)
    .await?;
    Ok(poids.flatten())
}
