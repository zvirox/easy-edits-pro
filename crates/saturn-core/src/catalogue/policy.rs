// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Session policy, tool profiles, and permission tiers (AF1).

use std::{collections::HashSet, fmt};
use serde::{Deserialize, Serialize};

use super::{CommandSpec, PROFILES, Tier};

/// Session operating mode governing permission grants and agent authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionMode {
    Autopilot,
    CoEdit,
    Review,
    Manual,
}

impl SessionMode {
    pub fn is_read_only(self) -> bool {
        matches!(self, Self::Review | Self::Manual)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Autopilot => "autopilot",
            Self::CoEdit => "coedit",
            Self::Review => "review",
            Self::Manual => "manual",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "autopilot" => Some(Self::Autopilot),
            "coedit" | "co-edit" => Some(Self::CoEdit),
            "review" => Some(Self::Review),
            "manual" => Some(Self::Manual),
            _ => None,
        }
    }

    pub fn default_tiers(self) -> HashSet<Tier> {
        if self.is_read_only() {
            [Tier::Read].into_iter().collect()
        } else {
            [Tier::Read, Tier::Edit].into_iter().collect()
        }
    }
}

/// Metadata describing one of the 12 canonical tool profiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ProfileInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub candidate_capabilities: &'static str,
}

pub const ALL_PROFILE_INFOS: &[ProfileInfo] = &[
    ProfileInfo {
        name: "admin",
        description: "Project settings and interchange.",
        candidate_capabilities: "Project settings, scratch disks, media pool management, interchange.",
    },
    ProfileInfo {
        name: "audio",
        description: "Telemetry, phase alignment, room tone, ducking, normalization, approved plugin loading.",
        candidate_capabilities: "Audio telemetry, DSP processing, phase alignment, room tone, plugin management.",
    },
    ProfileInfo {
        name: "captions",
        description: "Transcription, caption editing/style, subtitle export with its own grant.",
        candidate_capabilities: "Speech transcription, caption generation, subtitle formatting and export.",
    },
    ProfileInfo {
        name: "colour",
        description: "Scopes, shot matching, colour nodes and transforms.",
        candidate_capabilities: "Video scope telemetry, shot matching, primary/secondary color nodes, LUT transforms.",
    },
    ProfileInfo {
        name: "export",
        description: "Render lifecycle and delivery presets, with appropriate write grants.",
        candidate_capabilities: "Render start/status/cancel, delivery presets, export queue lifecycle.",
    },
    ProfileInfo {
        name: "fine-cut",
        description: "Trim, J/L through audio_process, match-cut, beat-grid, continuity, transitions.",
        candidate_capabilities: "Match cut alignment, beat-grid quantization, precision trim, cut continuity analysis.",
    },
    ProfileInfo {
        name: "graphics",
        description: "Titles and supported template operations.",
        candidate_capabilities: "Title generation, motion graphics, overlay graphics, template manipulation.",
    },
    ProfileInfo {
        name: "ingest",
        description: "Import, fingerprint, relink suggestions, labels, bins.",
        candidate_capabilities: "Media import, semantic indexing, metadata extraction, contact sheet generation.",
    },
    ProfileInfo {
        name: "multicam",
        description: "Angle synchronization and switching.",
        candidate_capabilities: "Multi-camera angle sync, audio/timecode alignment, live angle switching.",
    },
    ProfileInfo {
        name: "review",
        description: "Read-only inspection and verification; no edit/undo/branch mutation.",
        candidate_capabilities: "Read-only scopes, audio telemetry, cut evaluation, document inspection, proxy preview.",
    },
    ProfileInfo {
        name: "rough-cut",
        description: "Script alignment, transcription, insert/trim/ripple, markers, semantic search, shot detection.",
        candidate_capabilities: "Story assembly, rough trimming, ripple edits, semantic search, marker placement.",
    },
    ProfileInfo {
        name: "speed-motion",
        description: "Retime, stabilization and tracking.",
        candidate_capabilities: "Time remapping, optical flow speed ramps, stabilization, motion tracking.",
    },
];

pub fn profile_info(name: &str) -> Option<&'static ProfileInfo> {
    ALL_PROFILE_INFOS.iter().find(|info| info.name == name)
}

pub fn all_profiles() -> &'static [ProfileInfo] {
    ALL_PROFILE_INFOS
}

/// Errors returned when a command or tool violates the session's policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyError {
    ProfileRequired {
        id: String,
        required_profiles: Vec<String>,
        is_tool: bool,
    },
    PermissionDenied {
        id: String,
        required_tier: Tier,
        granted: Vec<Tier>,
        is_tool: bool,
    },
    ReadOnlySession {
        id: String,
        mode: SessionMode,
        required_tier: Tier,
        is_tool: bool,
    },
    InvalidProfile(String),
}

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProfileRequired {
                id,
                required_profiles,
                is_tool,
            } => {
                let kind = if *is_tool { "tool" } else { "command" };
                let hint = if required_profiles.len() == 1 {
                    required_profiles[0].clone()
                } else {
                    required_profiles.join("' or '")
                };
                write!(
                    f,
                    "ProfileRequired: {kind} '{id}' requires profile '{hint}'. Call tools.load_profile."
                )
            }
            Self::PermissionDenied {
                id,
                required_tier,
                granted,
                is_tool,
            } => {
                let kind = if *is_tool { "tool" } else { "command" };
                let granted_str = granted
                    .iter()
                    .map(|t| t.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(
                    f,
                    "PermissionDenied: {kind} '{id}' requires tier '{}', but session only holds [{granted_str}].",
                    required_tier.as_str()
                )
            }
            Self::ReadOnlySession {
                id,
                mode,
                required_tier,
                ..
            } => {
                write!(
                    f,
                    "PermissionDenied: session in {} mode is read-only. '{id}' requires tier '{}'.",
                    mode.as_str(),
                    required_tier.as_str()
                )
            }
            Self::InvalidProfile(name) => {
                write!(
                    f,
                    "InvalidProfile: unknown profile '{name}'. Valid profiles: {}",
                    PROFILES.join(", ")
                )
            }
        }
    }
}

impl std::error::Error for PolicyError {}

/// Session policy tracking active profiles and granted permission tiers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionPolicy {
    pub mode: SessionMode,
    pub active_profiles: HashSet<String>,
    pub granted_tiers: HashSet<Tier>,
}

impl Default for SessionPolicy {
    fn default() -> Self {
        Self {
            mode: SessionMode::Autopilot,
            active_profiles: ["rough-cut".to_string()].into_iter().collect(),
            granted_tiers: [Tier::Read, Tier::Edit].into_iter().collect(),
        }
    }
}

impl SessionPolicy {
    pub fn new(mode: SessionMode, profiles: &[&str], tiers: &[Tier]) -> Self {
        Self {
            mode,
            active_profiles: profiles.iter().map(|s| s.to_string()).collect(),
            granted_tiers: tiers.iter().copied().collect(),
        }
    }

    pub fn review() -> Self {
        Self {
            mode: SessionMode::Review,
            active_profiles: ["review".to_string()].into_iter().collect(),
            granted_tiers: [Tier::Read].into_iter().collect(),
        }
    }

    pub fn manual() -> Self {
        Self {
            mode: SessionMode::Manual,
            active_profiles: ["review".to_string()].into_iter().collect(),
            granted_tiers: [Tier::Read].into_iter().collect(),
        }
    }

    /// Load an additional profile into the active profile set.
    ///
    /// CRITICAL: Loading a profile cannot elevate permission tiers (plan §2.1).
    pub fn load_profile(&mut self, profile: &str) -> Result<bool, PolicyError> {
        if !PROFILES.contains(&profile) {
            return Err(PolicyError::InvalidProfile(profile.to_string()));
        }
        Ok(self.active_profiles.insert(profile.to_string()))
    }

    pub fn check_command(&self, spec: &CommandSpec) -> Result<(), PolicyError> {
        let tiers_to_check: &[Tier] = if spec.tiers.is_empty() {
            std::slice::from_ref(&spec.tier)
        } else {
            spec.tiers
        };

        for tier in tiers_to_check {
            if !self.granted_tiers.contains(tier) {
                if self.mode.is_read_only() {
                    return Err(PolicyError::ReadOnlySession {
                        id: spec.id.to_string(),
                        mode: self.mode,
                        required_tier: *tier,
                        is_tool: false,
                    });
                }
                let mut granted_vec: Vec<Tier> = self.granted_tiers.iter().copied().collect();
                granted_vec.sort();
                return Err(PolicyError::PermissionDenied {
                    id: spec.id.to_string(),
                    required_tier: *tier,
                    granted: granted_vec,
                    is_tool: false,
                });
            }
        }

        if spec.profiles.is_empty() || spec.profiles.contains(&"core") {
            return Ok(());
        }

        let has_profile = spec
            .profiles
            .iter()
            .any(|p| self.active_profiles.contains(*p));
        if !has_profile {
            return Err(PolicyError::ProfileRequired {
                id: spec.id.to_string(),
                required_profiles: spec.profiles.iter().map(|s| s.to_string()).collect(),
                is_tool: false,
            });
        }

        Ok(())
    }

    pub fn check_tool(
        &self,
        name: &str,
        required_tiers: &[Tier],
        required_profiles: &[&str],
    ) -> Result<(), PolicyError> {
        for tier in required_tiers {
            if !self.granted_tiers.contains(tier) {
                if self.mode.is_read_only() {
                    return Err(PolicyError::ReadOnlySession {
                        id: name.to_string(),
                        mode: self.mode,
                        required_tier: *tier,
                        is_tool: true,
                    });
                }
                let mut granted_vec: Vec<Tier> = self.granted_tiers.iter().copied().collect();
                granted_vec.sort();
                return Err(PolicyError::PermissionDenied {
                    id: name.to_string(),
                    required_tier: *tier,
                    granted: granted_vec,
                    is_tool: true,
                });
            }
        }

        if required_profiles.is_empty() || required_profiles.contains(&"core") {
            return Ok(());
        }

        let has_profile = required_profiles
            .iter()
            .any(|p| self.active_profiles.contains(*p));
        if !has_profile {
            return Err(PolicyError::ProfileRequired {
                id: name.to_string(),
                required_profiles: required_profiles.iter().map(|s| s.to_string()).collect(),
                is_tool: true,
            });
        }

        Ok(())
    }

    pub fn is_tool_visible(&self, required_profiles: &[&str]) -> bool {
        required_profiles.is_empty()
            || required_profiles.contains(&"core")
            || required_profiles
                .iter()
                .any(|p| self.active_profiles.contains(*p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_catalogue;

    #[test]
    fn default_policy_has_rough_cut_and_edit_grants() {
        let policy = SessionPolicy::default();
        assert_eq!(policy.mode, SessionMode::Autopilot);
        assert!(policy.active_profiles.contains("rough-cut"));
        assert!(policy.granted_tiers.contains(&Tier::Read));
        assert!(policy.granted_tiers.contains(&Tier::Edit));
        assert!(!policy.granted_tiers.contains(&Tier::Filesystem));
        assert!(!policy.granted_tiers.contains(&Tier::Destructive));
    }

    #[test]
    fn wrong_profile_returns_typed_profile_required_hint() {
        let policy = SessionPolicy::default(); // rough-cut only
        let catalogue = command_catalogue();
        let color_cmd = catalogue.iter().find(|c| c.id == "color.node_add").unwrap();

        let err = policy.check_command(color_cmd).unwrap_err();
        let err_str = err.to_string();
        match &err {
            PolicyError::ProfileRequired { id, required_profiles, is_tool } => {
                assert_eq!(id, "color.node_add");
                assert!(required_profiles.contains(&"colour".to_string()));
                assert!(!is_tool);
            }
            other => panic!("expected ProfileRequired, got {other:?}"),
        }
        assert!(err_str.contains("ProfileRequired: command 'color.node_add' requires profile 'colour'"));
        assert!(err_str.contains("Call tools.load_profile."));
    }

    #[test]
    fn loading_profile_enables_commands_without_permission_elevation() {
        let mut policy = SessionPolicy::default();
        let catalogue = command_catalogue();
        let color_cmd = catalogue.iter().find(|c| c.id == "color.node_add").unwrap();

        assert!(policy.check_command(color_cmd).is_err());
        let was_new = policy.load_profile("colour").unwrap();
        assert!(was_new);
        assert!(policy.check_command(color_cmd).is_ok());

        // Audio plugin command requiring filesystem still fails (no elevation)
        let plugin_cmd = catalogue.iter().find(|c| c.id == "dsp.load_plugin").unwrap();
        policy.load_profile("audio").unwrap();
        let err = policy.check_command(plugin_cmd).unwrap_err();
        assert!(matches!(err, PolicyError::PermissionDenied { .. }));
        assert!(err.to_string().contains("PermissionDenied: command 'dsp.load_plugin' requires tier 'filesystem'"));
    }

    #[test]
    fn review_session_is_strictly_read_only() {
        let mut policy = SessionPolicy::review();
        let catalogue = command_catalogue();
        let split_cmd = catalogue.iter().find(|c| c.id == "timeline.split_clip").unwrap();

        let err = policy.check_command(split_cmd).unwrap_err();
        assert!(matches!(err, PolicyError::ReadOnlySession { .. }));
        assert!(err.to_string().contains("is read-only"));

        // Loading rough-cut profile still leaves edits blocked
        policy.load_profile("rough-cut").unwrap();
        let err = policy.check_command(split_cmd).unwrap_err();
        assert!(matches!(err, PolicyError::ReadOnlySession { .. }));
    }

    #[test]
    fn invalid_profile_name_is_rejected() {
        let mut policy = SessionPolicy::default();
        let err = policy.load_profile("hacker-mode").unwrap_err();
        assert!(matches!(err, PolicyError::InvalidProfile(_)));
        assert!(err.to_string().contains("Valid profiles: admin, audio, captions"));
    }
}

