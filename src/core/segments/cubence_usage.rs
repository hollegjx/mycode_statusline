//! Cubence 用量段
//! 显示 5小时窗口 + 周窗口的使用情况

use crate::api::{cache, client::ApiClient, ApiConfig, CubenceData, VendorType};
use crate::config::Config;
use crate::config::InputData;
use crate::core::segments::SegmentData;
use std::collections::HashMap;

/// 收集 Cubence 用量数据（5小时窗口 + 周窗口）
pub fn collect(config: &Config, _input: &InputData) -> Option<SegmentData> {
    let segment = config
        .segments
        .iter()
        .find(|s| matches!(s.id, crate::config::SegmentId::CubenceUsage))?;

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
                primary: "未配置密钥".to_string(),
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

fn build_segment_data(data: &CubenceData) -> Option<SegmentData> {
    let mut metadata = HashMap::new();

    // 存储原始数据
    metadata.insert("five_hour_used".to_string(), data.five_hour_used.to_string());
    metadata.insert("five_hour_limit".to_string(), data.five_hour_limit.to_string());
    metadata.insert("five_hour_remaining".to_string(), data.five_hour_remaining.to_string());
    metadata.insert("five_hour_percentage".to_string(), format!("{:.1}", data.five_hour_percentage));
    metadata.insert("weekly_used".to_string(), data.weekly_used.to_string());
    metadata.insert("weekly_limit".to_string(), data.weekly_limit.to_string());
    metadata.insert("weekly_remaining".to_string(), data.weekly_remaining.to_string());
    metadata.insert("weekly_percentage".to_string(), format!("{:.1}", data.weekly_percentage));
    metadata.insert("service".to_string(), "cubence".to_string());

    // 格式化显示
    let five_hour_used_fmt = CubenceData::format_tokens(data.five_hour_used);
    let five_hour_limit_fmt = CubenceData::format_tokens(data.five_hour_limit);
    let weekly_used_fmt = CubenceData::format_tokens(data.weekly_used);
    let weekly_limit_fmt = CubenceData::format_tokens(data.weekly_limit);

    // 计算重置时间
    let five_hour_reset_str = format_duration(data.get_five_hour_reset_seconds());
    let weekly_reset_str = format_duration(data.get_weekly_reset_seconds());

    // 主显示：5小时窗口
    let primary = format!(
        "⏱ {}/{} ({:.0}%)",
        five_hour_used_fmt, five_hour_limit_fmt, data.five_hour_percentage
    );

    // 次要显示：周窗口 + 重置时间
    let secondary = format!(
        "📅 {}/{} ({:.0}%) | 5h重置: {} | 周重置: {}",
        weekly_used_fmt,
        weekly_limit_fmt,
        data.weekly_percentage,
        five_hour_reset_str,
        weekly_reset_str
    );

    Some(SegmentData {
        primary,
        secondary,
        metadata,
    })
}

/// 格式化持续时间
fn format_duration(seconds: i64) -> String {
    if seconds <= 0 {
        return "已到期".to_string();
    }

    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;

    if hours > 24 {
        let days = hours / 24;
        let remaining_hours = hours % 24;
        format!("{}天{}h", days, remaining_hours)
    } else if hours > 0 {
        format!("{}h{}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}
