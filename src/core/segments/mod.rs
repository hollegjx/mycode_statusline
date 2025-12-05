pub mod context_window;
pub mod cost;
pub mod cubence_balance;
pub mod cubence_five_hour;
pub mod cubence_load_status;
pub mod cubence_status;
pub mod cubence_usage;
pub mod cubence_weekly;
pub mod directory;
pub mod git;
pub mod model;
pub mod output_style;
pub mod session;
pub mod update;
pub mod usage;
pub mod uucode_status;
pub mod uucode_subscription;
pub mod uucode_usage;

use crate::config::{InputData, SegmentId};
use std::collections::HashMap;

// New Segment trait for data collection only
pub trait Segment {
    fn collect(&self, input: &InputData) -> Option<SegmentData>;
    fn id(&self) -> SegmentId;
}

#[derive(Debug, Clone)]
pub struct SegmentData {
    pub primary: String,
    pub secondary: String,
    pub metadata: HashMap<String, String>,
}

// Re-export all segment types
pub use context_window::ContextWindowSegment;
pub use cost::CostSegment;
pub use directory::DirectorySegment;
pub use git::GitSegment;
pub use model::ModelSegment;
pub use output_style::OutputStyleSegment;
pub use session::SessionSegment;
pub use update::UpdateSegment;
pub use usage::UsageSegment;
