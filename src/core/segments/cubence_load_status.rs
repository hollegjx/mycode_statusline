//! Cubence 负载状态段
//! 显示 Claude Pool 负载状态

use crate::api::VendorType;
use crate::config::Config;
use crate::config::InputData;
use crate::core::segments::SegmentData;
use std::collections::HashMap;

/// 收集 Cubence 负载状态数据
pub fn collect(config: &Config, _input: &InputData) -> Option<SegmentData> {
    let segment = config
        .segments
        .iter()
        .find(|s| matches!(s.id, crate::config::SegmentId::CubenceLoadStatus))?;

    if !segment.enabled {
        return None;
    }

    // 检查是否是 Cubence 服务商，不是则静默跳过
    let vendor = crate::api::detect_vendor_from_claude_settings();
    if vendor != VendorType::Cubence {
        return None;
    }

    let mut metadata = HashMap::new();
    metadata.insert("load_status".to_string(), "normal".to_string());

    // 负载状态显示（暂时显示正常状态）
    Some(SegmentData {
        primary: "正常".to_string(),
        secondary: String::new(),
        metadata,
    })
}
