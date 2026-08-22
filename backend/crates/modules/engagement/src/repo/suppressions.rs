//! La liste de suppression, et les retours du fournisseur.
//!
//! # Une suppression échue se lève sans intervention
//!
//! `is_email_suppressed()` compare déjà `expires_at` à maintenant : une boîte
//! pleine cesse d'être écartée toute seule (FR-098). **Aucun travail récurrent
//! ne la lève**, et c'est le bon choix : une purge programmée serait un second
//! dispositif à tenir d'accord avec la fonction du modèle, et le premier écart
//! entre les deux serait silencieux. La lecture, elle, montre la ligne tant
//! qu'elle existe — savoir qu'une adresse a rebondi le mois dernier a de la
//! valeur.
//!
//! # Une annonce du fournisseur rejouée ne crée pas de seconde trace
//!
//! `ux_email_messages_provider` porte sur `(created_at, provider,
//! provider_message_id)`. La table est **partitionnée par mois** et sa clé
//! primaire est `(created_at, id)` : toute mise à jour doit porter les deux,
//! sans quoi elle balaie **toutes** les partitions.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::template::EmailSuppression;

pub async fn lister(pool: &PgPool, recherche: Option<&str>) -> Result<Vec<EmailSuppression>> {
    let lignes = sqlx::query!(
        r#"SELECT s.email::text AS "email!", s.reason::text AS "reason!", s.detail,
                  s.expires_at, s.suppressed_at, s.suppressed_by
             FROM engagement.email_suppressions s
            WHERE $1::text IS NULL OR s.email::text ILIKE '%' || $1 || '%'
            ORDER BY s.suppressed_at DESC
            LIMIT 200"#,
        recherche
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| EmailSuppression {
            email: l.email,
            reason: l.reason,
            detail: l.detail,
            expires_at: l.expires_at,
            suppressed_at: l.suppressed_at,
            suppressed_by: l.suppressed_by,
        })
        .collect())
}

/// Inscrit une adresse, ou **met à jour** celle qui y était.
///
/// Une adresse qui rebondit une seconde fois ne doit pas produire un conflit :
/// le motif le plus récent est celui qui explique le mieux pourquoi la personne
/// ne reçoit plus rien.
pub async fn poser(
    conn: &mut PgConnection,
    email: &str,
    reason: &str,
    detail: Option<&str>,
    expires_at: Option<time::OffsetDateTime>,
    acteur: Option<Uuid>,
) -> std::result::Result<EmailSuppression, sqlx::Error> {
    let ligne = sqlx::query!(
        r#"INSERT INTO engagement.email_suppressions
               (email, reason, detail, expires_at, suppressed_by)
           VALUES ($1::text::platform.email,
                   $2::text::engagement.suppression_reason, $3, $4, $5)
           ON CONFLICT (email) DO UPDATE
               SET reason = EXCLUDED.reason,
                   detail = EXCLUDED.detail,
                   expires_at = EXCLUDED.expires_at,
                   suppressed_at = now(),
                   suppressed_by = EXCLUDED.suppressed_by
        RETURNING email::text AS "email!", reason::text AS "reason!", detail,
                  expires_at, suppressed_at, suppressed_by"#,
        email,
        reason,
        detail,
        expires_at,
        acteur
    )
    .fetch_one(conn)
    .await?;

    Ok(EmailSuppression {
        email: ligne.email,
        reason: ligne.reason,
        detail: ligne.detail,
        expires_at: ligne.expires_at,
        suppressed_at: ligne.suppressed_at,
        suppressed_by: ligne.suppressed_by,
    })
}

pub async fn retirer(conn: &mut PgConnection, email: &str) -> Result<bool> {
    let retirees = sqlx::query!(
        "DELETE FROM engagement.email_suppressions WHERE email = $1::text::platform.email",
        email
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(retirees == 1)
}

/// Une annonce du fournisseur, telle que le site la remonte.
#[derive(Debug, Clone)]
pub struct AnnonceRecue<'a> {
    /// Valeur de `engagement.email_status` : `delivered`, `bounced`,
    /// `complained`, `failed`.
    pub statut: &'a str,
    pub bounce_kind: Option<&'a str>,
    pub detail: Option<&'a str>,
    /// L'identifiant du fournisseur, conservé pour la corrélation de ses
    /// propres journaux. Il n'est **pas** ce qui relie l'annonce à la trace.
    pub provider_message_id: Option<&'a str>,
}

/// La trace visée par une annonce, et son adresse.
pub struct TraceVisee {
    pub created_at: time::OffsetDateTime,
    pub id: Uuid,
    pub email: String,
    pub deja_a_cet_etat: bool,
}

/// **La trace se retrouve par l'identifiant que l'API a remis au site**, pas par
/// celui du fournisseur.
///
/// Le contrat d'envoi du noyau ne rend rien : `Mailer::send()` ne rapporte aucun
/// identifiant, et l'API ne peut donc pas connaître celui du fournisseur au
/// moment où elle écrit sa trace. Se reposer dessus laisserait **toute** annonce
/// sans trace à mettre à jour — un dispositif complet qui ne ferait jamais rien,
/// et personne ne s'en apercevrait. `OutgoingMail.message_id` voyage dans les
/// deux sens ; c'est lui qui relie.
///
/// **La clé primaire porte l'instant de création** : la retrouver d'abord évite
/// que la mise à jour qui suit ne balaie toutes les partitions de la table.
pub async fn trace_du_message(
    conn: &mut PgConnection,
    job_id: Uuid,
    statut: &str,
) -> Result<Option<TraceVisee>> {
    let ligne = sqlx::query!(
        r#"SELECT created_at, id, to_email::text AS "email!",
                  (status::text = $2) AS "deja!"
             FROM engagement.email_messages
            WHERE job_id = $1
            ORDER BY created_at DESC
            LIMIT 1"#,
        job_id,
        statut
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne.map(|l| TraceVisee {
        created_at: l.created_at,
        id: l.id,
        email: l.email,
        deja_a_cet_etat: l.deja,
    }))
}

/// Applique une annonce à une trace. **La mise à jour porte les deux moitiés de
/// la clé primaire**, sans quoi elle balaie toutes les partitions.
pub async fn appliquer(
    conn: &mut PgConnection,
    trace: &TraceVisee,
    annonce: &AnnonceRecue<'_>,
) -> Result<()> {
    sqlx::query!(
        "UPDATE engagement.email_messages
            SET status       = $3::text::engagement.email_status,
                bounce_kind  = COALESCE($4, bounce_kind),
                last_error   = COALESCE($5, last_error),
                provider_message_id = COALESCE($6, provider_message_id),
                delivered_at = CASE WHEN $3 = 'delivered' THEN now() ELSE delivered_at END,
                failed_at    = CASE WHEN $3 IN ('bounced', 'failed') THEN now() ELSE failed_at END
          WHERE created_at = $1 AND id = $2",
        trace.created_at,
        trace.id,
        annonce.statut,
        annonce.bounce_kind,
        annonce.detail,
        annonce.provider_message_id
    )
    .execute(conn)
    .await?;
    Ok(())
}
