//! One module per `pmctl` subcommand, plus shared device-selection
//! (`device`) and driver-registration (`registry`) logic.

mod device;
pub mod discover;
pub mod info;
pub mod lighting;
pub mod list;
pub mod profile;
mod registry;
pub mod rgb;
pub mod sleep;
