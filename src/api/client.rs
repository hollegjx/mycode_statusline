use super::{ApiConfig, CubenceData, CubenceResponse, SubscriptionData, UsageData};
use reqwest::blocking::Client;
use std::time::Duration;

pub struct ApiClient {
    config: ApiConfig,
    client: Client,
    /// 缓存的 Cookie（手动配置）
    cached_cookie: Option<String>,
}

impl ApiClient {
    pub fn new(config: ApiConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("uucode/1.0.0")
            .build()?;

        // 获取 Cookie：使用手动配置
        let cached_cookie = config.cookie.clone();

        Ok(Self {
            config,
            client,
            cached_cookie,
        })
    }

    /// 获取当前使用的 Cookie
    pub fn get_cookie(&self) -> Option<&str> {
        self.cached_cookie.as_deref()
    }

    pub fn get_usage(&self) -> Result<UsageData, Box<dyn std::error::Error>> {
        // 根据 URL 判断是哪个服务商
        if self.config.usage_url.contains("cubence.com") {
            self.get_cubence_usage()
        } else {
            self.get_uucode_usage()
        }
    }

    /// 获取 uucode 用量数据
    fn get_uucode_usage(&self) -> Result<UsageData, Box<dyn std::error::Error>> {
        let response = self
            .client
            .get(&self.config.usage_url)
            .header("X-API-Key", &self.config.api_key)
            .send()?;

        if !response.status().is_success() {
            return Err(format!("Usage API request failed: {}", response.status()).into());
        }

        let response_text = response.text()?;

        let mut usage: UsageData = {
            let resp: super::NewVendorResponse =
                serde_json::from_str(&response_text).map_err(|e| {
                    format!(
                        "uucode JSON parse error: {} | Response: {}",
                        e, response_text
                    )
                })?;
            UsageData::NewVendor(resp.data)
        };

        usage.calculate();
        Ok(usage)
    }

    /// 获取 Cubence 用量数据
    fn get_cubence_usage(&self) -> Result<UsageData, Box<dyn std::error::Error>> {
        let mut request = self
            .client
            .get(&self.config.usage_url)
            .header("Authorization", &self.config.api_key);

        // 如果有 Cookie，添加到请求头
        if let Some(ref cookie) = self.cached_cookie {
            request = request.header("Cookie", cookie);
        }

        let response = request.send()?;

        if !response.status().is_success() {
            return Err(format!("Cubence API request failed: {}", response.status()).into());
        }

        let response_text = response.text()?;

        let resp: CubenceResponse = serde_json::from_str(&response_text).map_err(|e| {
            format!(
                "Cubence JSON parse error: {} | Response: {}",
                e, response_text
            )
        })?;

        let data = CubenceData::from_response(resp);
        Ok(UsageData::Cubence(data))
    }

    pub fn get_subscriptions(&self) -> Result<Vec<SubscriptionData>, Box<dyn std::error::Error>> {
        let response = self
            .client
            .post(&self.config.subscription_url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .send()?;

        if !response.status().is_success() {
            return Err(format!("Subscription API request failed: {}", response.status()).into());
        }

        // API返回的是数组,返回所有订阅
        let mut subscriptions: Vec<SubscriptionData> = response.json()?;

        // 格式化每个订阅的显示数据
        for subscription in &mut subscriptions {
            subscription.format();
        }

        Ok(subscriptions)
    }

    pub fn check_token_limit(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let usage = self.get_usage()?;
        Ok(usage.get_remaining_tokens() == 0)
    }
}
