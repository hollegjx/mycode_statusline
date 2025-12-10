//! Cubence 延迟检测段
//! 显示 API 延迟信息，根据 base_url 自动选择对应的 health 端点

use crate::api::VendorType;
use crate::config::Config;
use crate::config::InputData;
use crate::core::segments::SegmentData;
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Cubence 线路配置
const CUBENCE_ENDPOINTS: &[(&str, &str)] = &[
    (
        "api-dmit.cubence.com",
        "https://api-dmit.cubence.com/health",
    ),
    ("api-bwg.cubence.com", "https://api-bwg.cubence.com/health"),
    ("api-cf.cubence.com", "https://api-cf.cubence.com/health"),
    ("api.cubence.com", "https://api.cubence.com/health"),
];

/// 根据 base_url 获取对应的 health 端点
fn get_health_url_from_base(base_url: &str) -> Option<&'static str> {
    for (pattern, health_url) in CUBENCE_ENDPOINTS {
        if base_url.contains(pattern) {
            return Some(health_url);
        }
    }
    None
}

/// 测量 API 延迟
fn measure_latency(health_url: &str) -> Result<u128, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let start = Instant::now();
    let response = client.get(health_url).send().map_err(|e| e.to_string())?;
    let latency_ms = start.elapsed().as_millis();

    if response.status().is_success() {
        Ok(latency_ms)
    } else {
        Err(format!("HTTP {}", response.status()))
    }
}

/// 根据延迟返回对应的 emoji
fn get_latency_emoji(latency_ms: u128) -> &'static str {
    if latency_ms <= 300 {
        "🟢" // 绿色：300ms 以内
    } else if latency_ms <= 1000 {
        "🟡" // 黄色：300-1000ms
    } else if latency_ms <= 2000 {
        "🟠" // 橙色：1000-2000ms
    } else {
        "🔴" // 红色：2000ms 以上
    }
}

/// 收集 Cubence 延迟数据
pub fn collect(config: &Config, _input: &InputData) -> Option<SegmentData> {
    let segment = config
        .segments
        .iter()
        .find(|s| matches!(s.id, crate::config::SegmentId::CubenceLatency))?;

    if !segment.enabled {
        return None;
    }

    // 检查是否是 Cubence 服务商
    let vendor = crate::api::detect_vendor_from_claude_settings();
    if vendor != VendorType::Cubence {
        return None;
    }

    // 获取当前 base_url
    let base_url = crate::api::get_current_base_url()?;

    // 根据 base_url 获取对应的 health 端点
    let health_url = get_health_url_from_base(&base_url)?;

    let mut metadata = HashMap::new();
    metadata.insert("health_url".to_string(), health_url.to_string());

    // 测量延迟
    match measure_latency(health_url) {
        Ok(latency_ms) => {
            let emoji = get_latency_emoji(latency_ms);
            metadata.insert("latency_ms".to_string(), latency_ms.to_string());
            metadata.insert("status".to_string(), "ok".to_string());

            Some(SegmentData {
                primary: format!("{}延迟[{}ms]", emoji, latency_ms),
                secondary: String::new(),
                metadata,
            })
        }
        Err(e) => {
            metadata.insert("status".to_string(), "error".to_string());
            metadata.insert("error".to_string(), e.clone());

            Some(SegmentData {
                primary: "🔴延迟[超时]".to_string(),
                secondary: e,
                metadata,
            })
        }
    }
}
