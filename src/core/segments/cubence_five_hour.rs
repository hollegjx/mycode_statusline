//! Cubence 5小时窗口段
//! 显示 5小时滚动窗口的用量和重置时间（带进度条）

use crate::api::{cache, client::ApiClient, ApiConfig, CubenceData, VendorType};
use crate::config::Config;
use crate::config::InputData;
use crate::core::segments::SegmentData;
use std::collections::HashMap;

/// 收集 Cubence 5小时窗口数据
pub fn collect(config: &Config, _input: &InputData) -> Option<SegmentData> {
    let segment = config
        .segments
        .iter()
        .find(|s| matches!(s.id, crate::config::SegmentId::CubenceFiveHour))?;

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

    if hours > 0 {
        format!("{}h{}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

/// ANSI 颜色代码
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";

fn build_segment_data(data: &CubenceData) -> Option<SegmentData> {
    let mut metadata = HashMap::new();

    // 存储原始数据
    metadata.insert(
        "five_hour_used".to_string(),
        data.five_hour_used.to_string(),
    );
    metadata.insert(
        "five_hour_limit".to_string(),
        data.five_hour_limit.to_string(),
    );
    metadata.insert(
        "five_hour_remaining".to_string(),
        data.five_hour_remaining.to_string(),
    );
    metadata.insert(
        "five_hour_percentage".to_string(),
        format!("{:.1}", data.five_hour_percentage),
    );
    metadata.insert("service".to_string(), "cubence".to_string());

    // 格式化显示
    let used_fmt = CubenceData::format_tokens(data.five_hour_used);
    let limit_fmt = CubenceData::format_tokens(data.five_hour_limit);
    let reset_str = format_duration(data.get_five_hour_reset_seconds());
    let progress_bar = make_progress_bar(data.five_hour_percentage, 8);

    // 主显示：5h [进度条(绿色)] 数字(黄色) (重置时间)
    // 格式: 5h ████░░░░ $36.1/$80.0 (1h6m)
    let primary = format!(
        "5h {}{}{} {}{}/{}{} ({})",
        GREEN, progress_bar, RESET, YELLOW, used_fmt, limit_fmt, RESET, reset_str
    );

    Some(SegmentData {
        primary,
        secondary: String::new(),
        metadata,
    })
}
