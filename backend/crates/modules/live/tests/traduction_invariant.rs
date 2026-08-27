//! **Un refus de la base qui ÉCHAPPE au chemin nominal ressort avec son code
//! stable, jamais en `INTERNAL` ni avec le texte brut de PostgreSQL.**
//!
//! Les trois codes ajoutés au catalogue ne répondent jamais sur le chemin
//! nominal : le service valide en amont et rend l'issue que le contrat nomme.
//! Ils existent pour l'écriture concurrente, la donnée reprise, ou le chemin
//! ajouté plus tard qui oublierait la validation — et c'est exactement ce que ce
//! test simule, en écrivant **sans** passer par le service.

mod commun;

use commun::*;
use kernel::error::ErrorCode;

#[tokio::test]
async fn une_cible_incoherente_ressort_en_scope_target_mismatch_sur_le_champ_scope() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    // Portée `session` **sans** activité : ce que la validation du service
    // refuserait en amont, et que la base refuse en aval.
    let erreur = sqlx::query!(
        r#"INSERT INTO live.incidents (scope, incident_kind_code, severity, message, created_by)
           VALUES ('session', 'technical_issue', 'warning',
                   '{"fr":"Message.","en":"Message."}'::jsonb, $1)"#,
        comptes.globale
    )
    .execute(bac.pool())
    .await
    .expect_err("la contrainte refuse");

    let traduite = kernel::pg_error::translate(&erreur);
    assert_eq!(traduite.code, ErrorCode::LiveIncidentScopeTargetMismatch);
    assert_eq!(traduite.field.as_deref(), Some("scope"));
    assert!(
        !traduite.message.contains("live.incidents"),
        "aucun nom de table ne franchit la réponse : {}",
        traduite.message
    );
}

#[tokio::test]
async fn une_fenetre_inversee_ressort_en_window_invalid_sur_display_until() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    let erreur = sqlx::query!(
        r#"INSERT INTO live.incidents
               (scope, event_id, incident_kind_code, severity, message,
                display_from, display_until, created_by)
           VALUES ('event', $1, 'technical_issue', 'warning',
                   '{"fr":"Message.","en":"Message."}'::jsonb,
                   now(), now() - interval '1 hour', $2)"#,
        decor.event_id,
        comptes.globale
    )
    .execute(bac.pool())
    .await
    .expect_err("la contrainte refuse");

    let traduite = kernel::pg_error::translate(&erreur);
    assert_eq!(traduite.code, ErrorCode::LiveIncidentWindowInvalid);
    assert_eq!(traduite.field.as_deref(), Some("display_until"));
}
