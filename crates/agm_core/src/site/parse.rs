use std::collections::HashMap;

use regex::Regex;
use scraper::{Html, Selector};
use serde_json::Value;

use crate::error::{CoreError, CoreResult};

/// Metadata for one anime video page (one `sn`), aligned with `original/Anime.py` + `report/api-interactions.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeMetadata {
    /// Raw page title, e.g. `番名 [01]`.
    pub title_display: String,
    pub bangumi_name: String,
    /// Episode label used for filenames / DB (string, may be `1`, `特別篇1`, etc.).
    pub episode_label: String,
    /// Map episode label → sn (current sn always included).
    pub episodes: HashMap<String, u32>,
}

pub fn parse_episode_from_web_html(sn: u32, html: &str) -> CoreResult<EpisodeMetadata> {
    let document = Html::parse_document(html);

    let title_sel = Selector::parse("div.anime_name h1").map_err(|_| CoreError::Message("invalid CSS selector".into()))?;
    let title_display = document
        .select(&title_sel)
        .next()
        .map(|h| h.text().collect::<String>().trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or(CoreError::EpisodeNotFound(sn))?;

    let episode_label = extract_episode_label_from_html(&document, &title_display);
    let bangumi_name = bangumi_name_from_title(&title_display, &episode_label);
    let episodes = parse_season_section(&document, sn, &episode_label);

    Ok(EpisodeMetadata {
        title_display,
        bangumi_name,
        episode_label,
        episodes,
    })
}

pub fn parse_episode_from_mobile_json(sn: u32, json: &Value) -> CoreResult<EpisodeMetadata> {
    let title_display = json
        .pointer("/data/anime/title")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(CoreError::EpisodeNotFound(sn))?
        .to_string();

    let episode_label = extract_episode_label_from_title(&title_display);
    let bangumi_name = bangumi_name_from_title(&title_display, &episode_label);
    let episodes = parse_mobile_episodes(json)?;

    Ok(EpisodeMetadata {
        title_display,
        bangumi_name,
        episode_label,
        episodes,
    })
}

fn extract_episode_label_from_html(document: &Html, title: &str) -> String {
    let playing = Selector::parse("li.playing a").ok().and_then(|sel| {
        document
            .select(&sel)
            .next()
            .map(|a| a.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty())
    });
    playing.unwrap_or_else(|| extract_episode_label_from_title(title))
}

fn parse_season_section(document: &Html, self_sn: u32, fallback_ep: &str) -> HashMap<String, u32> {
    let Ok(section_sel) = Selector::parse("section.season") else {
        return single_episode_map(self_sn, fallback_ep);
    };
    let Some(section) = document.select(&section_sel).next() else {
        return single_episode_map(self_sn, fallback_ep);
    };

    let Ok(a_sel) = Selector::parse("a") else {
        return single_episode_map(self_sn, fallback_ep);
    };
    let Ok(p_sel) = Selector::parse("p") else {
        return single_episode_map(self_sn, fallback_ep);
    };

    let p_labels: Vec<String> = section
        .select(&p_sel)
        .filter_map(|p| {
            let t = p.text().collect::<String>();
            let t = t.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        })
        .collect();

    let mut out: HashMap<String, u32> = HashMap::new();
    let mut index_counter: HashMap<String, usize> = HashMap::new();

    for a in section.select(&a_sel) {
        let Some(href) = a.value().attr("href") else {
            continue;
        };
        let Some(num_str) = href.strip_prefix("?sn=") else {
            continue;
        };
        let Ok(ep_sn) = num_str.parse::<u32>() else {
            continue;
        };
        let mut ep = a.text().collect::<String>();
        ep = ep.trim().to_string();
        if ep.is_empty() {
            continue;
        }

        index_counter.entry(ep.clone()).or_insert(0);
        if out.contains_key(&ep) {
            let idx = index_counter.get_mut(&ep).expect("just inserted");
            *idx += 1;
            if *idx < p_labels.len() {
                ep = format!("{}{}", p_labels[*idx], ep);
            }
        }
        out.insert(ep, ep_sn);
    }

    if out.is_empty() {
        single_episode_map(self_sn, fallback_ep)
    } else {
        out
    }
}

fn single_episode_map(sn: u32, ep: &str) -> HashMap<String, u32> {
    let mut m = HashMap::new();
    m.insert(ep.to_string(), sn);
    m
}

fn parse_mobile_episodes(json: &Value) -> CoreResult<HashMap<String, u32>> {
    let mut out = HashMap::new();
    let Some(ep_obj) = json.pointer("/data/anime/episodes").and_then(|e| e.as_object()) else {
        return Ok(out);
    };

    for (type_key, entries) in ep_obj {
        let Some(arr) = entries.as_array() else {
            continue;
        };
        for item in arr {
            let video_sn = item
                .get("videoSn")
                .and_then(|v| v.as_u64())
                .or_else(|| item.get("videoSn").and_then(|v| v.as_i64()).map(|i| i as u64))
                .ok_or_else(|| CoreError::Message("mobile episode missing videoSn".into()))? as u32;
            let ep_raw = item.get("episode").and_then(|v| v.as_str()).unwrap_or("").trim();

            let key = match type_key.as_str() {
                "0" => {
                    if ep_raw.is_empty() {
                        continue;
                    }
                    ep_raw.to_string()
                }
                "1" => "電影".to_string(),
                "2" => format!("特別篇{ep_raw}"),
                "3" => format!("中文配音{ep_raw}"),
                _ => "中文電影".to_string(),
            };
            out.insert(key, video_sn);
        }
    }

    Ok(out)
}

fn bangumi_name_from_title(title: &str, episode: &str) -> String {
    let bracket = format!("[{episode}]");
    let mut s = title.replace(&bracket, "");
    s = regex_collapse_space(&s);
    s.trim().to_string()
}

fn extract_episode_label_from_title(title: &str) -> String {
    static RE_NUM: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    static RE_ANY: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();

    let re_num = RE_NUM.get_or_init(|| {
        Regex::new(r"\[(\d*\.?\d* *\.?[A-Za-z]*(?:電影)?)\]").expect("regex")
    });
    if let Some(c) = re_num.captures(title) {
        return c.get(1).map(|m| m.as_str().to_string()).unwrap_or_else(|| "1".into());
    }

    let re_any = RE_ANY.get_or_init(|| Regex::new(r"\[(.+?)\]").expect("regex"));
    if let Some(c) = re_any.captures(title) {
        return c.get(1).map(|m| m.as_str().to_string()).unwrap_or_else(|| "1".into());
    }

    "1".to_string()
}

fn regex_collapse_space(s: &str) -> String {
    static RE_SPC: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re = RE_SPC.get_or_init(|| Regex::new(r"\s+").expect("regex"));
    re.replace_all(s, " ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_episode_bracket() {
        let t = "測試番 [12]";
        let ep = extract_episode_label_from_title(t);
        assert_eq!(ep, "12");
        assert_eq!(bangumi_name_from_title(t, &ep), "測試番");
    }

    #[test]
    fn web_html_minimal() {
        let html = r#"<!DOCTYPE html><html><body>
            <div class="anime_name"><h1>某番 [SP]</h1></div>
        </body></html>"#;
        let m = parse_episode_from_web_html(999, html).unwrap();
        assert_eq!(m.episode_label, "SP");
        assert_eq!(m.bangumi_name, "某番");
        assert_eq!(m.episodes.get("SP"), Some(&999));
    }

    #[test]
    fn mobile_json_minimal() {
        let j = serde_json::json!({
            "data": {
                "anime": {
                    "title": "某番 [01]",
                    "episodes": {
                        "0": [{"episode": "1", "videoSn": 100}]
                    }
                }
            }
        });
        let m = parse_episode_from_mobile_json(100, &j).unwrap();
        assert_eq!(m.episode_label, "01");
        assert_eq!(m.episodes.get("1"), Some(&100));
    }
}
