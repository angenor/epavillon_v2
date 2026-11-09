//! Le lien d'inscription des intervenants en ligne (08/11) : l'inscrit devient
//! intervenant masqué sur la page, sa place Zoom est demandée au module Live,
//! et le lien ne sert plus après la fin de l'activité.

mod commun;

use commun::Bac;
use kernel::error::ErrorCode;
use programme::domain::ids::SessionId;
use programme::domain::online_speakers::OnlineSpeakerInput;
use programme::repo::session_parts;
use programme::service::online_speakers;
use uuid::Uuid;

fn saisie(consent: bool) -> OnlineSpeakerInput {
    serde_json::from_value(serde_json::json!({
        "civility": "dr", "first_name": "Koffi", "last_name": "Mensah",
        "job_title": "Chercheur", "organization": "Université d'exemple",
        "email": "k.mensah@exemple.org", "country_iso2": null, "bio": null,
        "consent": consent
    }))
    .unwrap()
}

#[tokio::test]
async fn linscrit_par_le_lien_attend_la_regie_pour_paraitre() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    commun::seances::canal_par_defaut(&bac, terrain.edition).await;
    let (seance, jeton): (Uuid, String) = sqlx::query_as(
        r#"INSERT INTO programme.sessions
               (event_id, title, slug, format, timezone, starts_at, ends_at, status,
                published_at, organization_id, is_streamed)
           VALUES ($1, '{"fr":"Finance climat"}', 'finance-climat', 'hybrid', 'America/Belem',
                   now() + interval '2 days', now() + interval '2 days 1 hour', 'scheduled',
                   now(), $2, true)
        RETURNING id, speaker_link_token"#,
    )
    .bind(terrain.edition)
    .bind(terrain.organisation)
    .fetch_one(bac.pool())
    .await
    .unwrap();

    let invitation = online_speakers::invitation(&bac.state, &jeton)
        .await
        .unwrap();
    assert_eq!(invitation.session_id, seance);

    let refus =
        online_speakers::sinscrire_par_le_lien(&bac.state, &bac.ctx(), &jeton, saisie(false))
            .await
            .expect_err("accord absent");
    assert_eq!(refus.code, ErrorCode::RegistrationConsentRequired);

    let cree = online_speakers::sinscrire_par_le_lien(&bac.state, &bac.ctx(), &jeton, saisie(true))
        .await
        .unwrap();
    let (source, affiche): (String, bool) =
        sqlx::query_as("SELECT source, is_listed FROM programme.session_speakers WHERE id = $1")
            .bind(cree.speaker_id)
            .fetch_one(bac.pool())
            .await
            .unwrap();
    assert_eq!((source.as_str(), affiche), ("lien", false));
    assert!(
        session_parts::intervenants_publics(bac.pool(), SessionId::from(seance))
            .await
            .unwrap()
            .is_empty(),
        "masqué sur la page tant que la régie ne l'affiche pas"
    );
    let places: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM platform.jobs WHERE task = 'live.speaker_places'
            AND payload ->> 'session_id' = $1::text",
    )
    .bind(seance)
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(places, 1, "place Zoom demandée au module Live");

    // La même adresse reprend sa ligne.
    let encore =
        online_speakers::sinscrire_par_le_lien(&bac.state, &bac.ctx(), &jeton, saisie(true))
            .await
            .unwrap();
    assert_eq!(encore.speaker_id, cree.speaker_id);

    sqlx::query("UPDATE programme.sessions SET starts_at = now() - interval '2 hours', ends_at = now() - interval '1 hour' WHERE id = $1")
        .bind(seance)
        .execute(bac.pool())
        .await
        .unwrap();
    let fini = online_speakers::invitation(&bac.state, &jeton)
        .await
        .expect_err("activité finie");
    assert_eq!(fini.code, ErrorCode::NotFound);
}
