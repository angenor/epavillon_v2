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

use crate::notifications::avis::{jour_en, jour_fr, Etat};
use crate::repo::courriel::{EtatReunion, EtatSession, SignalementPublie};

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

// ---------------------------------------------------------------------------
// Changement de session et réunion non annoncée (3b) — l'état final, relu au
// moment de partir ; jamais l'autrice d'un signalement.
// ---------------------------------------------------------------------------

pub struct Envoi<'a> {
    pub message_id: &'a str,
    pub to: &'a str,
    pub locale: &'a str,
    pub app_public_url: &'a str,
}

/// L'état qui ouvre le sujet. `None` : rien de ce qui prévient n'a changé dans
/// la tranche, aucun courriel.
pub fn etat_de_session(s: &EtatSession) -> Option<Etat> {
    let signale = |motif: &str| s.signalements.iter().any(|r| r.reason == motif);
    let change = |champ: &str| s.changes.iter().any(|c| c == champ);
    if (s.annulee && change("status")) || signale("cancelled") {
        Some(Etat::Annulee)
    } else if change("start") || signale("time") {
        Some(Etat::Deplacee)
    } else if change("venue") || signale("venue") {
        Some(Etat::SalleChangee)
    } else if signale("other") {
        Some(Etat::Signalee)
    } else {
        None
    }
}

/// « heure de Belém » ; sans ville connue, le nom du fuseau.
fn fuseau(ville: Option<&str>, tz: &str, en: bool) -> String {
    match (ville, en) {
        (Some(v), false) => format!(", heure de {v}"),
        (Some(v), true) => format!(", {v} time"),
        (None, _) => format!(" ({tz})"),
    }
}

fn ligne_signalement(r: &SignalementPublie, ville: Option<&str>, tz: &str, en: bool) -> String {
    let detail = match (r.reason.as_str(), en) {
        ("time", false) => r
            .heure
            .as_deref()
            .map_or("l'heure a changé".to_owned(), |h| {
                format!("nouvelle heure {h}{}", fuseau(ville, tz, false))
            }),
        ("time", true) => r
            .heure
            .as_deref()
            .map_or("the time has changed".to_owned(), |h| {
                format!("new time {h}{}", fuseau(ville, tz, true))
            }),
        ("venue", false) => r
            .salle
            .as_deref()
            .map_or("la salle a changé".to_owned(), |s| {
                format!("nouvelle salle : {s}")
            }),
        ("venue", true) => r
            .salle
            .as_deref()
            .map_or("the room has changed".to_owned(), |s| {
                format!("new room: {s}")
            }),
        ("cancelled", false) => "la session n'aura pas lieu".to_owned(),
        ("cancelled", true) => "the session will not take place".to_owned(),
        (_, false) => "voir la fiche de la session".to_owned(),
        (_, true) => "see the session page".to_owned(),
    };
    if en {
        format!("Reported by the network and validated by IFDD: {detail}.")
    } else {
        format!("Signalé par le réseau et validé par l'IFDD : {detail}.")
    }
}

pub fn changement_de_session(
    e: &Envoi<'_>,
    s: &EtatSession,
    etat: Etat,
    chemin: &str,
) -> OutgoingMail {
    let en = en_anglais(e.locale);
    let ville = s.ville.as_deref();
    let titre = if en {
        s.title_en.as_str()
    } else {
        s.title_fr.as_deref().unwrap_or(&s.title_en)
    };
    let mut lignes = Vec::new();
    if s.annulee {
        lignes.push(if en {
            "The session will not take place.".to_owned()
        } else {
            "La session n'aura pas lieu.".to_owned()
        });
    } else {
        let jour = if en { jour_en(s.jour) } else { jour_fr(s.jour) };
        let plage = match &s.fin {
            Some(fin) => format!("{}–{fin}", s.debut),
            None => s.debut.clone(),
        };
        lignes.push(if en {
            format!("Time: {jour}, {plage}{}", fuseau(ville, &s.fuseau, true))
        } else {
            format!(
                "Horaire : {jour}, {plage}{}",
                fuseau(ville, &s.fuseau, false)
            )
        });
        if let Some(salle) = &s.salle {
            lignes.push(if en {
                format!("Room: {salle}")
            } else {
                format!("Salle : {salle}")
            });
        }
    }
    if !s.changes.is_empty() {
        lignes.push(if en {
            "Per the official source.".to_owned()
        } else {
            "Selon la source officielle.".to_owned()
        });
    }
    lignes.extend(
        s.signalements
            .iter()
            .map(|r| ligne_signalement(r, ville, &s.fuseau, en)),
    );
    avis_par_courriel(e, etat, titre, &lignes, chemin)
}

pub fn reunion_non_annoncee(e: &Envoi<'_>, r: &EtatReunion, chemin: &str) -> OutgoingMail {
    let en = en_anglais(e.locale);
    let mut quand = if en { jour_en(r.jour) } else { jour_fr(r.jour) };
    if let Some(h) = &r.heure {
        quand.push_str(&if en {
            format!(" at {h}{}", fuseau(r.ville.as_deref(), &r.fuseau, true))
        } else {
            format!(" à {h}{}", fuseau(r.ville.as_deref(), &r.fuseau, false))
        });
    }
    let mut lignes = vec![if en {
        format!("When: {quand}")
    } else {
        format!("Quand : {quand}")
    }];
    if let Some(lieu) = &r.lieu {
        lignes.push(if en {
            format!("Where: {lieu}")
        } else {
            format!("Où : {lieu}")
        });
    }
    lignes.push(if en {
        "Reported by the network and validated by IFDD.".to_owned()
    } else {
        "Signalée par le réseau et validée par l'IFDD.".to_owned()
    });
    avis_par_courriel(e, Etat::NonAnnoncee, &r.titre, &lignes, chemin)
}

fn avis_par_courriel(
    e: &Envoi<'_>,
    etat: Etat,
    titre: &str,
    lignes: &[String],
    chemin: &str,
) -> OutgoingMail {
    let en = en_anglais(e.locale);
    let lien = format!("{}{chemin}", e.app_public_url.trim_end_matches('/'));
    let (subject, text) = if en {
        (
            format!("{} — {titre}", etat.en()),
            format!(
                "Hello,\n\n{} — {titre}\n\n{}\n\nOpen the page:\n\n{lien}\n\n\
                 You receive this email because you follow this session in Guide Négo. \
                 To stop these emails: Guide Négo, About, Notifications.\n\n\
                 Negotiation sessions — Guide Négo, IFDD",
                etat.en(),
                lignes.join("\n"),
            ),
        )
    } else {
        (
            format!("{} — {titre}", etat.fr()),
            format!(
                "Bonjour,\n\n{} — {titre}\n\n{}\n\nOuvrir la fiche :\n\n{lien}\n\n\
                 Vous recevez ce courriel parce que vous suivez cette session dans Guide Négo. \
                 Pour ne plus en recevoir : Guide Négo, À propos, Notifications.\n\n\
                 Sessions de négociation — Guide Négo, IFDD",
                etat.fr(),
                lignes.join("\n"),
            ),
        )
    };
    OutgoingMail {
        message_id: e.message_id.to_owned(),
        to: e.to.to_owned(),
        locale: e.locale.to_owned(),
        subject,
        text,
        html: None,
    }
}
