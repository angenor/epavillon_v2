//! Les neuf issues d'un code — chacune avec son message, et **aucune qui lève**.
//!
//! C'est la preuve de FR-015 : sept de ces neuf issues sont des refus prévus par
//! le parcours, et aucune ne doit sortir en erreur. Un refus en 4xx ferait lever
//! le transport du client, et l'écran perdrait ses sorties.

mod commun;

use commun::{Bac, Graine};
use negotiation::domain::redeem::RedeemIssue;
use time::{Duration, OffsetDateTime};

#[tokio::test]
async fn les_neuf_issues_sortent_avec_leur_message() {
    let bac = Bac::monter().await;
    let decor = commun::decor(&bac).await;

    // 1 — accepté. Le message nomme le réseau, dont le libellé vient de la
    // taxonomie et non d'un fichier de traduction (FR-011).
    let accepte = commun::saisir(&bac, decor.person_id, &decor.code, None).await;
    assert_eq!(accepte.issue, RedeemIssue::Accepted);
    assert!(
        accepte.message.contains("Code reconnu") && accepte.message.contains("négociatrices"),
        "{}",
        accepte.message
    );
    assert!(accepte.granted.is_some());

    // 2 — déjà admise : le même code, une seconde fois. Ni second accès, ni
    // perte de celui qu'on a.
    let encore = commun::saisir(&bac, decor.person_id, &decor.code, None).await;
    assert_eq!(encore.issue, RedeemIssue::AlreadyGranted);
    assert!(encore.granted.is_some());
    assert_eq!(commun::attributions(&bac, decor.person_id).await.len(), 1);
    assert_eq!(commun::usages(&bac, decor.code_id).await, (1, 1));

    // 3 — inconnu. Le message rappelle le nombre de caractères, parce que c'est
    // la seule chose utile à quelqu'un qui a recopié un code à la main.
    let inconnue = commun::personne(&bac, "fatou.sow@example.org").await;
    let inconnu = commun::saisir(&bac, inconnue, "ZZZZ-999", None).await;
    assert_eq!(inconnu.issue, RedeemIssue::Unknown);
    assert!(
        inconnu.message.contains("huit caractères"),
        "{}",
        inconnu.message
    );

    // 4 — révoqué. **La date est ce qui distingue ce refus d'un code inconnu.**
    let revoque_le = OffsetDateTime::now_utc() - Duration::days(3);
    let code_revoque = commun::semer(
        &bac,
        Graine {
            code: "REVO-001",
            libelle: "Code fuité",
            space_id: Some(decor.space_id),
            ..Graine::default()
        },
    )
    .await;
    commun::revoquer(&bac, code_revoque, revoque_le).await;

    let revoque = commun::saisir(&bac, inconnue, "REVO-001", None).await;
    assert_eq!(revoque.issue, RedeemIssue::Revoked);
    assert!(
        revoque.message.contains("révoqué le"),
        "{}",
        revoque.message
    );
    assert_eq!(revoque.revoked_at, Some(revoque_le));

    // 5 — épuisé : quota de un, déjà consommé par quelqu'un d'autre.
    commun::semer(
        &bac,
        Graine {
            code: "EPUI-001",
            libelle: "Un seul usage",
            space_id: Some(decor.space_id),
            max_uses: Some(1),
            ..Graine::default()
        },
    )
    .await;
    let premiere = commun::personne(&bac, "aminata.ba@example.org").await;
    assert_eq!(
        commun::saisir(&bac, premiere, "EPUI-001", None).await.issue,
        RedeemIssue::Accepted
    );
    let epuise = commun::saisir(&bac, inconnue, "EPUI-001", None).await;
    assert_eq!(epuise.issue, RedeemIssue::Exhausted);
    assert!(
        epuise.message.contains("nombre d'entrées"),
        "{}",
        epuise.message
    );

    // 6 — terminé : la validité est passée.
    let code_termine = commun::semer(
        &bac,
        Graine {
            code: "TERM-001",
            libelle: "Validité passée",
            space_id: Some(decor.space_id),
            valid_from: Some(OffsetDateTime::now_utc() - Duration::days(10)),
            valid_until: Some(OffsetDateTime::now_utc() - Duration::days(1)),
            ..Graine::default()
        },
    )
    .await;
    assert_eq!(commun::etat_du_code(&bac, code_termine).await, "expired");
    let termine = commun::saisir(&bac, inconnue, "TERM-001", None).await;
    assert_eq!(termine.issue, RedeemIssue::Expired);
    assert!(termine.message.contains("validité"), "{}", termine.message);

    // 7 — pas encore ouvert. Le message dit **quand** il le sera.
    commun::semer(
        &bac,
        Graine {
            code: "AVAN-001",
            libelle: "Pas encore ouvert",
            space_id: Some(decor.space_id),
            valid_from: Some(OffsetDateTime::now_utc() + Duration::days(4)),
            ..Graine::default()
        },
    )
    .await;
    let avant = commun::saisir(&bac, inconnue, "AVAN-001", None).await;
    assert_eq!(avant.issue, RedeemIssue::NotYetValid);
    assert!(
        avant.message.contains("pas encore ouvert"),
        "{}",
        avant.message
    );

    // 8 — demande ouverte : mode « les deux », le code juste ne suffit plus.
    commun::regler_le_mode(&bac, "code_and_approval").await;
    let candidate = commun::personne(&bac, "mariam.keita@example.org").await;
    let demande = commun::saisir(&bac, candidate, &decor.code, None).await;
    assert_eq!(demande.issue, RedeemIssue::PendingApproval);
    assert!(demande.request.is_some());
    assert!(
        !commun::a_lacces(&bac, candidate, Some(decor.space_id)).await,
        "le code juste ne doit pas ouvrir en mode « les deux »"
    );
    commun::regler_le_mode(&bac, "code").await;

    // 9 — trop d'essais. Le message annonce son attente, et ne dit rien du code.
    commun::regler_les_essais(&bac, 2, 15, 15).await;
    let pressee = commun::personne(&bac, "kadia.toure@example.org").await;
    for _ in 0..2 {
        assert_eq!(
            commun::saisir(&bac, pressee, "ZZZZ-888", None).await.issue,
            RedeemIssue::Unknown
        );
    }
    let bloquee = commun::saisir(&bac, pressee, &decor.code, None).await;
    assert_eq!(bloquee.issue, RedeemIssue::Throttled);
    assert_eq!(bloquee.retry_after_seconds, Some(900));
    assert!(
        !bloquee.message.contains("reconnu"),
        "le refus par limite ne doit rien apprendre du code essayé : {}",
        bloquee.message
    );
    assert!(
        !commun::a_lacces(&bac, pressee, Some(decor.space_id)).await,
        "un essai refusé par la limite n'ouvre rien, même avec le bon code"
    );
}

/// Un code vide n'est pas une issue du parcours : c'est un corps malformé, et il
/// sort en erreur de validation sur son champ.
#[tokio::test]
async fn un_code_vide_est_une_erreur_de_validation() {
    let bac = Bac::monter().await;
    let decor = commun::decor(&bac).await;

    let erreur = negotiation::service::redeem::redeem(
        &bac.state,
        &bac.ctx(decor.person_id),
        negotiation::service::redeem::Saisie {
            person_id: decor.person_id,
            session_id: None,
            code: "   -- ",
            device_id: None,
            locale: "fr",
        },
    )
    .await
    .expect_err("un code vide doit être refusé");

    assert_eq!(erreur.field.as_deref(), Some("code"));
}
