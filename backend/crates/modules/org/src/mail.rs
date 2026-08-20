//! Composition des trois courriels du module.
//!
//! **Le texte appartient au module qui déclenche l'envoi**, pas au relais : le
//! site reçoit un sujet et un corps déjà écrits, dans la langue de
//! `people.preferred_locale`, et ne fait que les transporter. En B6 la
//! composition passera aux modèles administrables de
//! `engagement.message_templates` ; si le site composait, il faudrait alors
//! défaire son travail.
//!
//! Ces textes **ne sont pas des traductions d'interface** : ils ne s'affichent
//! dans aucun écran, et aucun administrateur ne les modifie — jusqu'à B6, où ils
//! deviendront précisément une donnée.

use kernel::mail::OutgoingMail;

/// Les deux langues servies. Toute autre valeur de `preferred_locale` retombe
/// sur le français, comme `platform.t()`.
fn en_anglais(locale: &str) -> bool {
    locale.starts_with("en")
}

/// Les écrans du site. **Les chemins sont traduits** — le site sert
/// `/mon-organisation` en français et `/en/my-organization` en anglais : écrire
/// le nom du fichier de page donnerait un lien qui n'existe dans aucune des deux
/// langues, et c'est la faute que B1 a relevée.
#[derive(Debug, Clone, Copy)]
enum Ecran {
    Invitation,
    EspaceOrganisation,
}

impl Ecran {
    fn chemin(self, anglais: bool) -> &'static str {
        match (self, anglais) {
            (Self::Invitation, false) => "/invitation",
            (Self::Invitation, true) => "/en/invitation",
            (Self::EspaceOrganisation, false) => "/mon-organisation",
            (Self::EspaceOrganisation, true) => "/en/my-organization",
        }
    }
}

pub struct MailContext<'a> {
    pub message_id: &'a str,
    pub to: &'a str,
    pub locale: &'a str,
    pub first_name: &'a str,
    pub organization_name: &'a str,
    pub app_public_url: &'a str,
}

fn url(ctx: &MailContext<'_>, ecran: Ecran) -> String {
    format!(
        "{}{}",
        ctx.app_public_url.trim_end_matches('/'),
        ecran.chemin(en_anglais(ctx.locale))
    )
}

/// L'invitation à rejoindre une organisation. **Le seul des trois qui porte un
/// jeton**, et le seul dont la charge utile est un secret.
pub fn invitation(ctx: &MailContext<'_>, jeton: &str) -> OutgoingMail {
    let lien = format!("{}?token={jeton}", url(ctx, Ecran::Invitation));

    let (subject, text) = if en_anglais(ctx.locale) {
        (
            format!(
                "You are invited to join {} — ePavillon",
                ctx.organization_name
            ),
            format!(
                "Hello {prenom},\n\n\
                 You have been invited to join {organisation} on ePavillon. \
                 Follow this link to accept:\n\n\
                 {lien}\n\n\
                 This link is valid for seven days and can only be used once. \
                 If you were not expecting this invitation, ignore this message.\n\n\
                 The ePavillon team — IFDD",
                prenom = ctx.first_name,
                organisation = ctx.organization_name,
            ),
        )
    } else {
        (
            format!(
                "Invitation à rejoindre {} — ePavillon",
                ctx.organization_name
            ),
            format!(
                "Bonjour {prenom},\n\n\
                 Vous êtes invité·e à rejoindre {organisation} sur ePavillon. \
                 Suivez ce lien pour accepter :\n\n\
                 {lien}\n\n\
                 Ce lien est valable sept jours et ne sert qu'une fois. \
                 Si vous n'attendiez pas cette invitation, ignorez ce message.\n\n\
                 L'équipe ePavillon — IFDD",
                prenom = ctx.first_name,
                organisation = ctx.organization_name,
            ),
        )
    };

    compose(ctx, &subject, text)
}

/// Le message qui prévient les référents qu'une demande les attend. **Aucun
/// lien de décision** : il mène à l'espace de l'organisation, où la personne
/// s'authentifie.
pub fn demande_recue(ctx: &MailContext<'_>, demandeur: &str) -> OutgoingMail {
    let espace = url(ctx, Ecran::EspaceOrganisation);

    let (subject, text) = if en_anglais(ctx.locale) {
        (
            format!("A membership request for {}", ctx.organization_name),
            format!(
                "Hello {prenom},\n\n\
                 {demandeur} has asked to join {organisation} on ePavillon. \
                 Accept or decline from your organization space:\n\n\
                 {espace}\n\n\
                 The ePavillon team — IFDD",
                prenom = ctx.first_name,
                organisation = ctx.organization_name,
            ),
        )
    } else {
        (
            format!("Une demande d'adhésion pour {}", ctx.organization_name),
            format!(
                "Bonjour {prenom},\n\n\
                 {demandeur} demande à rejoindre {organisation} sur ePavillon. \
                 Acceptez ou refusez depuis votre espace organisation :\n\n\
                 {espace}\n\n\
                 L'équipe ePavillon — IFDD",
                prenom = ctx.first_name,
                organisation = ctx.organization_name,
            ),
        )
    };

    compose(ctx, &subject, text)
}

/// Le message qui annonce qu'une adhésion est active.
pub fn adhesion_approuvee(ctx: &MailContext<'_>) -> OutgoingMail {
    let espace = url(ctx, Ecran::EspaceOrganisation);

    let (subject, text) = if en_anglais(ctx.locale) {
        (
            format!("You are now a member of {}", ctx.organization_name),
            format!(
                "Hello {prenom},\n\n\
                 Your membership of {organisation} is now active on ePavillon. \
                 Your organization space:\n\n\
                 {espace}\n\n\
                 The ePavillon team — IFDD",
                prenom = ctx.first_name,
                organisation = ctx.organization_name,
            ),
        )
    } else {
        (
            format!("Vous êtes membre de {}", ctx.organization_name),
            format!(
                "Bonjour {prenom},\n\n\
                 Votre adhésion à {organisation} est active sur ePavillon. \
                 Votre espace organisation :\n\n\
                 {espace}\n\n\
                 L'équipe ePavillon — IFDD",
                prenom = ctx.first_name,
                organisation = ctx.organization_name,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn contexte() -> MailContext<'static> {
        MailContext {
            message_id: "01J",
            to: "b.ouedraogo@osed-sahel.org",
            locale: "fr",
            first_name: "Boureima",
            organization_name: "Observatoire du Sahel",
            app_public_url: "http://localhost:3000/",
        }
    }

    #[test]
    fn le_lien_dinvitation_mene_au_site_et_porte_le_jeton() {
        let mail = invitation(&contexte(), "abc123");
        assert!(mail
            .text
            .contains("http://localhost:3000/invitation?token=abc123"));
        assert!(
            !mail.text.contains("//invitation"),
            "la barre finale est absorbée"
        );
        assert!(mail.subject.contains("Observatoire du Sahel"));
    }

    #[test]
    fn le_chemin_suit_la_langue_du_destinataire() {
        let mut ctx = contexte();
        ctx.locale = "en-GB";
        assert!(invitation(&ctx, "x")
            .text
            .contains("/en/invitation?token=x"));
        assert!(adhesion_approuvee(&ctx)
            .text
            .contains("/en/my-organization"));

        ctx.locale = "pt";
        assert!(invitation(&ctx, "x").text.contains("/invitation?token=x"));
    }

    /// Les deux messages sans jeton ne doivent en porter aucun : un lien de
    /// décision reçu par courriel ferait entrer qui tient le message.
    #[test]
    fn seuls_les_liens_dinvitation_portent_un_jeton() {
        let ctx = contexte();
        assert!(!demande_recue(&ctx, "Awa Sow Fall").text.contains("token="));
        assert!(!adhesion_approuvee(&ctx).text.contains("token="));
    }
}
