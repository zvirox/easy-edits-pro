//! Search Bins Implementation
//!
//! Smart search bins for dynamic media organization based on queries.

use chrono;
use indexmap::IndexMap;
use saturn_timeline::{FrameRate, Ticks};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Search bin query definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchBinQuery {
    pub rules: Vec<SearchBinRule>,
    pub logic: SearchLogic,
    pub limit: Option<usize>,
    pub sort_by: Option<SortField>,
    pub sort_order: SortOrder,
}

impl Default for SearchBinQuery {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            logic: SearchLogic::And,
            limit: None,
            sort_by: None,
            sort_order: SortOrder::Descending,
        }
    }
}

/// Search logic for combining rules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchLogic {
    And,
    Or,
}

/// Search bin rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchBinRule {
    pub field: SearchField,
    pub operator: SearchOperator,
    pub value: Value,
    pub case_sensitive: bool,
}

/// Search fields
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchField {
    Name,
    Path,
    FileName,
    Extension,
    FileSize,
    Duration,
    FrameRate,
    Resolution,
    Codec,
    DateCreated,
    DateModified,
    DateImported,
    Tags,
    Rating,
    ColorLabel,
    Metadata(String), // Custom metadata field
    MediaType,
    AudioChannels,
    SampleRate,
    BitDepth,
    Custom(String),
}

/// Search operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    StartsWith,
    EndsWith,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    In,
    NotIn,
    Regex,
    IsEmpty,
    IsNotEmpty,
    Between,
    WithinDays,
    WithinWeeks,
    WithinMonths,
}

/// Sort fields
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortField {
    Name,
    DateCreated,
    DateModified,
    DateImported,
    FileSize,
    Duration,
    Rating,
    Random,
}

/// Sort order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    Ascending,
    Descending,
}

/// Search bin definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchBin {
    pub id: Uuid,
    pub name: String,
    pub query: SearchBinQuery,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub is_dynamic: bool, // Updates automatically
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub modified_at: chrono::DateTime<chrono::Utc>,
    pub match_count: usize,
    #[serde(skip)]
    pub matched_items: Vec<SearchBinMatch>,
}

impl SearchBin {
    pub fn new(name: String, query: SearchBinQuery) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            query,
            color: None,
            icon: None,
            is_dynamic: true,
            created_at: chrono::Utc::now(),
            modified_at: chrono::Utc::now(),
            match_count: 0,
            matched_items: Vec::new(),
        }
    }

    pub fn with_color(mut self, color: String) -> Self {
        self.color = Some(color);
        self
    }

    pub fn with_icon(mut self, icon: String) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn set_dynamic(mut self, dynamic: bool) -> Self {
        self.is_dynamic = dynamic;
        self
    }

    /// Evaluate query against a media item
    pub fn matches(&self, item: &MediaItem) -> bool {
        self.evaluate_rules(&self.query.rules, item, &self.query.logic)
    }

    fn evaluate_rules(
        &self,
        rules: &[SearchBinRule],
        item: &MediaItem,
        logic: &SearchLogic,
    ) -> bool {
        if rules.is_empty() {
            return true;
        }

        let results: Vec<bool> = rules
            .iter()
            .map(|rule| self.evaluate_rule(rule, item))
            .collect();

        match logic {
            SearchLogic::And => results.iter().all(|&r| r),
            SearchLogic::Or => results.iter().any(|&r| r),
        }
    }

    fn evaluate_rule(&self, rule: &SearchBinRule, item: &MediaItem) -> bool {
        let field_value = self.get_field_value(item, &rule.field);
        self.compare_values(
            &field_value,
            &rule.operator,
            &rule.value,
            rule.case_sensitive,
        )
    }

    fn get_field_value(&self, item: &MediaItem, field: &SearchField) -> Value {
        match field {
            SearchField::Name => json!(item.name),
            SearchField::Path => json!(item.path.display().to_string()),
            SearchField::FileName => json!(item.file_name()),
            SearchField::Extension => json!(item.extension()),
            SearchField::FileSize => json!(item.file_size),
            SearchField::Duration => json!(item.duration),
            SearchField::FrameRate => json!(format!("{:?}", item.frame_rate)),
            SearchField::Resolution => json!(format!("{}x{}", item.width, item.height)),
            SearchField::Codec => json!(item.codec),
            SearchField::DateCreated => json!(item.date_created),
            SearchField::DateModified => json!(item.date_modified),
            SearchField::DateImported => json!(item.date_imported),
            SearchField::Tags => json!(item.tags),
            SearchField::Rating => json!(item.rating),
            SearchField::ColorLabel => json!(item.color_label),
            SearchField::Metadata(key) => item.metadata.get(key).cloned().unwrap_or(Value::Null),
            SearchField::MediaType => json!(format!("{:?}", item.media_type)),
            SearchField::AudioChannels => json!(item.audio_channels),
            SearchField::SampleRate => json!(item.sample_rate),
            SearchField::BitDepth => json!(item.bit_depth),
            SearchField::Custom(key) => item.metadata.get(key).cloned().unwrap_or(Value::Null),
        }
    }

    fn compare_values(
        &self,
        field_value: &Value,
        operator: &SearchOperator,
        rule_value: &Value,
        case_sensitive: bool,
    ) -> bool {
        use SearchOperator::*;

        match operator {
            Equals => self.values_equal(field_value, rule_value, case_sensitive),
            NotEquals => !self.values_equal(field_value, rule_value, case_sensitive),
            Contains => self.values_contains(field_value, rule_value, case_sensitive),
            NotContains => !self.values_contains(field_value, rule_value, case_sensitive),
            StartsWith => self.values_starts_with(field_value, rule_value, case_sensitive),
            EndsWith => self.values_ends_with(field_value, rule_value, case_sensitive),
            GreaterThan => self.compare_numeric(field_value, rule_value, |a, b| a > b),
            GreaterThanOrEqual => self.compare_numeric(field_value, rule_value, |a, b| a >= b),
            LessThan => self.compare_numeric(field_value, rule_value, |a, b| a < b),
            LessThanOrEqual => self.compare_numeric(field_value, rule_value, |a, b| a <= b),
            In => self.values_in(field_value, rule_value),
            NotIn => !self.values_in(field_value, rule_value),
            Regex => self.values_regex(field_value, rule_value),
            IsEmpty => {
                field_value.is_null()
                    || (field_value.is_string() && field_value.as_str().unwrap_or("").is_empty())
                    || (field_value.is_array()
                        && field_value.as_array().unwrap_or(&vec![]).is_empty())
            }
            IsNotEmpty => {
                !field_value.is_null()
                    && !(field_value.is_string() && field_value.as_str().unwrap_or("").is_empty())
                    && !(field_value.is_array()
                        && field_value.as_array().unwrap_or(&vec![]).is_empty())
            }
            Between => self.values_between(field_value, rule_value),
            WithinDays => self.values_within_days(field_value, rule_value),
            WithinWeeks => self.values_within_weeks(field_value, rule_value),
            WithinMonths => self.values_within_months(field_value, rule_value),
        }
    }

    fn values_equal(&self, a: &Value, b: &Value, case_sensitive: bool) -> bool {
        if a.is_string() && b.is_string() {
            let a_str = a.as_str().unwrap();
            let b_str = b.as_str().unwrap();
            if case_sensitive {
                a_str == b_str
            } else {
                a_str.eq_ignore_ascii_case(b_str)
            }
        } else {
            a == b
        }
    }

    fn values_contains(&self, a: &Value, b: &Value, case_sensitive: bool) -> bool {
        if a.is_string() && b.is_string() {
            let a_str = a.as_str().unwrap();
            let b_str = b.as_str().unwrap();
            if case_sensitive {
                a_str.contains(b_str)
            } else {
                a_str.to_lowercase().contains(&b_str.to_lowercase())
            }
        } else if a.is_array() && b.is_string() {
            let b_str = b.as_str().unwrap();
            a.as_array().unwrap().iter().any(|v| {
                if v.is_string() {
                    let v_str = v.as_str().unwrap();
                    if case_sensitive {
                        v_str == b_str
                    } else {
                        v_str.eq_ignore_ascii_case(b_str)
                    }
                } else {
                    false
                }
            })
        } else {
            false
        }
    }

    fn values_starts_with(&self, a: &Value, b: &Value, case_sensitive: bool) -> bool {
        if a.is_string() && b.is_string() {
            let a_str = a.as_str().unwrap();
            let b_str = b.as_str().unwrap();
            if case_sensitive {
                a_str.starts_with(b_str)
            } else {
                a_str.to_lowercase().starts_with(&b_str.to_lowercase())
            }
        } else {
            false
        }
    }

    fn values_ends_with(&self, a: &Value, b: &Value, case_sensitive: bool) -> bool {
        if a.is_string() && b.is_string() {
            let a_str = a.as_str().unwrap();
            let b_str = b.as_str().unwrap();
            if case_sensitive {
                a_str.ends_with(b_str)
            } else {
                a_str.to_lowercase().ends_with(&b_str.to_lowercase())
            }
        } else {
            false
        }
    }

    fn compare_numeric(&self, a: &Value, b: &Value, cmp: fn(f64, f64) -> bool) -> bool {
        let a_num = a
            .as_f64()
            .or_else(|| a.as_i64().map(|v| v as f64))
            .or_else(|| a.as_u64().map(|v| v as f64));
        let b_num = b
            .as_f64()
            .or_else(|| b.as_i64().map(|v| v as f64))
            .or_else(|| b.as_u64().map(|v| v as f64));

        match (a_num, b_num) {
            (Some(a), Some(b)) => cmp(a, b),
            _ => false,
        }
    }

    fn values_in(&self, field_value: &Value, rule_value: &Value) -> bool {
        if let Some(arr) = rule_value.as_array() {
            if field_value.is_array() {
                let field_arr = field_value.as_array().unwrap();
                arr.iter().any(|v| field_arr.contains(v))
            } else {
                arr.contains(field_value)
            }
        } else {
            false
        }
    }

    fn values_between(&self, field_value: &Value, rule_value: &Value) -> bool {
        if let Some(arr) = rule_value.as_array() {
            if arr.len() == 2 {
                let min = arr[0].as_f64().unwrap_or(f64::NEG_INFINITY);
                let max = arr[1].as_f64().unwrap_or(f64::INFINITY);
                let field_num = field_value
                    .as_f64()
                    .or_else(|| field_value.as_i64().map(|v| v as f64))
                    .or_else(|| field_value.as_u64().map(|v| v as f64));
                field_num.map_or(false, |n| n >= min && n <= max)
            } else {
                false
            }
        } else {
            false
        }
    }

    fn values_within_days(&self, field_value: &Value, rule_value: &Value) -> bool {
        if let Some(days) = rule_value.as_i64() {
            if let Some(date_str) = field_value.as_str() {
                if let Ok(date) = chrono::DateTime::parse_from_rfc3339(date_str) {
                    let now = chrono::Utc::now();
                    let diff = now.signed_duration_since(date.with_timezone(&chrono::Utc));
                    return diff.num_days().abs() <= days;
                }
            }
        }
        false
    }

    fn values_within_weeks(&self, field_value: &Value, rule_value: &Value) -> bool {
        if let Some(weeks) = rule_value.as_i64() {
            if let Some(date_str) = field_value.as_str() {
                if let Ok(date) = chrono::DateTime::parse_from_rfc3339(date_str) {
                    let now = chrono::Utc::now();
                    let diff = now.signed_duration_since(date.with_timezone(&chrono::Utc));
                    return diff.num_weeks().abs() <= weeks;
                }
            }
        }
        false
    }

    fn values_within_months(&self, field_value: &Value, rule_value: &Value) -> bool {
        if let Some(months) = rule_value.as_i64() {
            if let Some(date_str) = field_value.as_str() {
                if let Ok(date) = chrono::DateTime::parse_from_rfc3339(date_str) {
                    let now = chrono::Utc::now();
                    let diff = now.signed_duration_since(date.with_timezone(&chrono::Utc));
                    return (diff.num_days() / 30).abs() <= months;
                }
            }
        }
        false
    }

    fn values_regex(&self, field_value: &Value, rule_value: &Value) -> bool {
        if let (Some(pattern), Some(text)) = (rule_value.as_str(), field_value.as_str()) {
            regex::Regex::new(pattern).map_or(false, |re| re.is_match(text))
        } else {
            false
        }
    }
}

/// Media item for search bin matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    pub id: Uuid,
    pub name: String,
    pub path: PathBuf,
    pub media_type: MediaType,
    pub file_size: u64,
    pub duration: Option<Ticks>,
    pub frame_rate: Option<FrameRate>,
    pub width: u32,
    pub height: u32,
    pub codec: String,
    pub audio_channels: u32,
    pub sample_rate: u32,
    pub bit_depth: u32,
    pub date_created: chrono::DateTime<chrono::Utc>,
    pub date_modified: chrono::DateTime<chrono::Utc>,
    pub date_imported: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
    pub rating: u8,
    pub color_label: Option<String>,
    pub metadata: HashMap<String, Value>,
}

impl MediaItem {
    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    }

    pub fn extension(&self) -> String {
        self.path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    Video,
    Audio,
    Image,
    Subtitle,
    Project,
    Sequence,
    GraphicsTemplate,
    Other,
}

/// Search bin match result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchBinMatch {
    pub item_id: Uuid,
    pub item_name: String,
    pub item_path: PathBuf,
    pub matched_fields: Vec<String>,
    pub score: f64,
}

/// Search bin manager
pub struct SearchBinManager {
    bins: IndexMap<Uuid, SearchBin>,
    media_items: IndexMap<Uuid, MediaItem>,
}

impl SearchBinManager {
    pub fn new() -> Self {
        Self {
            bins: IndexMap::new(),
            media_items: IndexMap::new(),
        }
    }

    pub fn add_bin(&mut self, bin: SearchBin) {
        self.bins.insert(bin.id, bin);
    }

    pub fn remove_bin(&mut self, bin_id: Uuid) -> Option<SearchBin> {
        self.bins.shift_remove(&bin_id)
    }

    pub fn get_bin(&self, bin_id: Uuid) -> Option<&SearchBin> {
        self.bins.get(&bin_id)
    }

    pub fn get_bin_mut(&mut self, bin_id: Uuid) -> Option<&mut SearchBin> {
        self.bins.get_mut(&bin_id)
    }

    pub fn list_bins(&self) -> Vec<&SearchBin> {
        self.bins.values().collect()
    }

    pub fn add_media_item(&mut self, item: MediaItem) {
        self.media_items.insert(item.id, item);
    }

    pub fn remove_media_item(&mut self, item_id: Uuid) -> Option<MediaItem> {
        self.media_items.shift_remove(&item_id)
    }

    pub fn update_media_item(&mut self, item: MediaItem) {
        self.media_items.insert(item.id, item);
    }

    pub fn get_media_item(&self, item_id: Uuid) -> Option<&MediaItem> {
        self.media_items.get(&item_id)
    }

    /// Update all dynamic search bins
    pub fn update_all(&mut self) {
        let items: Vec<&MediaItem> = self.media_items.values().collect();
        for bin in self.bins.values_mut() {
            if bin.is_dynamic {
                bin.matched_items.clear();
                for item in &items {
                    if bin.matches(item) {
                        bin.matched_items.push(SearchBinMatch {
                            item_id: item.id,
                            item_name: item.name.clone(),
                            item_path: item.path.clone(),
                            matched_fields: Vec::new(), // Could be populated with actual matched fields
                            score: 1.0,
                        });
                    }
                }
                bin.match_count = bin.matched_items.len();
                bin.modified_at = chrono::Utc::now();
            }
        }
    }

    /// Get matches for a bin
    pub fn get_matches(&self, bin_id: Uuid) -> Option<&Vec<SearchBinMatch>> {
        self.bins.get(&bin_id).map(|b| &b.matched_items)
    }

    /// Execute query and return matches
    pub fn execute_query(&self, query: &SearchBinQuery) -> Vec<SearchBinMatch> {
        let items: Vec<&MediaItem> = self.media_items.values().collect();
        let mut matches = Vec::new();

        for item in items {
            if self.evaluate_query(query, item) {
                matches.push(SearchBinMatch {
                    item_id: item.id,
                    item_name: item.name.clone(),
                    item_path: item.path.clone(),
                    matched_fields: Vec::new(),
                    score: 1.0,
                });
            }
        }

        // Apply sorting
        if let Some(_sort_field) = query.sort_by {
            matches.sort_by(|a, b| {
                let order = if query.sort_order == SortOrder::Ascending {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                };
                // Simplified sorting - would need actual field values
                a.item_name.cmp(&b.item_name).then(order)
            });
        }

        // Apply limit
        if let Some(limit) = query.limit {
            matches.truncate(limit);
        }

        matches
    }

    fn evaluate_query(&self, query: &SearchBinQuery, item: &MediaItem) -> bool {
        let bin = SearchBin::new("temp".to_string(), query.clone());
        bin.matches(item)
    }
}

impl Default for SearchBinManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Smart search bin presets
pub mod presets {
    use super::*;

    pub fn recently_added(days: i64) -> SearchBin {
        SearchBin::new(
            "Recently Added".to_string(),
            SearchBinQuery {
                rules: vec![SearchBinRule {
                    field: SearchField::DateImported,
                    operator: SearchOperator::WithinDays,
                    value: json!(days),
                    case_sensitive: false,
                }],
                logic: SearchLogic::And,
                limit: Some(50),
                sort_by: Some(SortField::DateImported),
                sort_order: SortOrder::Descending,
            },
        )
        .with_color("#4ECDC4".to_string())
    }

    pub fn video_files() -> SearchBin {
        SearchBin::new(
            "Video Files".to_string(),
            SearchBinQuery {
                rules: vec![SearchBinRule {
                    field: SearchField::MediaType,
                    operator: SearchOperator::Equals,
                    value: json!("Video"),
                    case_sensitive: false,
                }],
                logic: SearchLogic::And,
                limit: None,
                sort_by: Some(SortField::Name),
                sort_order: SortOrder::Ascending,
            },
        )
        .with_color("#FF6B6B".to_string())
    }

    pub fn audio_files() -> SearchBin {
        SearchBin::new(
            "Audio Files".to_string(),
            SearchBinQuery {
                rules: vec![SearchBinRule {
                    field: SearchField::MediaType,
                    operator: SearchOperator::Equals,
                    value: json!("Audio"),
                    case_sensitive: false,
                }],
                logic: SearchLogic::And,
                limit: None,
                sort_by: Some(SortField::Name),
                sort_order: SortOrder::Ascending,
            },
        )
        .with_color("#4ECDC4".to_string())
    }

    pub fn high_rated(min_rating: u8) -> SearchBin {
        SearchBin::new(
            format!("Rated {}+ Stars", min_rating),
            SearchBinQuery {
                rules: vec![SearchBinRule {
                    field: SearchField::Rating,
                    operator: SearchOperator::GreaterThanOrEqual,
                    value: json!(min_rating),
                    case_sensitive: false,
                }],
                logic: SearchLogic::And,
                limit: None,
                sort_by: Some(SortField::Rating),
                sort_order: SortOrder::Descending,
            },
        )
        .with_color("#FFBE0B".to_string())
    }

    pub fn large_files(min_size_mb: u64) -> SearchBin {
        SearchBin::new(
            format!("Files > {} MB", min_size_mb),
            SearchBinQuery {
                rules: vec![SearchBinRule {
                    field: SearchField::FileSize,
                    operator: SearchOperator::GreaterThan,
                    value: json!(min_size_mb * 1024 * 1024),
                    case_sensitive: false,
                }],
                logic: SearchLogic::And,
                limit: None,
                sort_by: Some(SortField::FileSize),
                sort_order: SortOrder::Descending,
            },
        )
        .with_color("#FF6B6B".to_string())
    }

    pub fn missing_metadata() -> SearchBin {
        SearchBin::new(
            "Missing Metadata".to_string(),
            SearchBinQuery {
                rules: vec![SearchBinRule {
                    field: SearchField::Tags,
                    operator: SearchOperator::IsEmpty,
                    value: json!(null),
                    case_sensitive: false,
                }],
                logic: SearchLogic::Or,
                limit: None,
                sort_by: Some(SortField::DateModified),
                sort_order: SortOrder::Descending,
            },
        )
        .with_color("#96CEB4".to_string())
    }

    pub fn proxy_files() -> SearchBin {
        SearchBin::new(
            "Proxy Files".to_string(),
            SearchBinQuery {
                rules: vec![SearchBinRule {
                    field: SearchField::Name,
                    operator: SearchOperator::Contains,
                    value: json!("proxy"),
                    case_sensitive: false,
                }],
                logic: SearchLogic::And,
                limit: None,
                sort_by: Some(SortField::Name),
                sort_order: SortOrder::Ascending,
            },
        )
        .with_color("#45B7D1".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use saturn_timeline::TICKS_PER_SECOND;

    #[test]
    fn test_search_bin_creation() {
        let bin = SearchBin::new("Test Bin".to_string(), SearchBinQuery::default());
        assert_eq!(bin.name, "Test Bin");
    }

    #[test]
    fn test_search_bin_matching() {
        let item = MediaItem {
            id: Uuid::new_v4(),
            name: "test_video.mp4".to_string(),
            path: PathBuf::from("/media/test_video.mp4"),
            media_type: MediaType::Video,
            file_size: 100_000_000,
            duration: Some(TICKS_PER_SECOND * 10),
            frame_rate: Some(FrameRate::Fps30),
            width: 1920,
            height: 1080,
            codec: "h264".to_string(),
            audio_channels: 2,
            sample_rate: 48000,
            bit_depth: 8,
            date_created: chrono::Utc::now(),
            date_modified: chrono::Utc::now(),
            date_imported: chrono::Utc::now(),
            tags: vec!["test".to_string()],
            rating: 5,
            color_label: Some("red".to_string()),
            metadata: HashMap::new(),
        };

        let bin = SearchBin::new(
            "Video Files".to_string(),
            SearchBinQuery {
                rules: vec![SearchBinRule {
                    field: SearchField::MediaType,
                    operator: SearchOperator::Equals,
                    value: json!("Video"),
                    case_sensitive: false,
                }],
                logic: SearchLogic::And,
                limit: None,
                sort_by: None,
                sort_order: SortOrder::Ascending,
            },
        );

        assert!(bin.matches(&item));
    }

    #[test]
    fn test_search_bin_presets() {
        let bin = presets::recently_added(7);
        assert_eq!(bin.name, "Recently Added");
        assert!(bin.query.rules.len() == 1);
    }

    #[test]
    fn test_search_bin_manager() {
        let mut manager = SearchBinManager::new();
        let bin = SearchBin::new("Test".to_string(), SearchBinQuery::default());
        let bin_id = bin.id;
        manager.add_bin(bin);

        let item = MediaItem {
            id: Uuid::new_v4(),
            name: "test.mp4".to_string(),
            path: PathBuf::from("/test.mp4"),
            media_type: MediaType::Video,
            file_size: 1000,
            duration: Some(TICKS_PER_SECOND),
            frame_rate: Some(FrameRate::Fps30),
            width: 1920,
            height: 1080,
            codec: "h264".to_string(),
            audio_channels: 2,
            sample_rate: 48000,
            bit_depth: 8,
            date_created: chrono::Utc::now(),
            date_modified: chrono::Utc::now(),
            date_imported: chrono::Utc::now(),
            tags: vec![],
            rating: 0,
            color_label: None,
            metadata: HashMap::new(),
        };
        manager.add_media_item(item);
        manager.update_all();

        assert!(manager.get_bin(bin_id).unwrap().match_count > 0);
    }
}
