//! Les travaux de la file « live ». Le worker n'écoute cette file que
//! parce que ces gestionnaires la déclarent.

pub mod cancel_meeting;
pub mod cancel_participant;
pub mod filet;
pub mod sync_meeting;
pub mod sync_participant;
pub mod webhook;
