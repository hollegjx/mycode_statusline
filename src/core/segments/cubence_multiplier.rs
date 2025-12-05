//! Cubence 价格倍率段
//! 显示当前服务的价格倍率 (all * claude_code)
//! 需要 Cookie 认证，通过 ~/.claude/mycode/cache/cubence/cookie.json 配置

use crate::config::Config;
use crate::config::InputData;
use crate::core::segments::SegmentData;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

/// 失败计数器（最多重试3次）
static FAILURE_COUNT: AtomicU8 = AtomicU8::new(0);
const MAX_FAILURES: u8 = 3;

/// Dashboard Overview API 端点
const OVERVIEW_URL: &str = "https://cubence.com/api/v1/dashboard/overview";

/// Cookie 配置文件结构
#[derive(Debug, Deserialize)]
struct CookieConfig {
    cookie: String,
}

/// Dashboard Overview API 响应 (只解析需要的部分)
#[derive(Debug, Deserialize)]
struct OverviewResponse {
    data: OverviewData,
}

#[derive(Debug, Deserialize)]
struct OverviewData {
    pricing_multiplier: PricingMultiplier,
}

#[derive(Debug, Deserialize)]
struct PricingMultiplier {
    by_service: ByService,
}

#[derive(Debug, Deserialize)]
struct ByService {
    all: ServiceMultiplier,
    claude_code: ServiceMultiplier,
}

#[derive(Debug, Deserialize)]
struct ServiceMultiplier {
    multiplier: f64,
    is_active: bool,
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

/// 读取 token 配置
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

/// 请求价格倍率
fn fetch_multiplier(token: &str) -> Result<f64, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(OVERVIEW_URL)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let text = response.text().map_err(|e| e.to_string())?;

    // 检查是否是错误响应
    if text.contains("No token provided") || text.contains("\"error\"") {
        return Err("Cookie 无效".to_string());
    }

    let resp: OverviewResponse = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    // 计算倍率: all.multiplier * claude_code.multiplier
    // 只有当 is_active 为 true 时才使用该倍率，否则视为 1.0
    let all_mult = if resp.data.pricing_multiplier.by_service.all.is_active {
        resp.data.pricing_multiplier.by_service.all.multiplier
    } else {
        1.0
    };

    let claude_code_mult = if resp.data.pricing_multiplier.by_service.claude_code.is_active {
        resp.data.pricing_multiplier.by_service.claude_code.multiplier
    } else {
        1.0
    };

    Ok(all_mult * claude_code_mult)
}

/// 收集 Cubence 价格倍率数据（已废弃，倍率现在直接显示在模型名后面）
pub fn collect(_config: &Config, _input: &InputData) -> Option<SegmentData> {
    // 倍率现在由模型段直接显示，此处返回 None
    None
}

/// 获取 Cubence 倍率（供模型段调用）
pub fn get_multiplier() -> Option<f64> {
    // 检查失败次数，超过3次则不再请求
    let failures = FAILURE_COUNT.load(Ordering::Relaxed);
    if failures >= MAX_FAILURES {
        return None;
    }

    // 读取 token
    let token = read_token()?;

    // 请求倍率
    match fetch_multiplier(&token) {
        Ok(multiplier) => {
            // 成功，重置失败计数
            FAILURE_COUNT.store(0, Ordering::Relaxed);
            Some(multiplier)
        }
        Err(_) => {
            // 失败，增加计数
            FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
            None
        }
    }
}
