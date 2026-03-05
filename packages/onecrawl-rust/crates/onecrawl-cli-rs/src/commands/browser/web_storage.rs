use colored::Colorize;
use super::helpers::{with_page};

// ---------------------------------------------------------------------------
// Web Storage
// ---------------------------------------------------------------------------

pub async fn web_storage_local_get() {
    with_page(|page| async move {
        let data = onecrawl_cdp::web_storage::get_local_storage(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&data).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn web_storage_local_set(key: &str, value: &str) {
    let k = key.to_string();
    let v = value.to_string();
    with_page(|page| async move {
        onecrawl_cdp::web_storage::set_local_storage(&page, &k, &v)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} localStorage['{}'] set", "✓".green(), k);
        Ok(())
    })
    .await;
}

pub async fn web_storage_local_clear() {
    with_page(|page| async move {
        onecrawl_cdp::web_storage::clear_local_storage(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} localStorage cleared", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn web_storage_session_get() {
    with_page(|page| async move {
        let data = onecrawl_cdp::web_storage::get_session_storage(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&data).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn web_storage_session_set(key: &str, value: &str) {
    let k = key.to_string();
    let v = value.to_string();
    with_page(|page| async move {
        onecrawl_cdp::web_storage::set_session_storage(&page, &k, &v)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} sessionStorage['{}'] set", "✓".green(), k);
        Ok(())
    })
    .await;
}

pub async fn web_storage_session_clear() {
    with_page(|page| async move {
        onecrawl_cdp::web_storage::clear_session_storage(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} sessionStorage cleared", "✓".green());
        Ok(())
    })
    .await;
}

pub async fn web_storage_indexeddb_list() {
    with_page(|page| async move {
        let names = onecrawl_cdp::web_storage::get_indexeddb_databases(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&names).unwrap_or_default()
        );
        Ok(())
    })
    .await;
}

pub async fn web_storage_clear_all() {
    with_page(|page| async move {
        onecrawl_cdp::web_storage::clear_site_data(&page)
            .await
            .map_err(|e| e.to_string())?;
        println!("{} All site data cleared", "✓".green());
        Ok(())
    })
    .await;
}
