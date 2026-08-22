//! Types métier purs : ils se testent sans base.
//!
//! Le fichier qui compte est [`guards`] — la table qui dit **qui** peut poser un
//! fichier sur quoi, parce qu'aucune permission `media.*` n'existe dans le
//! modèle (écart n° 127). Une ligne oubliée y serait une porte ouverte, et un
//! test d'intégration confronte donc cette table à la base.

pub mod asset;
pub mod attachment;
pub mod duration;
pub mod guards;
pub mod ids;
pub mod imaging;
pub mod keys;
pub mod rules;
pub mod variants;
