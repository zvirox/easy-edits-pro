//! Interop with external interchange formats: CMX 3600 EDL, FCPXML,
//! Editor XML exchange (xmeml), OpenTimelineIO, plus SMPTE timecode helpers.
//!
//! Split by format; the public names are re-exported here so
//! `crate::interop::*` (and `saturn_core::*`) callers are unchanged.

mod edl;
mod fcpxml;
mod otio;
mod xmeml;
mod timecode;
mod xml_utils;

#[cfg(test)]
mod tests;

pub use edl::{export_cmx3600_edl, import_cmx3600_edl};
pub use fcpxml::{export_fcpxml, import_fcpxml};
pub use otio::{export_otio, import_otio};
pub use xmeml::{export_xmeml_xml, import_xmeml_xml};
pub use timecode::{ticks_to_timecode, timecode_to_ticks};
