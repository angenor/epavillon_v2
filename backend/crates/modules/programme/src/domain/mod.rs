//! Types métier purs : ils se testent sans base.
//!
//! **Il est plus fourni que dans les trois modules précédents, et c'est
//! délibéré** : ce module porte dix règles que la base ne porte pas
//! (data-model.md § 3). Les loger dans le service les rendrait intestables sans
//! base ; ici, chacune se prouve seule — et `tests/domaine.rs` les éprouve sans
//! ouvrir une connexion.

pub mod blind;
pub mod bulk;
pub mod desk;
pub mod draft;
pub mod eligibility;
pub mod facets;
pub mod ids;
pub mod limits;
pub mod ownership;
pub mod permissions;
pub mod sanitize;
pub mod slug;
pub mod transitions;
