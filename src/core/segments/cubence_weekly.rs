//! Cubence 周窗口段
//! 显示周滚动窗口的用量和重置时间（带进度条）

use crate::api::{cache, client::ApiClient, ApiConfig, CubenceData, VendorType};
use crate::config::Config;
use crate::config::InputData;
use crate::core::segments::SegmentData;
use std::collections::HashMap;

/// 收集 Cubence 周窗口数据
pub fn collect(config: &Config, _input: &InputData) -> Option<SegmentData> {
    let segment = config
        .segments
        .iter()
        .find(|s| matches!(s.id, crate::config::SegmentId::CubenceWeekly))?;

    if !segment.enabled {
        return None;
    }

    // 检查是否是 Cubence 服务商，不是则静默跳过
    let vendor = crate::api::detect_vendor_from_claude_settings();
    if vendor != VendorType::Cubence {
        return None;
    }

    // 获取 API key
    let api_key = segment
        .options
        .get("api_key")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(crate::api::get_cubence_api_key_from_claude_settings);

    let api_key = match api_key {
        Some(key) if !key.is_empty() => key,
        _ => {
            return Some(SegmentData {
                primary: "未配置".to_string(),
                secondary: String::new(),
                metadata: HashMap::new(),
            });
        }
    };

    // 获取 API URL
    let usage_url = segment
        .options
        .get("usage_url")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "https://cubence.com/api/v1/user/subscription-info".to_string());

    // 获取数据
    let cubence_data = fetch_or_cache(&api_key, &usage_url)?;

    // 构建显示数据
    build_segment_data(&cubence_data)
}

fn fetch_or_cache(api_key: &str, usage_url: &str) -> Option<CubenceData> {
    let (cached, _) = cache::get_cached_cubence_usage();

    if let Some(mut fresh) = fetch_cubence_sync(api_key, usage_url) {
        fresh.calculate();
        let _ = cache::save_cached_cubence_usage(&fresh);
        Some(fresh)
    } else if let Some(mut cached_data) = cached {
        cached_data.calculate();
        Some(cached_data)
    } else {
        None
    }
}

fn fetch_cubence_sync(api_key: &str, usage_url: &str) -> Option<CubenceData> {
    let api_config = ApiConfig {
        enabled: true,
        api_key: api_key.to_string(),
        usage_url: usage_url.to_string(),
        subscription_url: String::new(),
        auto_cookie: true, // Cubence 需要 Cookie
        cookie: None,
    };

    let client = ApiClient::new(api_config).ok()?;
    let usage = client.get_usage().ok()?;
    usage.as_cubence().cloned()
}

/// 生成进度条
fn make_progress_bar(percentage: f64, width: usize) -> String {
    let filled = ((percentage / 100.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);

    // 使用 Unicode 方块字符
    let filled_char = '█';
    let empty_char = '░';

    format!(
        "{}{}",
        filled_char.to_string().repeat(filled),
        empty_char.to_string().repeat(empty)
    )
}

/// 格式化持续时间
fn format_duration(seconds: i64) -> String {
    if seconds <= 0 {
        return "即将重置".to_string();
    }

    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;

    if hours > 24 {
        let days = hours / 24;
        let remaining_hours = hours % 24;
        format!("{}d{}h", days, remaining_hours)
    } else if hours > 0 {
        format!("{}h{}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

fn build_segment_data(data: &CubenceData) -> Option<SegmentData> {
    let mut metadata = HashMap::new();

    // 存储原始数据
    metadata.insert("weekly_used".to_string(), data.weekly_used.to_string());
    metadata.insert("weekly_limit".to_string(), data.weekly_limit.to_string());
    metadata.insert("weekly_remaining".to_string(), data.weekly_remaining.to_string());
    metadata.insert("weekly_percentage".to_string(), format!("{:.1}", data.weekly_percentage));
    metadata.insert("service".to_string(), "cubence".to_string());

    // 格式化显示
    let used_fmt = CubenceData::format_tokens(data.weekly_used);
    let limit_fmt = CubenceData::format_tokens(data.weekly_limit);
    let reset_str = format_duration(data.get_weekly_reset_seconds());
    let progress_bar = make_progress_bar(data.weekly_percentage, 8);

    // 主显示：周 [进度条] 121M/200M (3d5h)
    let primary = format!(
        "周 {} {}/{} ({})",
        progress_bar,
        used_fmt,
        limit_fmt,
        reset_str
    );

    Some(SegmentData {
        primary,
        secondary: String::new(),
        metadata,
    })
}
