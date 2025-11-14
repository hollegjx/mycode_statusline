pub mod cache;
pub mod client;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub enabled: bool,
    pub api_key: String,
    pub usage_url: String,
    pub subscription_url: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: String::new(),
            usage_url: "https://api.uucode.org/account/billing".to_string(),
            subscription_url: String::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum UsageData {
    /// 目前仅支持 uucode 计费接口
    NewVendor(NewVendorData),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NewVendorResponse {
    pub data: NewVendorData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NewVendorData {
    pub usage_usd: String,
    pub limit_usd: String,
    pub subscription_name: String,
    pub remaining_seconds: i64,
    pub payg_balance_usd: String,

    #[serde(default)]
    pub used_tokens: u64,
    #[serde(default)]
    pub remaining_tokens: u64,
    #[serde(default)]
    pub percentage_used: f64,
    #[serde(default)]
    pub credit_limit: f64,
    #[serde(default)]
    pub current_credits: f64,
}

impl UsageData {
    pub fn calculate(&mut self) {
        let UsageData::NewVendor(data) = self;
        data.calculate();
    }

    pub fn is_exhausted(&self) -> bool {
        matches!(self, UsageData::NewVendor(data) if data.is_exhausted())
    }

    pub fn get_used_tokens(&self) -> u64 {
        match self {
            UsageData::NewVendor(data) => data.used_tokens,
        }
    }

    pub fn get_remaining_tokens(&self) -> u64 {
        match self {
            UsageData::NewVendor(data) => data.remaining_tokens,
        }
    }

    pub fn get_credit_limit(&self) -> f64 {
        match self {
            UsageData::NewVendor(data) => data.credit_limit,
        }
    }

    /// 返回订阅名称
    pub fn get_subscription_name(&self) -> Option<&str> {
        match self {
            UsageData::NewVendor(data) if !data.subscription_name.is_empty() => {
                Some(data.subscription_name.as_str())
            }
            _ => None,
        }
    }

    /// 返回订阅剩余秒数
    pub fn get_remaining_seconds(&self) -> Option<i64> {
        match self {
            UsageData::NewVendor(data) if data.remaining_seconds > 0 => {
                Some(data.remaining_seconds)
            }
            _ => None,
        }
    }

    /// 返回 PAYG 余额（美元字符串）
    pub fn get_payg_balance_usd(&self) -> Option<&str> {
        match self {
            UsageData::NewVendor(data) if !data.payg_balance_usd.is_empty() => {
                Some(data.payg_balance_usd.as_str())
            }
            _ => None,
        }
    }
}

impl NewVendorData {
    pub fn calculate(&mut self) {
        let usage = self.usage_usd.parse::<f64>().unwrap_or(0.0);
        let limit = self.limit_usd.parse::<f64>().unwrap_or(0.0);

        self.credit_limit = limit;
        self.current_credits = (limit - usage).max(0.0);

        self.used_tokens = (usage * 100.0).max(0.0) as u64;
        self.remaining_tokens = (self.current_credits * 100.0) as u64;

        self.percentage_used = if limit > 0.0 {
            (usage / limit * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };
    }

    pub fn is_exhausted(&self) -> bool {
        self.current_credits <= 0.0
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubscriptionData {
    #[serde(rename = "subscriptionPlanName")]
    pub plan_name: String,
    pub cost: f64,
    #[serde(rename = "endDate")]
    pub expires_at: Option<String>,
    #[serde(rename = "subscriptionStatus")]
    pub status: String,
    #[serde(rename = "remainingDays")]
    pub remaining_days: i32,
    #[serde(rename = "billingCycleDesc")]
    pub billing_cycle_desc: String,
    #[serde(rename = "resetTimes")]
    pub reset_times: i32,
    #[serde(rename = "isActive")]
    pub is_active: bool,

    // 计算字段
    #[serde(skip)]
    pub plan_price: String,
}

impl SubscriptionData {
    /// 格式化显示数据
    pub fn format(&mut self) {
        self.plan_price = format!("¥{}/{}", self.cost, self.billing_cycle_desc);
    }
}

/// Claude settings.json structure for reading API key
#[derive(Debug, Deserialize)]
struct ClaudeSettings {
    env: Option<ClaudeEnv>,
}

#[derive(Debug, Deserialize)]
struct ClaudeEnv {
    #[serde(rename = "ANTHROPIC_AUTH_TOKEN")]
    auth_token: Option<String>,
    #[serde(rename = "ANTHROPIC_BASE_URL")]
    base_url: Option<String>,
}

/// Get the path to Claude settings.json (cross-platform)
fn get_claude_settings_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".claude").join("settings.json"))
}

/// Read API key from Claude settings.json for supported vendors
/// 目前仅支持 uucode.org
pub fn get_api_key_from_claude_settings() -> Option<String> {
    let settings_path = get_claude_settings_path()?;

    if !settings_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(settings_path).ok()?;
    let settings: ClaudeSettings = serde_json::from_str(&content).ok()?;

    let env = settings.env?;

    // 仅当 ANTHROPIC_BASE_URL 指向 uucode.org 时读取
    if let Some(base_url) = env.base_url {
        if base_url.contains("uucode.org") {
            return env.auth_token;
        }
    }

    None
}

/// Get usage_url from Claude settings.json based on ANTHROPIC_BASE_URL
pub fn get_usage_url_from_claude_settings() -> Option<String> {
    let settings_path = get_claude_settings_path()?;

    if !settings_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(settings_path).ok()?;
    let settings = serde_json::from_str::<serde_json::Value>(&content).ok()?;

    let base_url = settings.get("env")?.get("ANTHROPIC_BASE_URL")?.as_str()?;

    if base_url.contains("uucode.org") {
        Some("https://api.uucode.org/account/billing".to_string())
    } else {
        None
    }
}
