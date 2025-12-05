pub mod cache;
pub mod client;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ============== 厂商类型定义 ==============

/// 检测当前配置的服务商类型
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum VendorType {
    Uucode,
    Cubence,
    Unknown,
}

impl VendorType {
    /// 获取服务商显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            VendorType::Uucode => "uucode",
            VendorType::Cubence => "Cubence",
            VendorType::Unknown => "未知",
        }
    }

    /// 是否是支持的服务商
    pub fn is_supported(&self) -> bool {
        !matches!(self, VendorType::Unknown)
    }
}

/// 厂商 URL 模式配置
/// 每个厂商可以有多个 base URL 域名
pub struct VendorUrlPatterns {
    pub vendor_type: VendorType,
    pub display_name: &'static str,
    pub url_patterns: &'static [&'static str],
}

/// 支持的厂商及其 URL 模式
pub const VENDOR_CONFIGS: &[VendorUrlPatterns] = &[
    VendorUrlPatterns {
        vendor_type: VendorType::Uucode,
        display_name: "uucode",
        url_patterns: &["uucode.org", "cometix.cn"],
    },
    VendorUrlPatterns {
        vendor_type: VendorType::Cubence,
        display_name: "Cubence",
        url_patterns: &[
            "cubence.com",
            "api.cubence.com",
            "api-dmit.cubence.com",
            "api-bwg.cubence.com",
            "api-cf.cubence.com",
        ],
    },
];

/// 获取支持的服务商列表字符串（用于提示信息）
pub fn get_supported_vendors_str() -> String {
    VENDOR_CONFIGS
        .iter()
        .map(|v| v.display_name)
        .collect::<Vec<_>>()
        .join(", ")
}

/// 获取所有支持的 URL 模式字符串（用于详细提示）
pub fn get_all_supported_urls_str() -> String {
    VENDOR_CONFIGS
        .iter()
        .flat_map(|v| v.url_patterns.iter())
        .copied()
        .collect::<Vec<_>>()
        .join(", ")
}

/// 根据 URL 检测厂商类型
pub fn detect_vendor_from_url(url: &str) -> VendorType {
    for config in VENDOR_CONFIGS {
        for pattern in config.url_patterns {
            if url.contains(pattern) {
                return config.vendor_type;
            }
        }
    }
    VendorType::Unknown
}

/// 检查 URL 是否属于指定厂商
pub fn url_matches_vendor(url: &str, vendor: &VendorType) -> bool {
    for config in VENDOR_CONFIGS {
        if &config.vendor_type == vendor {
            return config.url_patterns.iter().any(|p| url.contains(p));
        }
    }
    false
}

// ============== API 配置 ==============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub enabled: bool,
    pub api_key: String,
    pub usage_url: String,
    pub subscription_url: String,
    /// 是否自动从浏览器读取 Cookie
    #[serde(default)]
    pub auto_cookie: bool,
    /// 手动配置的 Cookie（优先级高于自动读取）
    #[serde(default)]
    pub cookie: Option<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: String::new(),
            usage_url: "https://api.uucode.org/account/billing".to_string(),
            subscription_url: String::new(),
            auto_cookie: false,
            cookie: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum UsageData {
    /// uucode 计费接口
    NewVendor(NewVendorData),
    /// Cubence 计费接口
    Cubence(CubenceData),
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
        match self {
            UsageData::NewVendor(data) => data.calculate(),
            UsageData::Cubence(data) => data.calculate(),
        }
    }

    pub fn is_exhausted(&self) -> bool {
        match self {
            UsageData::NewVendor(data) => data.is_exhausted(),
            UsageData::Cubence(data) => data.is_exhausted(),
        }
    }

    pub fn get_used_tokens(&self) -> u64 {
        match self {
            UsageData::NewVendor(data) => data.used_tokens,
            // Cubence 使用5小时窗口的已用量
            UsageData::Cubence(data) => data.five_hour_used as u64,
        }
    }

    pub fn get_remaining_tokens(&self) -> u64 {
        match self {
            UsageData::NewVendor(data) => data.remaining_tokens,
            // Cubence 使用5小时窗口的剩余量
            UsageData::Cubence(data) => data.five_hour_remaining as u64,
        }
    }

    pub fn get_credit_limit(&self) -> f64 {
        match self {
            UsageData::NewVendor(data) => data.credit_limit,
            // Cubence 返回账户余额
            UsageData::Cubence(data) => data.balance_usd,
        }
    }

    /// 返回订阅名称
    pub fn get_subscription_name(&self) -> Option<&str> {
        match self {
            UsageData::NewVendor(data) if !data.subscription_name.is_empty() => {
                Some(data.subscription_name.as_str())
            }
            // Cubence 没有订阅名称概念
            UsageData::Cubence(_) => None,
            _ => None,
        }
    }

    /// 返回订阅剩余秒数
    pub fn get_remaining_seconds(&self) -> Option<i64> {
        match self {
            UsageData::NewVendor(data) if data.remaining_seconds > 0 => {
                Some(data.remaining_seconds)
            }
            // Cubence 返回5小时窗口重置秒数
            UsageData::Cubence(data) => {
                let seconds = data.get_five_hour_reset_seconds();
                if seconds > 0 {
                    Some(seconds)
                } else {
                    None
                }
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
            // Cubence 没有 PAYG 概念
            UsageData::Cubence(_) => None,
            _ => None,
        }
    }

    /// 获取 Cubence 数据（如果是 Cubence 类型）
    pub fn as_cubence(&self) -> Option<&CubenceData> {
        match self {
            UsageData::Cubence(data) => Some(data),
            _ => None,
        }
    }
}

impl NewVendorData {
    /// 创建占位符数据（用于首次加载时显示）
    pub fn default_placeholder() -> Self {
        Self {
            usage_usd: "0".to_string(),
            limit_usd: "0".to_string(),
            subscription_name: String::new(),
            remaining_seconds: 0,
            payg_balance_usd: "0".to_string(),
            used_tokens: 0,
            remaining_tokens: 0,
            percentage_used: 0.0,
            credit_limit: 0.0,
            current_credits: 0.0,
        }
    }

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

// ============== Cubence 数据结构 ==============

/// Cubence API 响应结构
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CubenceResponse {
    pub normal_balance: CubenceBalance,
    pub subscription_window: CubenceSubscriptionWindow,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CubenceBalance {
    pub amount_dollar: f64,
    pub amount_units: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CubenceSubscriptionWindow {
    pub five_hour: CubenceWindowInfo,
    pub weekly: CubenceWindowInfo,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CubenceWindowInfo {
    pub limit: i64,
    pub remaining: i64,
    pub reset_at: i64,
    pub used: i64,
}

/// Cubence 标准化数据（计算后）
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CubenceData {
    /// 账户余额（美元）
    pub balance_usd: f64,
    /// 5小时窗口已用
    pub five_hour_used: i64,
    /// 5小时窗口限制
    pub five_hour_limit: i64,
    /// 5小时窗口剩余
    pub five_hour_remaining: i64,
    /// 5小时窗口重置时间戳
    pub five_hour_reset_at: i64,
    /// 周窗口已用
    pub weekly_used: i64,
    /// 周窗口限制
    pub weekly_limit: i64,
    /// 周窗口剩余
    pub weekly_remaining: i64,
    /// 周窗口重置时间戳
    pub weekly_reset_at: i64,
    /// 时间戳
    pub timestamp: i64,

    // 计算字段
    #[serde(default)]
    pub five_hour_percentage: f64,
    #[serde(default)]
    pub weekly_percentage: f64,
}

impl CubenceData {
    /// 创建占位符数据（用于首次加载时显示）
    pub fn default_placeholder() -> Self {
        Self {
            balance_usd: 0.0,
            five_hour_used: 0,
            five_hour_limit: 1, // 避免除零
            five_hour_remaining: 0,
            five_hour_reset_at: 0,
            weekly_used: 0,
            weekly_limit: 1, // 避免除零
            weekly_remaining: 0,
            weekly_reset_at: 0,
            timestamp: 0,
            five_hour_percentage: 0.0,
            weekly_percentage: 0.0,
        }
    }

    /// 从 API 响应创建
    pub fn from_response(resp: CubenceResponse) -> Self {
        let mut data = Self {
            balance_usd: resp.normal_balance.amount_dollar,
            five_hour_used: resp.subscription_window.five_hour.used,
            five_hour_limit: resp.subscription_window.five_hour.limit,
            five_hour_remaining: resp.subscription_window.five_hour.remaining,
            five_hour_reset_at: resp.subscription_window.five_hour.reset_at,
            weekly_used: resp.subscription_window.weekly.used,
            weekly_limit: resp.subscription_window.weekly.limit,
            weekly_remaining: resp.subscription_window.weekly.remaining,
            weekly_reset_at: resp.subscription_window.weekly.reset_at,
            timestamp: resp.timestamp,
            five_hour_percentage: 0.0,
            weekly_percentage: 0.0,
        };
        data.calculate();
        data
    }

    pub fn calculate(&mut self) {
        // 计算5小时窗口使用百分比
        self.five_hour_percentage = if self.five_hour_limit > 0 {
            (self.five_hour_used as f64 / self.five_hour_limit as f64 * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };

        // 计算周窗口使用百分比
        self.weekly_percentage = if self.weekly_limit > 0 {
            (self.weekly_used as f64 / self.weekly_limit as f64 * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };
    }

    /// 5小时窗口是否用完
    pub fn is_five_hour_exhausted(&self) -> bool {
        self.five_hour_remaining <= 0
    }

    /// 周窗口是否用完
    pub fn is_weekly_exhausted(&self) -> bool {
        self.weekly_remaining <= 0
    }

    /// 是否完全用完（两个窗口都用完）
    pub fn is_exhausted(&self) -> bool {
        self.is_five_hour_exhausted() && self.is_weekly_exhausted()
    }

    /// 格式化 token 数量为可读字符串（如 18.4M）
    pub fn format_tokens(tokens: i64) -> String {
        if tokens >= 1_000_000 {
            format!("{:.1}M", tokens as f64 / 1_000_000.0)
        } else if tokens >= 1_000 {
            format!("{:.1}K", tokens as f64 / 1_000.0)
        } else {
            tokens.to_string()
        }
    }

    /// 计算重置剩余时间（秒）
    pub fn get_five_hour_reset_seconds(&self) -> i64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        (self.five_hour_reset_at - now).max(0)
    }

    /// 计算周重置剩余时间（秒）
    pub fn get_weekly_reset_seconds(&self) -> i64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        (self.weekly_reset_at - now).max(0)
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
/// 支持所有在 VENDOR_CONFIGS 中配置的厂商
pub fn get_api_key_from_claude_settings() -> Option<String> {
    let settings_path = get_claude_settings_path()?;

    if !settings_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(settings_path).ok()?;
    let settings: ClaudeSettings = serde_json::from_str(&content).ok()?;

    let env = settings.env?;

    // 当 ANTHROPIC_BASE_URL 指向支持的厂商时读取
    if let Some(base_url) = env.base_url {
        let vendor = detect_vendor_from_url(&base_url);
        if vendor.is_supported() {
            return env.auth_token;
        }
    }

    None
}

/// Read API key from Claude settings.json specifically for Cubence
pub fn get_cubence_api_key_from_claude_settings() -> Option<String> {
    let settings_path = get_claude_settings_path()?;

    if !settings_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(settings_path).ok()?;
    let settings: ClaudeSettings = serde_json::from_str(&content).ok()?;

    let env = settings.env?;

    // 仅当 ANTHROPIC_BASE_URL 指向 Cubence 时读取
    if let Some(base_url) = env.base_url {
        if url_matches_vendor(&base_url, &VendorType::Cubence) {
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

    let vendor = detect_vendor_from_url(base_url);
    match vendor {
        VendorType::Uucode => Some("https://api.uucode.org/account/billing".to_string()),
        VendorType::Cubence => Some("https://cubence.com/api/v1/user/subscription-info".to_string()),
        VendorType::Unknown => None,
    }
}

/// 从 Claude settings.json 检测服务商类型
pub fn detect_vendor_from_claude_settings() -> VendorType {
    let settings_path = match get_claude_settings_path() {
        Some(p) => p,
        None => return VendorType::Unknown,
    };

    if !settings_path.exists() {
        return VendorType::Unknown;
    }

    let content = match std::fs::read_to_string(&settings_path) {
        Ok(c) => c,
        Err(_) => return VendorType::Unknown,
    };

    let settings: serde_json::Value = match serde_json::from_str(&content) {
        Ok(s) => s,
        Err(_) => return VendorType::Unknown,
    };

    let base_url = settings
        .get("env")
        .and_then(|e| e.get("ANTHROPIC_BASE_URL"))
        .and_then(|u| u.as_str())
        .unwrap_or("");

    // 使用新的多 URL 模式检测
    detect_vendor_from_url(base_url)
}

/// 获取当前 base_url（用于错误提示）
pub fn get_current_base_url() -> Option<String> {
    let settings_path = get_claude_settings_path()?;
    let content = std::fs::read_to_string(&settings_path).ok()?;
    let settings: serde_json::Value = serde_json::from_str(&content).ok()?;
    settings
        .get("env")?
        .get("ANTHROPIC_BASE_URL")?
        .as_str()
        .map(|s| s.to_string())
}

/// 检查当前服务商是否支持，返回错误信息（如果不支持）
pub fn check_vendor_support() -> Result<VendorType, String> {
    let vendor = detect_vendor_from_claude_settings();
    if vendor.is_supported() {
        Ok(vendor)
    } else {
        let current_url = get_current_base_url().unwrap_or_else(|| "未配置".to_string());
        Err(format!(
            "mycode 不支持当前厂商，请检查你的 ANTHROPIC_BASE_URL。当前: {}，支持: {}",
            current_url,
            get_supported_vendors_str()
        ))
    }
}
