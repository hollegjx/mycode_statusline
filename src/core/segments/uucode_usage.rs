use crate::api::{cache, client::ApiClient, ApiConfig};
use crate::config::Config;
use crate::config::InputData;
use crate::core::segments::SegmentData;
use std::collections::HashMap;

pub fn collect(config: &Config, _input: &InputData) -> Option<SegmentData> {
    // Get API config from segment options
    let segment = config
        .segments
        .iter()
        .find(|s| matches!(s.id, crate::config::SegmentId::UucodeUsage))?;

    if !segment.enabled {
        return None;
    }

    // Try to get API key from segment options first, then from Claude settings
    let api_key = segment
        .options
        .get("api_key")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(crate::api::get_api_key_from_claude_settings);

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

    let usage_url = segment
        .options
        .get("usage_url")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(crate::api::get_usage_url_from_claude_settings)
        .unwrap_or_else(|| "https://api.uucode.org/account/billing".to_string());

    let is_uucode = usage_url.contains("uucode.org");

    // 只支持 uucode，其它服务直接给出提示，不再发起 API 请求
    if !is_uucode {
        let mut metadata = HashMap::new();
        metadata.insert("service".to_string(), "unsupported".to_string());
        return Some(SegmentData {
            primary: "仅支持 uucode，用量段已禁用".to_string(),
            secondary: String::new(),
            metadata,
        });
    }

    let subscription_url = segment
        .options
        .get("subscription_url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "".to_string());

    // 获取使用数据：每次先尝试请求 API，失败时回退到本地缓存
    let usage = if is_uucode {
        // 先拿到当前缓存（可能为空，用于失败回退）
        let (cached, _needs_refresh) = cache::get_cached_usage();

        // 1. 每次先尝试同步请求最新用量
        if let Some(mut fresh) = fetch_usage_sync(&api_key, &usage_url) {
            fresh.calculate();
            let _ = cache::save_cached_usage(&fresh);
            fresh
        } else if let Some(mut cached_usage) = cached {
            // 2. 请求失败：如果有缓存（无论是否过期）就继续显示缓存
            cached_usage.calculate();
            cached_usage
        } else {
            // 3. 既没有网络也没有缓存：整个段不显示
            return None;
        }
    } else {
        // 理论上不会走到这里（前面已经限制仅支持 uucode），保留兜底逻辑
        let mut fresh = fetch_usage_sync(&api_key, &usage_url)?;
        fresh.calculate();
        fresh
    };

    fn fetch_usage_sync(api_key: &str, usage_url: &str) -> Option<crate::api::UsageData> {
        let api_config = ApiConfig {
            enabled: true,
            api_key: api_key.to_string(),
            usage_url: usage_url.to_string(),
            subscription_url: String::new(),
        };

        let client = ApiClient::new(api_config).ok()?;
        let usage = client.get_usage().ok()?;
        Some(usage)
    }

    // 处理使用数据
    let used_dollars = usage.get_used_tokens() as f64 / 100.0;
    let remaining_dollars = (usage.get_remaining_tokens() as f64 / 100.0).max(0.0);
    let total_dollars = usage.get_credit_limit();

    let mut metadata = HashMap::new();
    metadata.insert("used".to_string(), format!("{:.2}", used_dollars));
    metadata.insert("total".to_string(), format!("{:.2}", total_dollars));
    metadata.insert("remaining".to_string(), format!("{:.2}", remaining_dollars));

    // 对 uucode，将订阅和 PAYG 信息也写入 metadata 方便主题使用
    if is_uucode {
        if let Some(name) = usage.get_subscription_name() {
            metadata.insert("subscription_name".to_string(), name.to_string());
        }
        if let Some(seconds) = usage.get_remaining_seconds() {
            metadata.insert("remaining_seconds".to_string(), seconds.to_string());
        }
        if let Some(payg) = usage.get_payg_balance_usd() {
            metadata.insert("payg_balance_usd".to_string(), payg.to_string());
        }
    }

    // 根据 usage_url 判断是哪个服务，并设置动态图标（目前仅 uucode 有效）
    let service_name = if usage_url.contains("uucode.org") {
        "uucode"
    } else {
        "unsupported"
    };
    metadata.insert("service".to_string(), service_name.to_string());
    // 对 uucode 不显示前缀文字，只保留用量信息
    if service_name == "uucode" {
        metadata.insert("dynamic_icon".to_string(), String::new());
    } else {
        metadata.insert("dynamic_icon".to_string(), service_name.to_string());
    }

    // 检查额度是否用完（包括超额使用）
    if usage.is_exhausted() {
        // uucode：直接使用 /account/billing 中的订阅信息，不再请求订阅接口
        if is_uucode {
            let payg_display = usage
                .get_payg_balance_usd()
                .and_then(|s| s.parse::<f64>().ok().map(|v| format!("{:.2}", v)))
                .unwrap_or_else(|| "-".to_string());

            if let Some(name) = usage.get_subscription_name() {
                if let Some(seconds) = usage.get_remaining_seconds() {
                    let days = if seconds > 0 {
                        // 简单按整天向上取整
                        ((seconds as f64) / 86_400.0).ceil() as i64
                    } else {
                        0
                    };

                    let secondary = if days > 0 {
                        format!(
                            "📦 {} - 剩余{}天 | 💰 payg余额 ${} | 欢迎使用uucode（额度已用完）",
                            name, days, payg_display
                        )
                    } else {
                        format!(
                            "📦 {} | 💰 payg余额 ${} | 欢迎使用uucode（额度已用完）",
                            name, payg_display
                        )
                    };

                    return Some(SegmentData {
                        primary: format!("💳 ${:.2} / ${:.0}", used_dollars, total_dollars),
                        secondary,
                        metadata,
                    });
                }
            }

            // 无订阅（subscription_name 为空）但有 PAYG 余额的情况
            if usage.get_subscription_name().is_none() {
                if let Some(payg) = usage
                    .get_payg_balance_usd()
                    .and_then(|s| s.parse::<f64>().ok())
                {
                    if payg > 0.0 {
                        return Some(SegmentData {
                            primary: format!("💳 ${:.2} / ${:.0}", used_dollars, total_dollars),
                            secondary: format!(
                                "📦 无订阅 - 使用PayGo额度中 | 💰 payg余额 ${:.2} | 欢迎使用uucode",
                                payg
                            ),
                            metadata,
                        });
                    }
                }
            }

            // 没有订阅信息且无 PAYG 余额时的兜底提示
            return Some(SegmentData {
                primary: format!("💳 ${:.2} / ${:.0}", used_dollars, total_dollars),
                secondary: "📦 额度已用完 | 欢迎使用uucode".to_string(),
                metadata,
            });
        }

        // 历史遗留：仅保留对旧订阅接口的兼容处理，uucode 已不使用此分支
        let subscriptions = fetch_subscriptions_sync(&api_key, &subscription_url);

        if let Some(subs) = subscriptions {
            let active_subs: Vec<_> = subs.iter().filter(|s| s.is_active).collect();

            if active_subs.len() > 1 {
                // 有多个订阅，提示切换到其他套餐
                return Some(SegmentData {
                    primary: format!("${:.2}/${:.0} 已用完", used_dollars, total_dollars),
                    secondary: "提示：你有其他套餐可用".to_string(),
                    metadata,
                });
            } else if active_subs.len() == 1 {
                // 只有一个订阅，提示手动重置
                let reset_times = active_subs[0].reset_times;
                if reset_times > 0 {
                    return Some(SegmentData {
                        primary: format!("${:.2}/${:.0} 已用完", used_dollars, total_dollars),
                        secondary: format!("可重置{}次，请手动重置", reset_times),
                        metadata,
                    });
                } else {
                    return Some(SegmentData {
                        primary: format!("${:.2}/${:.0} 已用完", used_dollars, total_dollars),
                        secondary: "无可用重置次数".to_string(),
                        metadata,
                    });
                }
            }
        }

        // 没有订阅信息或无活跃订阅，显示基本提示
        return Some(SegmentData {
            primary: format!("${:.2}/${:.0} 已用完", used_dollars, total_dollars),
            secondary: "请充值或重置额度".to_string(),
            metadata,
        });
    }

    // 正常显示
    if is_uucode {
        let primary = format!("💳 ${:.2} / ${:.0}", used_dollars, total_dollars);

        let payg_display = usage
            .get_payg_balance_usd()
            .and_then(|s| s.parse::<f64>().ok().map(|v| format!("{:.2}", v)))
            .unwrap_or_else(|| "-".to_string());

        let secondary = if let Some(name) = usage.get_subscription_name() {
            if let Some(seconds) = usage.get_remaining_seconds() {
                let days = if seconds > 0 {
                    ((seconds as f64) / 86_400.0).ceil() as i64
                } else {
                    0
                };

                if days > 0 {
                    format!(
                        "📦 {} - 剩余{}天 | 💰 payg余额 ${} | 欢迎使用uucode",
                        name, days, payg_display
                    )
                } else {
                    format!(
                        "📦 {} | 💰 payg余额 ${} | 欢迎使用uucode",
                        name, payg_display
                    )
                }
            } else {
                format!(
                    "📦 {} | 💰 payg余额 ${} | 欢迎使用uucode",
                    name, payg_display
                )
            }
        } else {
            format!(
                "📦 无订阅 - 使用PayGo额度中 | 💰 payg余额 ${} | 欢迎使用uucode",
                payg_display
            )
        };

        Some(SegmentData {
            primary,
            secondary,
            metadata,
        })
    } else {
        // 默认显示（非 uucode 场景）
        Some(SegmentData {
            primary: format!("${:.2}/${:.0}", used_dollars, total_dollars),
            secondary: format!("剩${:.2}", remaining_dollars),
            metadata,
        })
    }
}

fn fetch_subscriptions_sync(
    api_key: &str,
    subscription_url: &str,
) -> Option<Vec<crate::api::SubscriptionData>> {
    let api_config = ApiConfig {
        enabled: true,
        api_key: api_key.to_string(),
        usage_url: String::new(),
        subscription_url: subscription_url.to_string(),
    };

    let client = ApiClient::new(api_config).ok()?;
    let subs = client.get_subscriptions().ok()?;
    Some(subs)
}
