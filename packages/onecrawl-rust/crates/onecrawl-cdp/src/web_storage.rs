//! Web Storage (localStorage, sessionStorage, IndexedDB) management via JS APIs.

use chromiumoxide::Page;
use onecrawl_core::Result;

/// Get all localStorage contents as a JSON object.
pub async fn get_local_storage(page: &Page) -> Result<serde_json::Value> {
    let result = page
        .evaluate(
            r#"
            (() => {
                const obj = {};
                for (let i = 0; i < localStorage.length; i++) {
                    const k = localStorage.key(i);
                    obj[k] = localStorage.getItem(k);
                }
                return obj;
            })()
            "#,
        )
        .await
        .map_err(|e| onecrawl_core::Error::Browser(format!("get_local_storage failed: {e}")))?;

    let val: serde_json::Value = result.into_value().unwrap_or(serde_json::Value::Object(Default::default()));
    Ok(val)
}

/// Set a localStorage item.
pub async fn set_local_storage(page: &Page, key: &str, value: &str) -> Result<()> {
    let js = format!(
        "localStorage.setItem({}, {})",
        serde_json::to_string(key).unwrap_or_default(),
        serde_json::to_string(value).unwrap_or_default(),
    );
    page.evaluate(js.as_str())
        .await
        .map_err(|e| onecrawl_core::Error::Browser(format!("set_local_storage failed: {e}")))?;
    Ok(())
}

/// Clear all localStorage.
pub async fn clear_local_storage(page: &Page) -> Result<()> {
    page.evaluate("localStorage.clear()")
        .await
        .map_err(|e| onecrawl_core::Error::Browser(format!("clear_local_storage failed: {e}")))?;
    Ok(())
}

/// Get all sessionStorage contents as a JSON object.
pub async fn get_session_storage(page: &Page) -> Result<serde_json::Value> {
    let result = page
        .evaluate(
            r#"
            (() => {
                const obj = {};
                for (let i = 0; i < sessionStorage.length; i++) {
                    const k = sessionStorage.key(i);
                    obj[k] = sessionStorage.getItem(k);
                }
                return obj;
            })()
            "#,
        )
        .await
        .map_err(|e| onecrawl_core::Error::Browser(format!("get_session_storage failed: {e}")))?;

    let val: serde_json::Value = result.into_value().unwrap_or(serde_json::Value::Object(Default::default()));
    Ok(val)
}

/// Set a sessionStorage item.
pub async fn set_session_storage(page: &Page, key: &str, value: &str) -> Result<()> {
    let js = format!(
        "sessionStorage.setItem({}, {})",
        serde_json::to_string(key).unwrap_or_default(),
        serde_json::to_string(value).unwrap_or_default(),
    );
    page.evaluate(js.as_str())
        .await
        .map_err(|e| onecrawl_core::Error::Browser(format!("set_session_storage failed: {e}")))?;
    Ok(())
}

/// Clear all sessionStorage.
pub async fn clear_session_storage(page: &Page) -> Result<()> {
    page.evaluate("sessionStorage.clear()")
        .await
        .map_err(|e| onecrawl_core::Error::Browser(format!("clear_session_storage failed: {e}")))?;
    Ok(())
}

/// Get IndexedDB database names.
pub async fn get_indexeddb_databases(page: &Page) -> Result<Vec<String>> {
    let result = page
        .evaluate(
            r#"
            (async () => {
                try {
                    if (indexedDB.databases) {
                        const dbs = await indexedDB.databases();
                        return dbs.map(d => d.name || '');
                    }
                    return [];
                } catch(e) { return []; }
            })()
            "#,
        )
        .await
        .map_err(|e| onecrawl_core::Error::Browser(format!("get_indexeddb_databases failed: {e}")))?;

    let names: Vec<String> = match result.into_value() {
        Ok(v) => v,
        Err(_) => Vec::new(),
    };
    Ok(names)
}

/// Clear all site data (localStorage + sessionStorage + cookies + cache).
pub async fn clear_site_data(page: &Page) -> Result<()> {
    page.evaluate(
        r#"
        (async () => {
            localStorage.clear();
            sessionStorage.clear();
            document.cookie.split(';').forEach(c => {
                document.cookie = c.trim().split('=')[0] + '=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/';
            });
            if (caches && caches.keys) {
                const keys = await caches.keys();
                for (const k of keys) await caches.delete(k);
            }
        })()
        "#,
    )
    .await
    .map_err(|e| onecrawl_core::Error::Browser(format!("clear_site_data failed: {e}")))?;
    Ok(())
}
