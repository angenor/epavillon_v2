//! Le jour J d'une activité retenue : ce que la fiche du dossier lit et ce
//! qu'elle refuse, sans base.

use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

/// `ProposalDayPlan`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProposalDayPlan {
    pub focal_points: Vec<FocalPoint>,
    pub technical_needs: TechnicalNeeds,
    pub social_posts: Vec<SocialPost>,
    /// Dix jours avant la première séance ; nulle sans séance. Indicative.
    #[serde(with = "time::serde::rfc3339::option")]
    pub needs_deadline: Option<OffsetDateTime>,
}

/// `FocalPoint`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FocalPoint {
    /// Nul pour le déposant proposé par défaut, tant que rien n'est enregistré.
    pub id: Option<Uuid>,
    pub speaker_id: Option<Uuid>,
    pub person_id: Option<Uuid>,
    pub full_name: String,
    pub phone: String,
    pub whatsapp: String,
    pub is_default: bool,
}

/// `TechnicalNeeds`.
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct TechnicalNeeds {
    pub video: bool,
    pub remote: bool,
    pub remote_details: String,
    pub accessibility: bool,
    pub accessibility_details: String,
    pub other: String,
    /// Nul tant que rien n'est déclaré, même « aucun besoin ».
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<OffsetDateTime>,
}

/// `SocialPost`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SocialPost {
    pub id: Uuid,
    pub url: String,
    pub platform: SocialPlatform,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// `SocialPlatform` : déduite de l'adresse à la lecture, jamais stockée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SocialPlatform {
    Linkedin,
    Facebook,
    X,
    Instagram,
    Youtube,
    Tiktok,
    Other,
}

/// Domaines reconnus, raccourcisseurs compris : un lien partagé depuis un
/// téléphone arrive souvent sous sa forme courte.
const DOMAINES: &[(&str, SocialPlatform)] = &[
    ("linkedin.com", SocialPlatform::Linkedin),
    ("lnkd.in", SocialPlatform::Linkedin),
    ("facebook.com", SocialPlatform::Facebook),
    ("fb.com", SocialPlatform::Facebook),
    ("fb.watch", SocialPlatform::Facebook),
    ("fb.me", SocialPlatform::Facebook),
    ("x.com", SocialPlatform::X),
    ("twitter.com", SocialPlatform::X),
    ("t.co", SocialPlatform::X),
    ("instagram.com", SocialPlatform::Instagram),
    ("instagr.am", SocialPlatform::Instagram),
    ("youtube.com", SocialPlatform::Youtube),
    ("youtu.be", SocialPlatform::Youtube),
    ("tiktok.com", SocialPlatform::Tiktok),
];

/// Longueur de `platform.url`.
pub const URL_MAX: usize = 2048;

/// Points focaux par dossier, au plus.
pub const POINTS_FOCAUX_MAX: usize = 10;

/// Publications par dossier, au plus.
pub const PUBLICATIONS_MAX: i64 = 50;

/// L'adresse saisie, prête à enregistrer : espaces de bord retirés, schéma en
/// minuscules (`platform.url` ne reconnaît que `http://` et `https://`). Rien
/// si elle n'est pas une adresse http(s) complète.
///
/// Volontairement strict : ce lien part chez l'IFDD pour être relayé, et une
/// adresse tronquée par un copier-coller doit se voir à la saisie.
pub fn normaliser_url(saisie: &str) -> Option<String> {
    let saisie = saisie.trim();
    let (schema, reste) = saisie.split_once("://")?;
    let schema = schema.to_ascii_lowercase();
    if schema != "http" && schema != "https" {
        return None;
    }
    let url = format!("{schema}://{reste}");
    hote_de(&url).map(|_| url)
}

/// L'hôte d'une adresse normalisée, en minuscules, ou rien.
fn hote_de(url: &str) -> Option<String> {
    if url.len() > URL_MAX || url.chars().any(char::is_whitespace) {
        return None;
    }
    let reste = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let autorite = reste.split(['/', '?', '#']).next()?;
    let hote = autorite.rsplit('@').next()?;
    let hote = hote.split(':').next()?.to_ascii_lowercase();
    let etiquettes: Vec<&str> = hote.split('.').collect();
    let valide = etiquettes.len() >= 2
        && etiquettes
            .iter()
            .all(|e| !e.is_empty() && e.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
    valide.then_some(hote)
}

/// La plateforme d'une adresse déjà validée.
pub fn plateforme_de(url: &str) -> SocialPlatform {
    let Some(hote) = hote_de(url) else {
        return SocialPlatform::Other;
    };
    DOMAINES
        .iter()
        .find(|(domaine, _)| {
            hote == *domaine
                || hote
                    .strip_suffix(domaine)
                    .is_some_and(|debut| debut.ends_with('.'))
        })
        .map_or(SocialPlatform::Other, |(_, p)| *p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_reseaux_se_reconnaissent_sous_toutes_leurs_formes() {
        for (url, attendu) in [
            (
                "https://www.linkedin.com/posts/ifdd_cop31-activity-1",
                SocialPlatform::Linkedin,
            ),
            ("https://lnkd.in/eAbC123", SocialPlatform::Linkedin),
            (
                "https://m.facebook.com/story.php?id=1",
                SocialPlatform::Facebook,
            ),
            ("https://fb.watch/abc/", SocialPlatform::Facebook),
            ("https://twitter.com/ifdd/status/1", SocialPlatform::X),
            ("https://x.com/ifdd/status/1", SocialPlatform::X),
            (
                "https://www.instagram.com/p/Cxyz/",
                SocialPlatform::Instagram,
            ),
            ("https://youtu.be/dQw4w9WgXcQ", SocialPlatform::Youtube),
            (
                "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
                SocialPlatform::Youtube,
            ),
            ("https://vm.tiktok.com/ZMabc/", SocialPlatform::Tiktok),
            ("https://WWW.LINKEDIN.COM/feed", SocialPlatform::Linkedin),
            (
                "https://www.ifdd.francophonie.org/actu",
                SocialPlatform::Other,
            ),
        ] {
            assert_eq!(plateforme_de(url), attendu, "{url}");
        }
    }

    /// Un domaine qui finit comme un réseau n'en est pas un.
    #[test]
    fn un_suffixe_sans_point_ne_trompe_pas() {
        assert_eq!(plateforme_de("https://notx.com/a"), SocialPlatform::Other);
        assert_eq!(
            plateforme_de("https://fakelinkedin.com/a"),
            SocialPlatform::Other
        );
    }

    #[test]
    fn seule_une_adresse_http_complete_passe() {
        assert_eq!(
            normaliser_url("  HTTPS://www.LinkedIn.com/posts/1 ").as_deref(),
            Some("https://www.LinkedIn.com/posts/1")
        );
        assert!(normaliser_url("http://exemple.org").is_some());
        assert!(normaliser_url("https://user@exemple.org:8443/a").is_some());
        for refus in [
            "",
            "linkedin.com/posts/1",
            "ftp://exemple.org/a",
            "https://",
            "https://localhost/a",
            "https://exemple .org/a",
            "javascript:alert(1)",
            "https://exemple..org",
        ] {
            assert!(normaliser_url(refus).is_none(), "{refus}");
        }
    }
}
