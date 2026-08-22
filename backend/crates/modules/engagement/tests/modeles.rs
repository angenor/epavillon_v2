//! **Une faute d'orthographe se corrige sans redéploiement, dans les deux
//! langues.**
//!
//! Ce que ces tests éprouvent et qu'aucune relecture ne remplace :
//!
//! - **un `href="{{lien}}"` survit à l'assainissement**, relu **après écriture
//!   en base** : sans cette relecture, la décision de R26 resterait une
//!   intention, et le lien mort ne se verrait qu'à la réception du courriel ;
//! - **une variable non promise fait refuser la publication, en la nommant** —
//!   à l'envoi, il serait trop tard pour corriger sans que personne n'ait rien
//!   reçu ;
//! - **publier une révision antérieure est le retour arrière**, mesuré sur le
//!   texte du courriel réellement expédié.

mod commun;

use commun::Bac;
use engagement::domain::template::TemplateVersionPayload;
use engagement::service::rules::{self, ReminderRulePayload};
use engagement::service::templates::{self, PreviewPayload};
use kernel::ErrorCode;
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

const TYPE_RAPPEL: &str = "programme.session.reminder";

/// Un modèle vide, sans révision — c'est l'état d'un modèle qu'on vient de
/// créer au back-office. Le semis n'en fournit aucun (écart n° 131).
async fn modele(bac: &Bac, cle: &str, type_code: Option<&str>) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO engagement.message_templates (key, label, type_code)
           VALUES ($1::text::platform.slug,
                   jsonb_build_object('fr', $1::text, 'en', $1::text)::platform.i18n_text,
                   $2)
        RETURNING id"#,
        cle,
        type_code
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du modèle")
}

fn revision(sujet_fr: &str, corps_fr: &str) -> TemplateVersionPayload {
    TemplateVersionPayload {
        subject: json!({ "fr": sujet_fr, "en": sujet_fr }),
        body_html: json!({ "fr": corps_fr, "en": corps_fr }),
        body_text: None,
    }
}

/// Le corps enregistré, relu **en base** — jamais celui que le service a rendu.
async fn corps_enregistre(bac: &Bac, template_id: Uuid, version: i16) -> String {
    sqlx::query_scalar!(
        r#"SELECT body_html ->> 'fr' AS "corps!"
             FROM engagement.template_versions
            WHERE template_id = $1 AND version = $2"#,
        template_id,
        version
    )
    .fetch_one(bac.pool())
    .await
    .expect("relecture du corps")
}

// -----------------------------------------------------------------------------
// T159 — l'assainissement, relu en base
// -----------------------------------------------------------------------------

/// 🔴 **Un `href="{{lien_participation}}"` survit à l'assainissement**, et un
/// `<script>` disparaît **sans emporter le texte** d'à côté.
///
/// Pour un analyseur d'URL, `{{lien_participation}}` est une adresse
/// **relative** : la politique par défaut d'un assainisseur la réécrirait ou la
/// refuserait, et dans les deux cas la variable serait détruite. Le lien du
/// courriel serait alors mort — un défaut qui ne se voit **qu'à la réception**,
/// c'est-à-dire après l'envoi à tous les destinataires.
///
/// La relecture se fait **en base**, après écriture : c'est elle qui fait la
/// différence entre une décision et une intention (R26).
#[tokio::test]
async fn un_lien_porteur_dune_variable_survit_a_lecriture() {
    let bac = Bac::monter().await;
    let redactrice = commun::redactrice(&bac).await;
    let template = modele(&bac, "rappel-seance", Some(TYPE_RAPPEL)).await;

    let ecrite = templates::ecrire_revision(
        &bac.state,
        &bac.ctx(),
        redactrice,
        template,
        &revision(
            "{{titre_session}}",
            r#"<p>Bonjour {{prenom}}.</p>
               <a href="{{lien_participation}}">Rejoindre</a>
               <script>voler()</script>
               <marquee>Texte à garder</marquee>
               <p onclick="voler()">Encore du texte</p>"#,
        ),
    )
    .await
    .expect("écriture de la révision");

    assert_eq!(ecrite.version, 1);
    assert!(
        ecrite.published_at.is_none(),
        "une révision écrite n'est PAS servie : publier est un second geste"
    );

    let corps = corps_enregistre(&bac, template, 1).await;
    assert!(
        corps.contains("{{lien_participation}}"),
        "la variable du lien a été détruite : {corps}"
    );
    assert!(corps.contains("{{prenom}}"));
    assert!(!corps.contains("script"), "le script reste : {corps}");
    assert!(!corps.contains("onclick"), "l'attribut reste : {corps}");
    assert!(
        corps.contains("Texte à garder"),
        "une balise refusée ne doit pas emporter le texte qu'elle contient : {corps}"
    );
    assert!(corps.contains("Encore du texte"));

    // Les variables citées sont **relevées**, jamais déclarées : une liste
    // saisie à la main divergerait du gabarit au premier ajustement.
    let mut citees = ecrite.variables.clone();
    citees.sort();
    assert_eq!(
        citees,
        vec![
            "lien_participation".to_owned(),
            "prenom".to_owned(),
            "titre_session".to_owned()
        ]
    );
}

// -----------------------------------------------------------------------------
// T161 — la variable non promise
// -----------------------------------------------------------------------------

/// 🔴 **Une variable que le type ne promet pas fait refuser la publication, en
/// la nommant.**
///
/// Le refus arrive à la publication et non à l'envoi : à l'envoi, il serait trop
/// tard pour corriger sans que personne n'ait rien reçu — le courriel partirait
/// avec un trou, et le trou ne se verrait qu'à la réception.
#[tokio::test]
async fn une_variable_non_promise_fait_refuser_la_publication() {
    let bac = Bac::monter().await;
    let redactrice = commun::redactrice(&bac).await;
    let template = modele(&bac, "rappel-seance", Some(TYPE_RAPPEL)).await;

    templates::ecrire_revision(
        &bac.state,
        &bac.ctx(),
        redactrice,
        template,
        &revision("Rappel", "<p>Bonjour {{prenom_du_president}}.</p>"),
    )
    .await
    .expect("écriture de la révision");

    let refus = templates::publier(&bac.state, &bac.ctx(), redactrice, template, 1)
        .await
        .expect_err("la publication doit être refusée");

    assert_eq!(refus.code, ErrorCode::EngagementTemplateVariableUnknown);
    let detail = refus.detail.unwrap_or_default();
    assert!(
        detail.contains("prenom_du_president"),
        "le refus doit NOMMER la variable : {detail}"
    );
    assert!(
        detail.contains("lien_participation"),
        "et dire ce que le type promet : {detail}"
    );

    // Et rien n'est servi : le modèle n'a toujours aucune révision publiée.
    let detail = templates::detail(&bac.state, redactrice, template)
        .await
        .expect("détail du modèle");
    assert!(detail.current.is_none());
}

/// **Une variable citée dans la seule version anglaise est refusée comme en
/// français.** Les relever langue par langue laisserait passer celle qu'on ne
/// relit pas.
#[tokio::test]
async fn une_variable_non_promise_en_anglais_est_refusee_aussi() {
    let bac = Bac::monter().await;
    let redactrice = commun::redactrice(&bac).await;
    let template = modele(&bac, "rappel-seance", Some(TYPE_RAPPEL)).await;

    templates::ecrire_revision(
        &bac.state,
        &bac.ctx(),
        redactrice,
        template,
        &TemplateVersionPayload {
            subject: json!({ "fr": "Rappel", "en": "Reminder" }),
            body_html: json!({
                "fr": "<p>Bonjour {{prenom}}.</p>",
                "en": "<p>Hello {{nickname}}.</p>"
            }),
            body_text: None,
        },
    )
    .await
    .expect("écriture de la révision");

    let refus = templates::publier(&bac.state, &bac.ctx(), redactrice, template, 1)
        .await
        .expect_err("la publication doit être refusée");
    assert!(refus.detail.unwrap_or_default().contains("nickname"));
}

// -----------------------------------------------------------------------------
// T160 et T163 — publier, revenir en arrière, et la trace
// -----------------------------------------------------------------------------

/// 🔴 **Une révision non publiée n'est pas servie ; publiée, elle l'est ; la
/// précédente reste republiable.**
///
/// Mesuré sur **le texte du courriel réellement expédié**, et non sur le
/// pointeur du modèle : c'est la seule façon de prouver qu'un retour arrière
/// change ce que les gens reçoivent.
///
/// La trace d'expédition porte le modèle **et le numéro de révision** servis
/// (FR-089) — sans le numéro, on saurait quel modèle a servi sans savoir quel
/// texte est parti, ce qui est exactement la question qu'on se pose après une
/// correction.
#[tokio::test]
async fn publier_sert_le_nouveau_texte_et_republier_rend_lancien() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let redactrice = commun::redactrice(&bac).await;
    let template = modele(&bac, "rappel-seance", Some(TYPE_RAPPEL)).await;

    for (sujet, corps) in [
        (
            "Première version — {{titre_session}}",
            "<p>Ancien texte.</p>",
        ),
        (
            "Seconde version — {{titre_session}}",
            "<p>Nouveau texte.</p>",
        ),
    ] {
        templates::ecrire_revision(
            &bac.state,
            &bac.ctx(),
            redactrice,
            template,
            &revision(sujet, corps),
        )
        .await
        .expect("écriture de la révision");
    }

    // Trois décalages : trois vagues, donc trois occasions de mesurer.
    rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &ReminderRulePayload {
            event_id: Some(terrain.edition),
            session_id: None,
            offsets: Some(vec![2880, 1440, 60]),
            channels: None,
            type_code: None,
            template_id: None,
            is_active: None,
        },
    )
    .await
    .expect("écriture de la règle");
    commun::relayer(&bac, "programme.registration.").await;

    // **Vague 1 : aucune révision publiée** — le texte de secours part, et la
    // trace porte un modèle nul (R27).
    commun::avancer_un_decalage(&bac, terrain.seance, 2880).await;
    commun::passer_le_worker(&bac).await;
    assert!(
        bac.boite.messages()[0]
            .subject
            .starts_with("Rappel de session"),
        "sans révision publiée, le texte de secours part : {}",
        bac.boite.messages()[0].subject
    );
    assert_eq!(traces_de_revision(&bac, template).await, Vec::<i16>::new());

    // **Vague 2 : la seconde révision est publiée.**
    bac.boite.vider();
    templates::publier(&bac.state, &bac.ctx(), redactrice, template, 2)
        .await
        .expect("publication de la seconde révision");
    commun::avancer_un_decalage(&bac, terrain.seance, 1440).await;
    commun::passer_le_worker(&bac).await;

    let messages = bac.boite.messages();
    assert_eq!(messages.len(), 3);
    assert_eq!(
        messages[0].subject,
        "Seconde version — Financer l'adaptation"
    );
    assert_eq!(traces_de_revision(&bac, template).await, vec![2, 2, 2]);

    // **Vague 3 : retour arrière.** Republier la première est le geste, et rien
    // n'a été effacé pour le permettre.
    bac.boite.vider();
    let detail = templates::publier(&bac.state, &bac.ctx(), redactrice, template, 1)
        .await
        .expect("retour à la première révision");
    assert_eq!(detail.current.expect("une révision servie").version, 1);
    assert_eq!(detail.versions.len(), 2, "rien n'est effacé");

    commun::avancer_un_decalage(&bac, terrain.seance, 60).await;
    commun::passer_le_worker(&bac).await;
    assert_eq!(
        bac.boite.messages()[0].subject,
        "Première version — Financer l'adaptation"
    );
}

/// Les numéros de révision portés par les traces d'expédition de ce modèle.
async fn traces_de_revision(bac: &Bac, template_id: Uuid) -> Vec<i16> {
    sqlx::query_scalar!(
        r#"SELECT template_version AS "version!"
             FROM engagement.email_messages
            WHERE template_id = $1
            ORDER BY created_at"#,
        template_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des traces")
}

// -----------------------------------------------------------------------------
// T162 — la variable manquante à l'exécution
// -----------------------------------------------------------------------------

/// 🔴 **Une variable manquante à l'exécution fait échouer l'envoi : rien ne
/// part, et le motif la nomme.**
///
/// Le modèle l'écrit lui-même : *« mieux vaut un job en échec visible qu'un
/// email "Bonjour  ," envoyé à 2 000 personnes »*. Le cas est atteignable
/// malgré le contrôle de publication — un modèle qui ne sert **aucun type** ne
/// promet rien, donc rien ne peut lui être reproché, et une règle de rappel peut
/// le désigner.
#[tokio::test]
async fn une_variable_manquante_a_lexecution_fait_echouer_lenvoi() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let redactrice = commun::redactrice(&bac).await;

    // Sans type : aucune promesse, donc aucun contrôle possible à la
    // publication.
    let template = modele(&bac, "modele-libre", None).await;
    templates::ecrire_revision(
        &bac.state,
        &bac.ctx(),
        redactrice,
        template,
        &revision("Rappel", "<p>Bonjour {{variable_absente}}.</p>"),
    )
    .await
    .expect("écriture de la révision");
    templates::publier(&bac.state, &bac.ctx(), redactrice, template, 1)
        .await
        .expect("publication : rien n'est promis, rien n'est refusé");

    rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &ReminderRulePayload {
            event_id: Some(terrain.edition),
            session_id: None,
            offsets: Some(vec![1440]),
            channels: None,
            type_code: None,
            template_id: Some(template),
            is_active: None,
        },
    )
    .await
    .expect("écriture de la règle");
    commun::relayer(&bac, "programme.registration.").await;
    commun::avancer_les_rappels(&bac, terrain.seance).await;

    let issues = commun::passer_le_worker(&bac).await;
    assert!(
        issues.iter().all(Result::is_err),
        "l'envoi doit échouer, pas partir amputé"
    );

    assert_eq!(bac.boite.compte(), 0, "rien ne part");
    assert_eq!(
        commun::compter_rappels(&bac, terrain.seance, "sent").await,
        0,
        "et la ligne n'est pas marquée partie : le travail se reprendra"
    );

    let motif = sqlx::query_scalar!(
        r#"SELECT last_error AS "motif!" FROM platform.jobs
            WHERE task = 'engagement.send_reminder' AND last_error IS NOT NULL
            LIMIT 1"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("le motif de l'échec");
    assert!(
        motif.contains("variable_absente"),
        "le motif doit NOMMER la variable : {motif}"
    );
}

// -----------------------------------------------------------------------------
// T164 — l'aperçu n'envoie rien
// -----------------------------------------------------------------------------

/// 🔴 **L'aperçu n'écrit aucune trace d'expédition et n'appelle pas
/// l'expéditeur** — et il rend les **deux langues**.
#[tokio::test]
async fn lapercu_rend_les_deux_langues_et_nenvoie_rien() {
    let bac = Bac::monter().await;
    let redactrice = commun::redactrice(&bac).await;
    let template = modele(&bac, "rappel-seance", Some(TYPE_RAPPEL)).await;

    templates::ecrire_revision(
        &bac.state,
        &bac.ctx(),
        redactrice,
        template,
        &TemplateVersionPayload {
            subject: json!({ "fr": "Rappel — {{titre_session}}", "en": "Reminder — {{titre_session}}" }),
            body_html: json!({
                "fr": "<p>Bonjour {{prenom}}, c'est {{delai}}.</p>",
                // L'anglais est absent du corps : le repli sur le français est
                // celui de `platform.t()`, et il vaut pour l'aperçu aussi.
                "en": ""
            }),
            body_text: None,
        },
    )
    .await
    .expect("écriture de la révision");

    let apercu = templates::apercu(
        &bac.state,
        redactrice,
        template,
        &PreviewPayload {
            version: None,
            variables: HashMap::from([("prenom".to_owned(), "Awa".to_owned())]),
        },
    )
    .await
    .expect("l'aperçu d'un brouillon, avant publication");

    assert!(apercu.fr.subject.contains("« titre_session »"));
    assert!(
        apercu.fr.body_html.contains("Awa"),
        "la valeur fournie est employée : {}",
        apercu.fr.body_html
    );
    assert!(
        apercu.fr.body_html.contains("« delai »"),
        "et ce qui manque prend un exemple VISIBLE : {}",
        apercu.fr.body_html
    );
    assert!(
        apercu.en.body_html.contains("Bonjour"),
        "une langue absente se replie sur le français : {}",
        apercu.en.body_html
    );

    assert_eq!(bac.boite.compte(), 0, "l'aperçu n'appelle pas l'expéditeur");
    let traces = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM engagement.email_messages"#)
        .fetch_one(bac.pool())
        .await
        .expect("comptage des traces");
    assert_eq!(traces, 0, "et il n'écrit aucune trace d'expédition");
}

// -----------------------------------------------------------------------------
// La garde
// -----------------------------------------------------------------------------

/// **Les cinq gestes sont gardés par `engagement.template.manage`, sur la
/// portée GLOBALE.**
///
/// Un modèle de message sert toutes les éditions à la fois : le borner à l'une
/// d'elles n'aurait pas de sens, et l'ouvrir à qui administre **une** COP
/// laisserait réécrire les courriels de toutes les autres. L'administratrice de
/// la COP31 ne le détient donc pas, et l'animatrice d'une organisation encore
/// moins.
#[tokio::test]
async fn les_modeles_sont_gardes_par_leur_permission_globale() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let redactrice = commun::redactrice(&bac).await;
    let template = modele(&bac, "rappel-seance", Some(TYPE_RAPPEL)).await;

    templates::lister(&bac.state, redactrice)
        .await
        .expect("la portée globale gère les modèles");

    for (qui, quoi) in [
        (terrain.administratrice, "l'administratrice d'UNE édition"),
        (terrain.animatrice, "l'animatrice d'une organisation"),
    ] {
        let refus = templates::lister(&bac.state, qui)
            .await
            .expect_err("{quoi} ne gère pas les modèles de la plateforme");
        assert_eq!(refus.code, ErrorCode::Forbidden, "{quoi}");

        let refus = templates::ecrire_revision(
            &bac.state,
            &bac.ctx(),
            qui,
            template,
            &revision("Rappel", "<p>Texte.</p>"),
        )
        .await
        .expect_err("l'écriture est gardée elle aussi");
        assert_eq!(refus.code, ErrorCode::Forbidden, "{quoi}");
    }
}
