//! Cubence 账户余额段
//! 显示 Cubence 账户的美元余额

use crate::api::{cache, client::ApiClient, ApiConfig, CubenceData, VendorType};
use crate::config::Config;
use crate::config::InputData;
use crate::core::segments::SegmentData;
use std::collections::HashMap;

/// 收集 Cubence 余额数据
pub fn collect(config: &Config, _input: &InputData) -> Option<SegmentData> {
    let segment = config
        .segments
        .iter()
        .find(|s| matches!(s.id, crate::config::SegmentId::CubenceBalance))?;

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

    // 构建显示
    let mut metadata = HashMap::new();
    metadata.insert(
        "balance_usd".to_string(),
        format!("{:.2}", cubence_data.balance_usd),
    );
    metadata.insert("service".to_string(), "cubence".to_string());

    // 金色/黄色 ANSI 代码
    const GOLD: &str = "\x1b[38;5;220m";
    const RESET: &str = "\x1b[0m";

    Some(SegmentData {
        primary: format!("{}${:.2}{}", GOLD, cubence_data.balance_usd, RESET),
        secondary: String::new(),
        metadata,
    })
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
