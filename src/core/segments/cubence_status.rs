//! Cubence 标识段
//! 显示 Cubence 服务商标识 (🦢 Cubence)

use crate::api::VendorType;
use crate::config::Config;
use crate::config::InputData;
use crate::core::segments::SegmentData;
use std::collections::HashMap;

/// 收集 Cubence 标识数据
pub fn collect(config: &Config, _input: &InputData) -> Option<SegmentData> {
    let segment = config
        .segments
        .iter()
        .find(|s| matches!(s.id, crate::config::SegmentId::CubenceStatus))?;

    if !segment.enabled {
        return None;
    }

    // 检查是否是 Cubence 服务商，不是则静默跳过
    let vendor = crate::api::detect_vendor_from_claude_settings();
    if vendor != VendorType::Cubence {
        return None;
    }

    let mut metadata = HashMap::new();
    metadata.insert("service".to_string(), "cubence".to_string());

    // 简单显示 Cubence 标识
    Some(SegmentData {
        primary: "Cubence".to_string(),
        secondary: String::new(),
        metadata,
    })
}
