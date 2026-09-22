//! Les deux courriels de décision — **composés ici, et jamais ailleurs**.
//!
//! Le texte appartient au module qui déclenche l'envoi. Le faire écrire par
//! `identity` demanderait à ce crate une arête que le principe II interdit, et
//! au module de l'identité de connaître une règle d'admission qui ne le regarde
//! pas.
//!
//! Ces textes **ne sont pas des traductions d'interface** : ils ne s'affichent
//! dans aucun écran, et aucun administrateur ne les modifie — jusqu'à B6, où
//! ils deviendront une donnée de `engagement.message_templates`.
//!
//! # LE LIEN RAMÈNE DANS L'APPLICATION
//!
//! `/guide-nego/…`, et **sans préfixe de langue** : les adresses de Guide Négo
//! ne sont pas localisées (`defineI18nRoute(false)`), contrairement aux écrans
//! du site. Envoyer vers `/en/…` donnerait un lien qui n'existe nulle part.

use kernel::mail::OutgoingMail;

/// Les deux langues servies. Toute autre valeur de `preferred_locale` retombe
/// sur le français, comme `platform.t()`.
fn en_anglais(locale: &str) -> bool {
    locale.starts_with("en")
}

pub struct MailContext<'a> {
    pub message_id: &'a str,
    pub to: &'a str,
    pub locale: &'a str,
    pub first_name: &'a str,
    /// Le nom de l'espace ouvert, quand la demande en visait un. Absent : la
    /// demande portait sur Guide Négo en entier.
    pub space_name: Option<&'a str>,
    pub app_public_url: &'a str,
}

/// L'écran d'arrivée : « Mon accès », dans les ressources de l'application.
fn lien(ctx: &MailContext<'_>) -> String {
    format!(
        "{}/guide-nego/ressources/acces",
        ctx.app_public_url.trim_end_matches('/')
    )
}

/// La demande est admise. Le message **nomme ce qui s'ouvre** : « la COP31 »
/// n'est pas « Guide Négo en entier », et la personne doit savoir lequel des
/// deux elle vient d'obtenir.
pub fn demande_admise(ctx: &MailContext<'_>) -> OutgoingMail {
    let acces = lien(ctx);

    let (subject, text) = if en_anglais(ctx.locale) {
        let ouvre = match ctx.space_name {
            Some(nom) => format!("the reserved modules for {nom}"),
            None => "the reserved modules of Guide Négo".to_owned(),
        };
        (
            "Your access to Guide Négo is open".to_owned(),
            format!(
                "Hello {prenom},\n\n\
                 Your access request has been approved: {ouvre} are now open to you.\n\n\
                 Open Guide Négo to see them:\n\n\
                 {acces}\n\n\
                 The ePavillon team — IFDD",
                prenom = ctx.first_name,
            ),
        )
    } else {
        let ouvre = match ctx.space_name {
            Some(nom) => format!("les modules réservés de {nom} vous sont"),
            None => "les modules réservés de Guide Négo vous sont".to_owned(),
        };
        (
            "Votre accès à Guide Négo est ouvert".to_owned(),
            format!(
                "Bonjour {prenom},\n\n\
                 Votre demande d'accès a été acceptée : {ouvre} ouverts.\n\n\
                 Ouvrez Guide Négo pour les retrouver :\n\n\
                 {acces}\n\n\
                 L'équipe ePavillon — IFDD",
                prenom = ctx.first_name,
            ),
        )
    };

    compose(ctx, &subject, text)
}

/// La demande est refusée. **Le motif est repris tel quel** quand il y en a
/// un : c'est la seule chose que la personne lira pour comprendre, et la
/// paraphraser la priverait de ce que l'administrateur a voulu dire.
///
/// Sans motif, le message ne fait pas semblant d'en avoir un : il dit ce qui
/// reste possible — demander le code en cours à son réseau.
pub fn demande_refusee(ctx: &MailContext<'_>, motif: Option<&str>) -> OutgoingMail {
    let (subject, text) = if en_anglais(ctx.locale) {
        let raison = match motif {
            Some(m) => format!("\n\nReason given: {m}"),
            None => String::new(),
        };
        (
            "Your Guide Négo access request".to_owned(),
            format!(
                "Hello {prenom},\n\n\
                 Your access request to the reserved modules has not been approved.{raison}\n\n\
                 If you belong to a francophone negotiators' network, ask your group for the \
                 current invitation code: it opens the modules straight away.\n\n\
                 The ePavillon team — IFDD",
                prenom = ctx.first_name,
            ),
        )
    } else {
        let raison = match motif {
            Some(m) => format!("\n\nMotif indiqué : {m}"),
            None => String::new(),
        };
        (
            "Votre demande d'accès à Guide Négo".to_owned(),
            format!(
                "Bonjour {prenom},\n\n\
                 Votre demande d'accès aux modules réservés n'a pas été retenue.{raison}\n\n\
                 Si vous faites partie d'un réseau francophone de négociatrices et de \
                 négociateurs, demandez à votre groupe le code d'invitation en cours : il ouvre \
                 les modules aussitôt.\n\n\
                 L'équipe ePavillon — IFDD",
                prenom = ctx.first_name,
            ),
        )
    };

    compose(ctx, &subject, text)
}

fn compose(ctx: &MailContext<'_>, subject: &str, text: String) -> OutgoingMail {
    OutgoingMail {
        message_id: ctx.message_id.to_owned(),
        to: ctx.to.to_owned(),
        locale: ctx.locale.to_owned(),
        subject: subject.to_owned(),
        text,
        html: None,
    }
}
