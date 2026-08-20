//! Les lectures publiques — **aucune session, aucun périmètre**.
//!
//! Ce fichier est court, et c'est un bon signe : le critère de publicité vit
//! dans `event.v_public_editions`, donc dans le modèle. Le recopier ici serait
//! exactement l'écart n° 26 — chaque écran filtrant à sa façon, jusqu'à ce que
//! l'un d'eux se trompe. La v1 en est morte : son accueil filtrait sur
//! `upcoming|ongoing` avant de trier sur `completed`, si bien qu'aucun événement
//! passé ne s'y affichait jamais.
//!
//! Il n'y a donc **rien à décider ici** : le service nomme les lectures, il ne
//! les corrige pas.

use kernel::error::Result;
use sqlx::PgPool;

use crate::domain::ids::EventId;
use crate::domain::public::PublicEdition;
use crate::repo::{cross, public};

/// Les éditions publiques, décroissantes sur la date de début.
pub async fn editions(pool: &PgPool) -> Result<Vec<PublicEdition>> {
    public::editions(pool).await
}

/// La page d'une édition, par son adresse d'URL.
///
/// **Une requête, deux vues** (research.md § R16) : l'édition, sa série, son
/// pays, ses trois déclinaisons d'image, son état temporel, son appel résolu et
/// le volume de son programme publié arrivent ensemble. C'est ce qui referme
/// l'écart n° 25 sans écrire une ligne pour lui — la lecture d'image séparée du
/// front n'a plus d'objet.
///
/// `None` pour un brouillon, une annulée ou une adresse inconnue : les trois
/// sont **indiscernables**.
pub async fn edition_par_slug(pool: &PgPool, slug: &str) -> Result<Option<PublicEdition>> {
    public::edition_par_slug(pool, slug).await
}

/// **Les trois déclinaisons d'une édition, servies à part.**
///
/// Cette lecture est **vouée à disparaître** : `GET /events/{slug}` porte
/// désormais les trois images résolues, et cet appel supplémentaire n'a plus de
/// raison d'être. Elle est livrée pour ne pas casser un écran déjà en place, et
/// son retrait est inscrit aux obligations de B7 (écart n° 25).
pub async fn images(pool: &PgPool, event_id: EventId) -> Result<serde_json::Value> {
    cross::images_de_l_edition(pool, event_id).await
}
