//! Composition des courriels du module.
//!
//! **Le texte appartient au module qui déclenche l'envoi**, pas au relais : le
//! site reçoit un sujet et un corps déjà écrits, dans la langue de
//! `people.preferred_locale`, et ne fait que les transporter (research.md
//! § R13). En B6 la composition passera aux modèles administrables de
//! `engagement.message_templates` ; si le site composait, il faudrait alors
//! défaire son travail.
//!
//! Ces textes **ne sont pas des traductions d'interface** et n'ont rien à faire
//! dans les fichiers i18n du site : ils ne s'affichent dans aucun écran, et
//! aucun administrateur ne les modifie — jusqu'à B6, où ils deviendront
//! précisément une donnée.

use kernel::mail::OutgoingMail;

/// Les deux langues servies. Toute autre valeur de `preferred_locale` retombe
/// sur le français, comme `platform.t()`.
fn en_anglais(locale: &str) -> bool {
    locale.starts_with("en")
}

/// Les écrans du site, dans les deux langues.
///
/// **Les chemins sont traduits** : le site sert `/verification-adresse` en
/// français et `/en/verify-email` en anglais — `prefix_except_default`, français
/// par défaut. Écrire ici le nom du fichier de page (`/auth/verify-email`)
/// donnait un lien qui n'existe dans aucune des deux langues, et un courriel
/// dont le seul contenu utile menait à une page introuvable.
#[derive(Debug, Clone, Copy)]
enum Ecran {
    VerificationAdresse,
    NouveauMotDePasse,
    Connexion,
    MotDePasseOublie,
}

impl Ecran {
    fn chemin(self, anglais: bool) -> &'static str {
        match (self, anglais) {
            (Self::VerificationAdresse, false) => "/verification-adresse",
            (Self::VerificationAdresse, true) => "/en/verify-email",
            (Self::NouveauMotDePasse, false) => "/nouveau-mot-de-passe",
            (Self::NouveauMotDePasse, true) => "/en/reset-password",
            (Self::Connexion, false) => "/connexion",
            (Self::Connexion, true) => "/en/login",
            (Self::MotDePasseOublie, false) => "/mot-de-passe-oublie",
            (Self::MotDePasseOublie, true) => "/en/forgot-password",
        }
    }
}

/// Le lien mène à un **écran du site**, jamais à une route de l'API :
/// `APP_PUBLIC_URL` est l'adresse du front.
fn url(ctx: &MailContext<'_>, ecran: Ecran) -> String {
    format!(
        "{}{}",
        ctx.app_public_url.trim_end_matches('/'),
        ecran.chemin(en_anglais(ctx.locale))
    )
}

fn lien(ctx: &MailContext<'_>, ecran: Ecran, jeton: &str) -> String {
    format!("{}?token={jeton}", url(ctx, ecran))
}

pub struct MailContext<'a> {
    pub message_id: &'a str,
    pub to: &'a str,
    pub locale: &'a str,
    pub first_name: &'a str,
    pub app_public_url: &'a str,
}

pub fn verification_email(ctx: &MailContext<'_>, jeton: &str) -> OutgoingMail {
    let url = lien(ctx, Ecran::VerificationAdresse, jeton);

    let (subject, text) = if en_anglais(ctx.locale) {
        (
            "Confirm your email address — ePavillon",
            format!(
                "Hello {prenom},\n\n\
                 Your ePavillon account has been created. Confirm your email address to sign in:\n\n\
                 {url}\n\n\
                 This link is valid for 24 hours. If you did not create this account, ignore this message.\n\n\
                 The ePavillon team — IFDD",
                prenom = ctx.first_name,
            ),
        )
    } else {
        (
            "Confirmez votre adresse — ePavillon",
            format!(
                "Bonjour {prenom},\n\n\
                 Votre compte ePavillon a été créé. Confirmez votre adresse pour pouvoir vous connecter :\n\n\
                 {url}\n\n\
                 Ce lien est valable 24 heures. Si vous n'êtes pas à l'origine de cette demande, ignorez ce message.\n\n\
                 L'équipe ePavillon — IFDD",
                prenom = ctx.first_name,
            ),
        )
    };

    compose(ctx, subject, text)
}

/// Le rappel envoyé quand l'adresse était **déjà connue**.
///
/// C'est la moitié visible de la réponse invariable (FR-035) : l'écran affiche
/// la même chose dans les deux cas, et seul le courriel diffère. Il ne contient
/// **aucun lien de vérification** — rien ne doit permettre à un tiers de
/// prendre la main sur un compte existant en le « réinscrivant ».
pub fn existing_account_notice(ctx: &MailContext<'_>) -> OutgoingMail {
    let connexion = url(ctx, Ecran::Connexion);
    let oubli = url(ctx, Ecran::MotDePasseOublie);

    let (subject, text) = if en_anglais(ctx.locale) {
        (
            "You already have an ePavillon account",
            format!(
                "Hello {prenom},\n\n\
                 Someone tried to create an ePavillon account with this address. \
                 One already exists, so nothing has been changed.\n\n\
                 Sign in: {connexion}\n\
                 Forgot your password? {oubli}\n\n\
                 If this was not you, no action is required.\n\n\
                 The ePavillon team — IFDD",
                prenom = ctx.first_name,
            ),
        )
    } else {
        (
            "Vous avez déjà un compte ePavillon",
            format!(
                "Bonjour {prenom},\n\n\
                 Une inscription vient d'être tentée avec cette adresse. \
                 Un compte existe déjà : rien n'a été modifié.\n\n\
                 Se connecter : {connexion}\n\
                 Mot de passe oublié ? {oubli}\n\n\
                 Si vous n'êtes pas à l'origine de cette demande, il n'y a rien à faire.\n\n\
                 L'équipe ePavillon — IFDD",
                prenom = ctx.first_name,
            ),
        )
    };

    compose(ctx, subject, text)
}

/// Le lien de réinitialisation.
///
/// **Une heure de validité**, et le message le dit : c'est la durée que la
/// configuration donne à cette finalité, et la seule qu'une personne pressée
/// ait besoin de connaître. Le texte rappelle aussi que ne rien faire suffit —
/// un lien demandé par un tiers s'ignore, il ne se « refuse » pas.
pub fn password_reset_email(ctx: &MailContext<'_>, jeton: &str) -> OutgoingMail {
    let url = lien(ctx, Ecran::NouveauMotDePasse, jeton);

    let (subject, text) = if en_anglais(ctx.locale) {
        (
            "Reset your password — ePavillon",
            format!(
                "Hello {prenom},\n\n\
                 A new password was requested for your ePavillon account. Choose one here:\n\n\
                 {url}\n\n\
                 This link is valid for one hour and can only be used once. \
                 If you did not request it, ignore this message: your password remains unchanged.\n\n\
                 The ePavillon team — IFDD",
                prenom = ctx.first_name,
            ),
        )
    } else {
        (
            "Réinitialisez votre mot de passe — ePavillon",
            format!(
                "Bonjour {prenom},\n\n\
                 Un nouveau mot de passe a été demandé pour votre compte ePavillon. \
                 Choisissez-le ici :\n\n\
                 {url}\n\n\
                 Ce lien est valable une heure et ne sert qu'une fois. \
                 Si vous n'êtes pas à l'origine de cette demande, ignorez ce message : \
                 votre mot de passe reste inchangé.\n\n\
                 L'équipe ePavillon — IFDD",
                prenom = ctx.first_name,
            ),
        )
    };

    compose(ctx, subject, text)
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
            to: "awa.diallo@example.org",
            locale: "fr",
            first_name: "Awa",
            app_public_url: "http://localhost:3000/",
        }
    }

    #[test]
    fn le_lien_mene_au_site_et_porte_le_jeton() {
        let mail = verification_email(&contexte(), "abc123");
        assert!(mail
            .text
            .contains("http://localhost:3000/verification-adresse?token=abc123"));
        assert!(
            !mail.text.contains("//verification"),
            "la barre finale est absorbée"
        );
    }

    /// Le chemin du fichier de page n'est pas celui que le site sert : un lien
    /// écrit `/auth/verify-email` mène à une page introuvable dans les deux
    /// langues.
    #[test]
    fn le_chemin_suit_la_langue_du_destinataire() {
        let mut ctx = contexte();
        assert!(verification_email(&ctx, "x")
            .text
            .contains("/verification-adresse?token=x"));

        ctx.locale = "en";
        assert!(verification_email(&ctx, "x")
            .text
            .contains("/en/verify-email?token=x"));
        assert!(password_reset_email(&ctx, "x")
            .text
            .contains("/en/reset-password?token=x"));
    }

    #[test]
    fn le_lien_de_reinitialisation_mene_au_formulaire_de_nouveau_mot_de_passe() {
        let mail = password_reset_email(&contexte(), "abc123");
        assert!(mail
            .text
            .contains("http://localhost:3000/nouveau-mot-de-passe?token=abc123"));
        assert!(
            mail.text.contains("une heure"),
            "la durée annoncée est celle de la configuration"
        );
    }

    #[test]
    fn le_rappel_de_compte_existant_ne_porte_aucun_lien_de_verification() {
        let mail = existing_account_notice(&contexte());
        assert!(!mail.text.contains("verify-email"));
        assert!(!mail.text.contains("token="));
    }

    #[test]
    fn langlais_est_servi_des_que_la_langue_commence_par_en() {
        let mut ctx = contexte();
        ctx.locale = "en-GB";
        assert!(verification_email(&ctx, "x").subject.starts_with("Confirm"));

        ctx.locale = "pt";
        assert!(verification_email(&ctx, "x")
            .subject
            .starts_with("Confirmez"));
    }
}
