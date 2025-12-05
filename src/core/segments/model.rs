use super::{Segment, SegmentData};
use crate::config::{InputData, ModelConfig, SegmentId};
use std::collections::HashMap;

/// ANSI 颜色代码
const GOLD: &str = "\x1b[38;5;220m";
const RESET: &str = "\x1b[0m";

#[derive(Default)]
pub struct ModelSegment;

impl ModelSegment {
    pub fn new() -> Self {
        Self
    }
}

impl Segment for ModelSegment {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        let mut metadata = HashMap::new();
        metadata.insert("model_id".to_string(), input.model.id.clone());
        metadata.insert("display_name".to_string(), input.model.display_name.clone());

        let model_name = self.format_model_name(&input.model.id, &input.model.display_name);

        // 尝试获取 Cubence 倍率并附加到模型名后面
        let multiplier_suffix = self.get_cubence_multiplier();

        let primary = if let Some(mult) = multiplier_suffix {
            metadata.insert("has_ansi_colors".to_string(), "true".to_string());
            format!("{}{}(x{}){}", model_name, GOLD, mult, RESET)
        } else {
            model_name
        };

        Some(SegmentData {
            primary,
            secondary: String::new(),
            metadata,
        })
    }

    fn id(&self) -> SegmentId {
        SegmentId::Model
    }
}

impl ModelSegment {
    fn format_model_name(&self, id: &str, display_name: &str) -> String {
        let model_config = ModelConfig::load();

        // Try to get display name from external config first
        if let Some(config_name) = model_config.get_display_name(id) {
            config_name
        } else {
            // Fallback to Claude Code's official display_name for unrecognized models
            display_name.to_string()
        }
    }

    /// 获取 Cubence 倍率（如果是 Cubence 服务商）
    fn get_cubence_multiplier(&self) -> Option<f64> {
        use crate::api::VendorType;

        // 检查是否是 Cubence 服务商
        let vendor = crate::api::detect_vendor_from_claude_settings();
        if vendor != VendorType::Cubence {
            return None;
        }

        // 尝试从 cubence_multiplier 模块获取倍率
        super::cubence_multiplier::get_multiplier()
    }
}
