//! Cubence 订阅段
//! 显示当前订阅计划和剩余时间
//! Cookie 通过 ~/.claude/mycode/cache/cubence/cookie.json 手动配置

use crate::api::VendorType;
use crate::config::Config;
use crate::config::InputData;
use crate::core::segments::SegmentData;
use chrono::{DateTime, FixedOffset, Utc};
use reqwest::blocking::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// 订阅 API 端点
const SUBSCRIPTION_URL: &str = "https://cubence.com/api/v1/subscription/current";

/// Cookie 配置文件结构
#[derive(Debug, Deserialize)]
struct CookieConfig {
    cookie: String,
}

/// 订阅 API 响应
#[derive(Debug, Deserialize)]
struct SubscriptionResponse {
    subscription: SubscriptionInfo,
}

#[derive(Debug, Deserialize)]
struct SubscriptionInfo {
    status: String,
    end_date: String,
    plan: PlanInfo,
}

#[derive(Debug, Deserialize)]
struct PlanInfo {
    name: String,
}

/// 获取 cookie 配置文件路径
fn get_cookie_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| {
        home.join(".claude")
            .join("mycode")
            .join("cache")
            .join("cubence")
            .join("cookie.json")
    })
}

/// 读取 token
fn read_token() -> Option<String> {
    let path = get_cookie_config_path()?;

    if !path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&path).ok()?;
    let config: CookieConfig = serde_json::from_str(&content).ok()?;

    if config.cookie.trim().is_empty() {
        None
    } else {
        // 从 cookie 字符串中提取 token 值
        // 格式可能是 "token=xxx" 或直接是 token
        let cookie = config.cookie.trim();
        if cookie.starts_with("token=") {
            Some(cookie.strip_prefix("token=").unwrap().to_string())
        } else {
            Some(cookie.to_string())
        }
    }
}

/// 请求订阅信息
fn fetch_subscription(token: &str) -> Result<SubscriptionResponse, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(SUBSCRIPTION_URL)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let text = response.text().map_err(|e| e.to_string())?;

    // 检查是否是错误响应
    if text.contains("No token provided") || text.contains("error") {
        return Err("Cookie 无效".to_string());
    }

    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// 计算剩余时间
fn calculate_remaining(end_date: &str) -> Option<(i64, i64)> {
    // 解析 ISO 8601 日期
    let end: DateTime<FixedOffset> = DateTime::parse_from_rfc3339(end_date).ok()?;
    let now = Utc::now();
    let duration = end.signed_duration_since(now);

    if duration.num_seconds() <= 0 {
        return Some((0, 0));
    }

    let days = duration.num_days();
    let hours = (duration.num_hours() % 24).abs();

    Some((days, hours))
}

/// 收集 Cubence 订阅数据
pub fn collect(config: &Config, _input: &InputData) -> Option<SegmentData> {
    let segment = config
        .segments
        .iter()
        .find(|s| matches!(s.id, crate::config::SegmentId::CubenceSubscription))?;

    if !segment.enabled {
        return None;
    }

    // 检查是否是 Cubence 服务商
    let vendor = crate::api::detect_vendor_from_claude_settings();
    if vendor != VendorType::Cubence {
        return None;
    }

    // 读取 token，没有则不显示此段
    let token = read_token()?;

    // 请求订阅信息
    let subscription = fetch_subscription(&token).ok()?;

    // 检查订阅状态
    if subscription.subscription.status != "active" {
        return None;
    }

    let mut metadata = HashMap::new();
    metadata.insert(
        "plan_name".to_string(),
        subscription.subscription.plan.name.clone(),
    );
    metadata.insert(
        "end_date".to_string(),
        subscription.subscription.end_date.clone(),
    );
    metadata.insert(
        "status".to_string(),
        subscription.subscription.status.clone(),
    );

    // 计算剩余时间
    let (days, hours) = calculate_remaining(&subscription.subscription.end_date)?;

    let plan_name = subscription.subscription.plan.name;

    // 格式: 💎 Prism-剩余5d 12h
    let primary = format!("💎 {}-剩余{}d {}h", plan_name, days, hours);

    Some(SegmentData {
        primary,
        secondary: String::new(),
        metadata,
    })
}
