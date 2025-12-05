//! Cubence 负载状态段
//! 显示 Claude Pool 负载状态
//! Cookie 通过 ~/.claude/mycode/cache/cubence/cookie.json 手动配置

use crate::api::VendorType;
use crate::config::Config;
use crate::config::InputData;
use crate::core::segments::SegmentData;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

/// 失败计数器（最多重试3次）
static FAILURE_COUNT: AtomicU8 = AtomicU8::new(0);
const MAX_FAILURES: u8 = 3;

/// 负载 API 端点
const LOAD_STATUS_URL: &str = "https://cubence.com/api/v1/claudepool/load-status";

/// Cookie 配置文件结构
#[derive(Debug, Deserialize, Serialize)]
struct CookieConfig {
    /// Cookie 值，为空表示未配置
    cookie: String,
    /// 配置说明
    #[serde(default = "default_description")]
    description: String,
}

fn default_description() -> String {
    "请将 Cubence 网站的 Cookie 粘贴到 cookie 字段中".to_string()
}

impl Default for CookieConfig {
    fn default() -> Self {
        Self {
            cookie: String::new(),
            description: default_description(),
        }
    }
}

/// 负载状态 API 响应
#[derive(Debug, Deserialize)]
struct LoadStatusResponse {
    current: CurrentLoadStatus,
}

/// 当前负载状态
#[derive(Debug, Deserialize)]
struct CurrentLoadStatus {
    load_percentage: f64,
    load_level: String,
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

/// 确保配置文件存在，如果不存在则创建模板
fn ensure_cookie_config_exists() -> Option<PathBuf> {
    let path = get_cookie_config_path()?;

    // 如果文件已存在，直接返回
    if path.exists() {
        return Some(path);
    }

    // 创建目录
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return None;
        }
    }

    // 创建默认配置文件
    let default_config = CookieConfig::default();
    let content = serde_json::to_string_pretty(&default_config).ok()?;
    std::fs::write(&path, content).ok()?;

    Some(path)
}

/// 读取 cookie 配置
/// 返回: Ok(Some(cookie)) - cookie 已配置
///       Ok(None) - cookie 为空（未配置）
///       Err - 文件读取失败
fn read_cookie() -> Result<Option<String>, String> {
    let path = ensure_cookie_config_exists().ok_or("无法创建配置文件")?;

    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let config: CookieConfig = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    if config.cookie.trim().is_empty() {
        Ok(None) // Cookie 为空，未配置
    } else {
        Ok(Some(config.cookie))
    }
}

/// 请求负载状态
fn fetch_load_status(cookie: &str) -> Result<(f64, String), String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(LOAD_STATUS_URL)
        .header("Cookie", cookie)
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

    let resp: LoadStatusResponse = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    Ok((resp.current.load_percentage, resp.current.load_level))
}

/// ANSI 颜色代码
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

/// 根据状态返回对应的 emoji 和描述
fn get_status_display(load_level: &str, load_percentage: f64) -> (String, String) {
    // 转换为百分比整数显示
    let percent = (load_percentage * 100.0).round() as i64;

    match load_level {
        "normal" => {
            let emoji = "🚴";
            let status_emoji = "😎";
            // 绿色数字
            (
                format!("{} 负载[{}{}%{}-使劲蹬{}]", emoji, GREEN, percent, RESET, status_emoji),
                "normal".to_string(),
            )
        }
        "warning" => {
            let emoji = "🚴";
            let status_emoji = "😰";
            // 黄色数字
            (
                format!("{} 负载[{}{}%{}-轻点蹬{}]", emoji, YELLOW, percent, RESET, status_emoji),
                "warning".to_string(),
            )
        }
        "emergency" => {
            let emoji = "💥";
            let status_emoji = "🥵";
            // 红色数字
            (
                format!("{} 负载[{}{}%{}-蹬炸了{}]", emoji, RED, percent, RESET, status_emoji),
                "emergency".to_string(),
            )
        }
        _ => {
            let emoji = "❓";
            (
                format!("{} 负载[{}%-未知]", emoji, percent),
                "unknown".to_string(),
            )
        }
    }
}

/// 收集 Cubence 负载状态数据
pub fn collect(config: &Config, _input: &InputData) -> Option<SegmentData> {
    let segment = config
        .segments
        .iter()
        .find(|s| matches!(s.id, crate::config::SegmentId::CubenceLoadStatus))?;

    if !segment.enabled {
        return None;
    }

    // 检查是否是 Cubence 服务商
    let vendor = crate::api::detect_vendor_from_claude_settings();
    if vendor != VendorType::Cubence {
        return None;
    }

    let mut metadata = HashMap::new();

    // 检查失败次数，超过3次则不再请求
    let failures = FAILURE_COUNT.load(Ordering::Relaxed);
    if failures >= MAX_FAILURES {
        metadata.insert("status".to_string(), "disabled".to_string());
        return Some(SegmentData {
            primary: "🔒 负载: Cookie已失效".to_string(),
            secondary: String::new(),
            metadata,
        });
    }

    // 读取 cookie
    let cookie = match read_cookie() {
        Ok(Some(c)) => c,
        Ok(None) => {
            // Cookie 为空，未配置
            metadata.insert("status".to_string(), "not_configured".to_string());
            return Some(SegmentData {
                primary: "🔧 负载: 请配置Cookie".to_string(),
                secondary: String::new(),
                metadata,
            });
        }
        Err(_) => {
            // 文件读取失败
            metadata.insert("status".to_string(), "config_error".to_string());
            return Some(SegmentData {
                primary: "⚠️ 负载: 配置文件错误".to_string(),
                secondary: String::new(),
                metadata,
            });
        }
    };

    // 请求负载状态
    match fetch_load_status(&cookie) {
        Ok((load_percentage, load_level)) => {
            // 成功，重置失败计数
            FAILURE_COUNT.store(0, Ordering::Relaxed);

            let (display, status) = get_status_display(&load_level, load_percentage);
            let percent = (load_percentage * 100.0).round() as i64;
            metadata.insert("status".to_string(), status);
            metadata.insert("load_percentage".to_string(), percent.to_string());

            Some(SegmentData {
                primary: display,
                secondary: String::new(),
                metadata,
            })
        }
        Err(_) => {
            // 失败，增加计数
            FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);

            metadata.insert("status".to_string(), "invalid".to_string());
            Some(SegmentData {
                primary: "🔒 负载: Cookie已失效".to_string(),
                secondary: String::new(),
                metadata,
            })
        }
    }
}
