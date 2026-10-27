//! Time representation module

use serde::{Deserialize, Serialize};

/// Timeline position in ticks (254,016,000,000 ticks per second = 1/254016000000s resolution)
pub type Ticks = i64;

/// Ticks per second constant (matching the reference editor's 254B ticks/sec)
pub const TICKS_PER_SECOND: Ticks = 254_016_000_000;

/// Frame rates supported
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameRate {
    Fps23_976,
    Fps24,
    Fps25,
    Fps29_97,
    Fps30,
    Fps48,
    Fps50,
    Fps59_94,
    Fps60,
    Custom { numerator: u32, denominator: u32 },
}

impl FrameRate {
    pub fn ticks_per_frame(&self) -> Ticks {
        match self {
            FrameRate::Fps23_976 => TICKS_PER_SECOND / 24000 * 1001,
            FrameRate::Fps24 => TICKS_PER_SECOND / 24,
            FrameRate::Fps25 => TICKS_PER_SECOND / 25,
            FrameRate::Fps29_97 => TICKS_PER_SECOND / 30000 * 1001,
            FrameRate::Fps30 => TICKS_PER_SECOND / 30,
            FrameRate::Fps48 => TICKS_PER_SECOND / 48,
            FrameRate::Fps50 => TICKS_PER_SECOND / 50,
            FrameRate::Fps59_94 => TICKS_PER_SECOND / 60000 * 1001,
            FrameRate::Fps60 => TICKS_PER_SECOND / 60,
            FrameRate::Custom {
                numerator,
                denominator,
            } => TICKS_PER_SECOND * *denominator as i64 / *numerator as i64,
        }
    }

    pub fn fps(&self) -> f64 {
        match self {
            FrameRate::Fps23_976 => 24000.0 / 1001.0,
            FrameRate::Fps24 => 24.0,
            FrameRate::Fps25 => 25.0,
            FrameRate::Fps29_97 => 30000.0 / 1001.0,
            FrameRate::Fps30 => 30.0,
            FrameRate::Fps48 => 48.0,
            FrameRate::Fps50 => 50.0,
            FrameRate::Fps59_94 => 60000.0 / 1001.0,
            FrameRate::Fps60 => 60.0,
            FrameRate::Custom {
                numerator,
                denominator,
            } => *numerator as f64 / *denominator as f64,
        }
    }
}

impl Default for FrameRate {
    fn default() -> Self {
        FrameRate::Fps30
    }
}

/// Timecode representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timecode {
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
    pub frames: u8,
    pub drop_frame: bool,
}

impl Timecode {
    pub fn new(hours: u8, minutes: u8, seconds: u8, frames: u8, drop_frame: bool) -> Self {
        Self {
            hours,
            minutes,
            seconds,
            frames,
            drop_frame,
        }
    }

    pub fn from_ticks(ticks: Ticks, frame_rate: FrameRate, drop_frame: bool) -> Self {
        let total_frames = (ticks as f64 / frame_rate.ticks_per_frame() as f64).round() as i64;
        let fps = frame_rate.fps().round() as i64;

        let mut total = total_frames;
        if drop_frame && fps == 30 {
            let frames_per_minute = fps * 60;
            let frames_per_10min = frames_per_minute * 10 - 18;
            let d = total / frames_per_10min;
            let m = total % frames_per_10min;
            total += d * 18 + ((m - 2) / (frames_per_minute - 2)).max(0) * 2;
        }

        let frames = (total % fps).max(0) as u8;
        let total_seconds = (total / fps).max(0);
        let seconds = (total_seconds % 60) as u8;
        let total_minutes = total_seconds / 60;
        let minutes = (total_minutes % 60) as u8;
        let hours = (total_minutes / 60) as u8;

        Self {
            hours,
            minutes,
            seconds,
            frames,
            drop_frame,
        }
    }

    pub fn to_ticks(&self, frame_rate: FrameRate) -> Ticks {
        let fps = frame_rate.fps().round() as i64;
        let total_frames = self.hours as i64 * 3600 * fps
            + self.minutes as i64 * 60 * fps
            + self.seconds as i64 * fps
            + self.frames as i64;

        (total_frames * frame_rate.ticks_per_frame()) as Ticks
    }

    pub fn format(&self) -> String {
        if self.drop_frame {
            format!(
                "{:02}:{:02}:{:02};{:02}",
                self.hours, self.minutes, self.seconds, self.frames
            )
        } else {
            format!(
                "{:02}:{:02}:{:02}:{:02}",
                self.hours, self.minutes, self.seconds, self.frames
            )
        }
    }
}

/// Timeline time representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TimelineTime {
    pub ticks: Ticks,
}

impl TimelineTime {
    pub const ZERO: Self = Self { ticks: 0 };
    pub const MAX: Self = Self { ticks: Ticks::MAX };

    pub fn new(ticks: Ticks) -> Self {
        Self { ticks }
    }

    pub fn from_seconds(seconds: f64) -> Self {
        Self {
            ticks: (seconds * TICKS_PER_SECOND as f64) as Ticks,
        }
    }

    pub fn from_frames(frames: i64, frame_rate: FrameRate) -> Self {
        Self {
            ticks: frames * frame_rate.ticks_per_frame(),
        }
    }

    pub fn from_timecode(tc: Timecode, frame_rate: FrameRate) -> Self {
        Self {
            ticks: tc.to_ticks(frame_rate),
        }
    }

    pub fn as_seconds(&self) -> f64 {
        self.ticks as f64 / TICKS_PER_SECOND as f64
    }

    pub fn as_frames(&self, frame_rate: FrameRate) -> i64 {
        self.ticks / frame_rate.ticks_per_frame()
    }

    pub fn as_timecode(&self, frame_rate: FrameRate, drop_frame: bool) -> Timecode {
        Timecode::from_ticks(self.ticks, frame_rate, drop_frame)
    }

    pub fn add(&self, other: Self) -> Self {
        Self {
            ticks: self.ticks.saturating_add(other.ticks),
        }
    }

    pub fn sub(&self, other: Self) -> Self {
        Self {
            ticks: self.ticks.saturating_sub(other.ticks),
        }
    }

    pub fn mul(&self, factor: f64) -> Self {
        Self {
            ticks: (self.ticks as f64 * factor) as Ticks,
        }
    }

    pub fn div(&self, divisor: f64) -> Self {
        Self {
            ticks: (self.ticks as f64 / divisor) as Ticks,
        }
    }
}

impl std::ops::Add for TimelineTime {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            ticks: self.ticks.saturating_add(other.ticks),
        }
    }
}

impl std::ops::Sub for TimelineTime {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            ticks: self.ticks.saturating_sub(other.ticks),
        }
    }
}

impl std::ops::Mul<f64> for TimelineTime {
    type Output = Self;
    fn mul(self, factor: f64) -> Self {
        Self {
            ticks: (self.ticks as f64 * factor) as Ticks,
        }
    }
}

impl std::ops::Div<f64> for TimelineTime {
    type Output = Self;
    fn div(self, divisor: f64) -> Self {
        Self {
            ticks: (self.ticks as f64 / divisor) as Ticks,
        }
    }
}

impl std::fmt::Display for TimelineTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tc = self.as_timecode(FrameRate::Fps30, false);
        write!(f, "{}", tc.format())
    }
}

/// Color spaces
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorSpace {
    Rec709,
    Rec2020,
    P3D65,
    Srgb,
    Linear,
    Custom,
}

impl Default for ColorSpace {
    fn default() -> Self {
        ColorSpace::Rec709
    }
}

/// Timeline errors
#[derive(Debug, thiserror::Error)]
pub enum TimelineError {
    #[error("Track not found")]
    TrackNotFound,
    #[error("Clip not found")]
    ClipNotFound,
    #[error("Sequence already exists")]
    SequenceAlreadyExists,
    #[error("Invalid time range")]
    InvalidTimeRange,
    #[error("Track locked")]
    TrackLocked,
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_conversions() {
        let time = TimelineTime::from_seconds(1.5);
        assert_eq!(
            time.ticks,
            TICKS_PER_SECOND as i64 + TICKS_PER_SECOND as i64 / 2
        );
        assert!((time.as_seconds() - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_timecode_conversion() {
        let tc = Timecode::new(1, 30, 45, 15, false);
        let time = TimelineTime::from_timecode(tc, FrameRate::Fps30);
        let tc2 = time.as_timecode(FrameRate::Fps30, false);
        assert_eq!(tc, tc2);
    }

    #[test]
    fn test_frame_rate_ticks() {
        assert_eq!(FrameRate::Fps30.ticks_per_frame(), TICKS_PER_SECOND / 30);
        assert_eq!(FrameRate::Fps24.ticks_per_frame(), TICKS_PER_SECOND / 24);
    }

    #[test]
    fn test_time_arithmetic() {
        let t1 = TimelineTime::from_seconds(10.0);
        let t2 = TimelineTime::from_seconds(5.0);
        assert_eq!((t1 + t2).as_seconds(), 15.0);
        assert_eq!((t1 - t2).as_seconds(), 5.0);
    }
}
