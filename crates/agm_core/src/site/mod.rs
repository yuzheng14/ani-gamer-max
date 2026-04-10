//! HTTP access to ani.gamer.com.tw / api.gamer.com.tw using `wreq` browser emulation.

mod parse;

pub use parse::{EpisodeMetadata, parse_episode_from_mobile_json, parse_episode_from_web_html};

use std::time::Duration;

use wreq::header::{
    ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, CACHE_CONTROL, CONNECTION, COOKIE, ORIGIN, REFERER, USER_AGENT,
};
use wreq::{Client, EmulationFactory, Proxy};
use wreq_util::{Emulation, EmulationOption};

use crate::config::AppConfig;
use crate::error::{CoreError, CoreResult};

const MOBILE_UA: &str =
    "Animad/1.16.16 (tw.com.gamer.android.animad; build:328; Android 9) okHttp/4.4.0";

/// Site client with TLS fingerprinting aligned to user-agent (Chrome vs Firefox), per `architecture.md`.
#[derive(Clone)]
pub struct GamerSiteClient {
    inner: Client,
    ua: String,
    web_emulation: wreq::Emulation,
    mobile_emulation: wreq::Emulation,
}

impl GamerSiteClient {
    /// Builds a client from application config (proxy, UA → emulation profile).
    pub fn from_config(cfg: &AppConfig) -> CoreResult<Self> {
        let ua = cfg.ua.clone();
        let web_emulation = browser_emulation(&ua);
        let mobile_emulation = mobile_tls_emulation();

        let mut builder = Client::builder()
            .timeout(Duration::from_secs(120))
            .emulation(web_emulation.clone());

        if cfg.use_proxy && !cfg.proxy.trim().is_empty() {
            let mut proxy = Proxy::all(cfg.proxy.trim()).map_err(|e| CoreError::HttpClient(e.to_string()))?;
            if cfg.no_proxy_akamai {
                proxy = proxy.no_proxy(wreq::NoProxy::from_string("bahamut.akamaized.net"));
            }
            builder = builder.proxy(proxy);
        }

        let inner = builder
            .build()
            .map_err(|e: wreq::Error| CoreError::HttpClient(e.to_string()))?;

        Ok(Self {
            inner,
            ua,
            web_emulation,
            mobile_emulation,
        })
    }

    /// Fetches episode metadata for `sn` (web HTML or mobile JSON, matching `use_mobile_api`).
    pub async fn fetch_episode_metadata(
        &self,
        sn: u32,
        use_mobile_api: bool,
        cookie: Option<&str>,
    ) -> CoreResult<EpisodeMetadata> {
        if use_mobile_api {
            self.fetch_episode_metadata_mobile(sn, cookie).await
        } else {
            self.fetch_episode_metadata_web(sn, cookie).await
        }
    }

    async fn fetch_episode_metadata_web(&self, sn: u32, cookie: Option<&str>) -> CoreResult<EpisodeMetadata> {
        let url = format!("https://ani.gamer.com.tw/animeVideo.php?sn={sn}");
        let mut req = self
            .inner
            .get(&url)
            .emulation(self.web_emulation.clone())
            .header(
                USER_AGENT,
                wreq::header::HeaderValue::from_str(&self.ua).map_err(|e| CoreError::HttpClient(e.to_string()))?,
            )
            .header(REFERER, &url)
            .header(ACCEPT_LANGUAGE, "zh-TW,zh;q=0.9,en-US;q=0.8,en;q=0.6")
            .header(
                ACCEPT,
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8",
            )
            .header(ACCEPT_ENCODING, "gzip, deflate")
            .header(CACHE_CONTROL, "max-age=0")
            .header(ORIGIN, "https://ani.gamer.com.tw");

        if let Some(c) = cookie.filter(|s| !s.trim().is_empty()) {
            req = req.header(
                COOKIE,
                wreq::header::HeaderValue::from_str(c.trim()).map_err(|e| CoreError::HttpClient(e.to_string()))?,
            );
        }

        let resp = req.send().await.map_err(|e| CoreError::HttpClient(e.to_string()))?;
        let status = resp.status().as_u16();
        let body = resp.text().await.map_err(|e| CoreError::HttpClient(e.to_string()))?;
        if !(200..300).contains(&status) {
            return Err(CoreError::HttpStatus {
                status,
                detail: truncate_body(&body),
            });
        }

        parse_episode_from_web_html(sn, &body)
    }

    async fn fetch_episode_metadata_mobile(&self, sn: u32, cookie: Option<&str>) -> CoreResult<EpisodeMetadata> {
        let url = format!("https://api.gamer.com.tw/mobile_app/anime/v4/video.php?sn={sn}");
        let mut req = self
            .inner
            .get(&url)
            .emulation(self.mobile_emulation.clone())
            .header(
                USER_AGENT,
                wreq::header::HeaderValue::from_static(MOBILE_UA),
            )
            .header(
                wreq::header::HeaderName::from_static("x-bahamut-app-android"),
                "tw.com.gamer.android.animad",
            )
            .header(
                wreq::header::HeaderName::from_static("x-bahamut-app-version"),
                "328",
            )
            .header(ACCEPT_ENCODING, "gzip")
            .header(CONNECTION, "Keep-Alive");

        if let Some(c) = cookie.filter(|s| !s.trim().is_empty()) {
            req = req.header(
                COOKIE,
                wreq::header::HeaderValue::from_str(c.trim()).map_err(|e| CoreError::HttpClient(e.to_string()))?,
            );
        }

        let resp = req.send().await.map_err(|e| CoreError::HttpClient(e.to_string()))?;
        let status = resp.status().as_u16();
        let body = resp.text().await.map_err(|e| CoreError::HttpClient(e.to_string()))?;
        if !(200..300).contains(&status) {
            return Err(CoreError::HttpStatus {
                status,
                detail: truncate_body(&body),
            });
        }

        let v: serde_json::Value = serde_json::from_str(&body)?;
        parse_episode_from_mobile_json(sn, &v)
    }
}

fn browser_emulation(ua: &str) -> wreq::Emulation {
    let util = if ua.to_ascii_lowercase().contains("firefox") {
        Emulation::Firefox136
    } else {
        Emulation::Chrome107
    };
    EmulationOption::builder()
        .emulation(util)
        .skip_headers(true)
        .build()
        .emulation()
}

fn mobile_tls_emulation() -> wreq::Emulation {
    EmulationOption::builder()
        .emulation(Emulation::OkHttp4_12)
        .skip_headers(true)
        .build()
        .emulation()
}

fn truncate_body(s: &str) -> String {
    let t = s.trim();
    if t.len() > 512 {
        format!("{}…", &t[..512])
    } else {
        t.to_string()
    }
}
