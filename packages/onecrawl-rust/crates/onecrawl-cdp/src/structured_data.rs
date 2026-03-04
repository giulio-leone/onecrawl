//! Structured data extraction — JSON-LD, OpenGraph, Twitter Card, and
//! general page metadata from live browser pages via CDP.

use chromiumoxide::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─────────────────────────── Types ───────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonLdData {
    pub context: String,
    pub data_type: String,
    pub properties: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenGraphData {
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub url: Option<String>,
    pub site_name: Option<String>,
    pub og_type: Option<String>,
    pub locale: Option<String>,
    pub extra: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterCardData {
    pub card: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub site: Option<String>,
    pub creator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageMetadata {
    pub title: String,
    pub description: String,
    pub canonical_url: Option<String>,
    pub author: Option<String>,
    pub published_date: Option<String>,
    pub modified_date: Option<String>,
    pub language: Option<String>,
    pub charset: Option<String>,
    pub favicon: Option<String>,
    pub robots: Option<String>,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredDataResult {
    pub json_ld: Vec<JsonLdData>,
    pub open_graph: Option<OpenGraphData>,
    pub twitter_card: Option<TwitterCardData>,
    pub metadata: PageMetadata,
    pub schema_types: Vec<String>,
}

// ─────────────────────────── Extraction ──────────────────────

/// Extract all structured data types concurrently via tokio::join!.
pub async fn extract_all(page: &Page) -> Result<StructuredDataResult> {
    let (json_ld, open_graph, twitter_card, metadata) = tokio::join!(
        extract_json_ld(page),
        extract_open_graph(page),
        extract_twitter_card(page),
        extract_metadata(page),
    );
    let json_ld = json_ld?;
    let open_graph = open_graph?;
    let twitter_card = twitter_card?;
    let metadata = metadata?;

    let mut schema_types: Vec<String> = json_ld.iter().map(|j| j.data_type.clone()).collect();
    schema_types.dedup();

    let og = if open_graph.title.is_some()
        || open_graph.description.is_some()
        || open_graph.image.is_some()
    {
        Some(open_graph)
    } else {
        None
    };

    let tc = if twitter_card.card.is_some()
        || twitter_card.title.is_some()
        || twitter_card.image.is_some()
    {
        Some(twitter_card)
    } else {
        None
    };

    Ok(StructuredDataResult {
        json_ld,
        open_graph: og,
        twitter_card: tc,
        metadata,
        schema_types,
    })
}

/// Extract JSON-LD scripts from the page.
pub async fn extract_json_ld(page: &Page) -> Result<Vec<JsonLdData>> {
    let js = r#"(() => {
const scripts = document.querySelectorAll('script[type="application/ld+json"]');
const results = [];
scripts.forEach(s => {
    try {
        const data = JSON.parse(s.textContent);
        const items = Array.isArray(data) ? data : [data];
        items.forEach(item => {
            results.push({
                context: item['@context'] || '',
                data_type: item['@type'] || '',
                properties: item
            });
        });
    } catch(e) {}
});
return JSON.stringify(results);
})()"#;

    let raw: String = page
        .evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(e.to_string()))?
        .into_value()
        .map_err(|e| onecrawl_core::Error::Cdp(e.to_string()))?;

    let items: Vec<JsonLdData> = serde_json::from_str(&raw).unwrap_or_default();
    Ok(items)
}

/// Extract OpenGraph meta tags from the page.
pub async fn extract_open_graph(page: &Page) -> Result<OpenGraphData> {
    let js = r#"(() => {
const tags = {};
document.querySelectorAll('meta[property^="og:"]').forEach(m => {
    const key = (m.getAttribute('property') || '').replace('og:', '');
    const val = m.getAttribute('content') || '';
    tags[key] = val;
});
return JSON.stringify(tags);
})()"#;

    let raw: String = page
        .evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(e.to_string()))?
        .into_value()
        .map_err(|e| onecrawl_core::Error::Cdp(e.to_string()))?;

    let tags: HashMap<String, String> = serde_json::from_str(&raw).unwrap_or_default();

    let mut extra = tags.clone();
    let known = [
        "title",
        "description",
        "image",
        "url",
        "site_name",
        "type",
        "locale",
    ];
    for k in &known {
        extra.remove(*k);
    }

    Ok(OpenGraphData {
        title: tags.get("title").cloned(),
        description: tags.get("description").cloned(),
        image: tags.get("image").cloned(),
        url: tags.get("url").cloned(),
        site_name: tags.get("site_name").cloned(),
        og_type: tags.get("type").cloned(),
        locale: tags.get("locale").cloned(),
        extra,
    })
}

/// Extract Twitter Card meta tags from the page.
pub async fn extract_twitter_card(page: &Page) -> Result<TwitterCardData> {
    let js = r#"(() => {
const tags = {};
document.querySelectorAll('meta[name^="twitter:"]').forEach(m => {
    const key = (m.getAttribute('name') || '').replace('twitter:', '');
    const val = m.getAttribute('content') || '';
    tags[key] = val;
});
return JSON.stringify(tags);
})()"#;

    let raw: String = page
        .evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(e.to_string()))?
        .into_value()
        .map_err(|e| onecrawl_core::Error::Cdp(e.to_string()))?;

    let tags: HashMap<String, String> = serde_json::from_str(&raw).unwrap_or_default();

    Ok(TwitterCardData {
        card: tags.get("card").cloned(),
        title: tags.get("title").cloned(),
        description: tags.get("description").cloned(),
        image: tags.get("image").cloned(),
        site: tags.get("site").cloned(),
        creator: tags.get("creator").cloned(),
    })
}

/// Extract general page metadata.
pub async fn extract_metadata(page: &Page) -> Result<PageMetadata> {
    let js = r#"(() => {
const meta = (name) => {
    const el = document.querySelector(`meta[name="${name}"]`) ||
               document.querySelector(`meta[property="${name}"]`);
    return el ? (el.getAttribute('content') || '') : '';
};
const link = (rel) => {
    const el = document.querySelector(`link[rel="${rel}"]`);
    return el ? (el.getAttribute('href') || '') : '';
};
const charset = document.querySelector('meta[charset]');
const lang = document.documentElement.lang || '';

return JSON.stringify({
    title: document.title || '',
    description: meta('description'),
    canonical_url: link('canonical'),
    author: meta('author'),
    published_date: meta('article:published_time') || meta('date') || meta('DC.date.issued'),
    modified_date: meta('article:modified_time') || meta('last-modified'),
    language: lang,
    charset: charset ? charset.getAttribute('charset') : '',
    favicon: link('icon') || link('shortcut icon'),
    robots: meta('robots'),
    keywords: meta('keywords')
});
})()"#;

    let raw: String = page
        .evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(e.to_string()))?
        .into_value()
        .map_err(|e| onecrawl_core::Error::Cdp(e.to_string()))?;

    let m: HashMap<String, String> = serde_json::from_str(&raw).unwrap_or_default();

    let keywords_str = m.get("keywords").cloned().unwrap_or_default();
    let keywords: Vec<String> = if keywords_str.is_empty() {
        vec![]
    } else {
        keywords_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    Ok(PageMetadata {
        title: m.get("title").cloned().unwrap_or_default(),
        description: m.get("description").cloned().unwrap_or_default(),
        canonical_url: non_empty(m.get("canonical_url")),
        author: non_empty(m.get("author")),
        published_date: non_empty(m.get("published_date")),
        modified_date: non_empty(m.get("modified_date")),
        language: non_empty(m.get("language")),
        charset: non_empty(m.get("charset")),
        favicon: non_empty(m.get("favicon")),
        robots: non_empty(m.get("robots")),
        keywords,
    })
}

// ─────────────────────────── Validation ──────────────────────

/// Validate structured data completeness. Returns a list of warnings.
pub fn validate_schema(data: &StructuredDataResult) -> Vec<String> {
    let mut warnings = Vec::new();

    if data.json_ld.is_empty() {
        warnings.push("no JSON-LD structured data found".into());
    }
    for (i, ld) in data.json_ld.iter().enumerate() {
        if ld.context.is_empty() {
            warnings.push(format!("JSON-LD[{i}]: missing @context"));
        }
        if ld.data_type.is_empty() {
            warnings.push(format!("JSON-LD[{i}]: missing @type"));
        }
    }

    if data.open_graph.is_none() {
        warnings.push("no OpenGraph metadata found".into());
    } else if let Some(og) = &data.open_graph {
        if og.title.is_none() {
            warnings.push("OpenGraph: missing og:title".into());
        }
        if og.description.is_none() {
            warnings.push("OpenGraph: missing og:description".into());
        }
        if og.image.is_none() {
            warnings.push("OpenGraph: missing og:image".into());
        }
    }

    if data.twitter_card.is_none() {
        warnings.push("no Twitter Card metadata found".into());
    } else if let Some(tc) = &data.twitter_card {
        if tc.card.is_none() {
            warnings.push("Twitter Card: missing twitter:card".into());
        }
        if tc.title.is_none() {
            warnings.push("Twitter Card: missing twitter:title".into());
        }
    }

    if data.metadata.title.is_empty() {
        warnings.push("page title is empty".into());
    }
    if data.metadata.description.is_empty() {
        warnings.push("meta description is empty".into());
    }
    if data.metadata.canonical_url.is_none() {
        warnings.push("canonical URL is not set".into());
    }
    if data.metadata.language.is_none() {
        warnings.push("html lang attribute is not set".into());
    }

    warnings
}

// ─────────────────────────── Helpers ─────────────────────────

fn non_empty(s: Option<&String>) -> Option<String> {
    s.filter(|v| !v.is_empty()).cloned()
}
