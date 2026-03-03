//! Smart Form Filler — automatic form detection, analysis, and filling.

use chromiumoxide::Page;
use onecrawl_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Describes a single form field detected on the page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    pub selector: String,
    pub name: String,
    /// "text", "email", "password", "select", "checkbox", "radio", "textarea", "file", "hidden", "submit"
    pub field_type: String,
    pub label: String,
    pub placeholder: String,
    pub required: bool,
    pub value: String,
    /// Populated for select/radio fields.
    pub options: Vec<String>,
}

/// Metadata for a form detected on the page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormInfo {
    pub action: String,
    pub method: String,
    pub selector: String,
    pub fields: Vec<FormField>,
    pub submit_selector: Option<String>,
}

/// Result of a fill operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillResult {
    pub filled: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

// ── JS snippets ───────────────────────────────────────────────────

const DETECT_FORMS_JS: &str = r#"
(() => {
    const forms = document.querySelectorAll('form');
    const result = [];
    forms.forEach((form, fi) => {
        const formSel = form.id ? '#' + form.id : 'form:nth-of-type(' + (fi + 1) + ')';
        const fields = [];
        const inputs = form.querySelectorAll('input, select, textarea');
        inputs.forEach((el, ei) => {
            const tag = el.tagName.toLowerCase();
            let ft = tag === 'select' ? 'select' : tag === 'textarea' ? 'textarea' : (el.type || 'text');
            const name = el.name || el.id || '';
            const id = el.id || '';
            let label = '';
            if (id) {
                const lbl = document.querySelector('label[for="' + id + '"]');
                if (lbl) label = lbl.textContent.trim();
            }
            if (!label) {
                const parent = el.closest('label');
                if (parent) label = parent.textContent.trim();
            }
            const placeholder = el.placeholder || '';
            const required = el.required || el.hasAttribute('required');
            const value = el.value || '';
            let options = [];
            if (tag === 'select') {
                options = Array.from(el.options).map(o => o.value);
            }
            if (tag === 'input' && ft === 'radio') {
                options = Array.from(form.querySelectorAll('input[name="' + name + '"]')).map(r => r.value);
            }
            const elSel = el.id ? '#' + el.id
                : el.name ? formSel + ' [name="' + el.name + '"]'
                : formSel + ' ' + tag + ':nth-of-type(' + (ei + 1) + ')';
            fields.push({
                selector: elSel,
                name: name,
                field_type: ft,
                label: label,
                placeholder: placeholder,
                required: required,
                value: value,
                options: options,
            });
        });
        let submitSel = null;
        const submitBtn = form.querySelector('button[type="submit"], input[type="submit"]');
        if (submitBtn) {
            submitSel = submitBtn.id ? '#' + submitBtn.id : formSel + ' [type="submit"]';
        }
        result.push({
            action: form.action || '',
            method: (form.method || 'GET').toUpperCase(),
            selector: formSel,
            fields: fields,
            submit_selector: submitSel,
        });
    });
    return JSON.stringify(result);
})()
"#;

// ── public API ────────────────────────────────────────────────────

/// Detect all forms on the current page with their fields.
pub async fn detect_forms(page: &Page) -> Result<Vec<FormInfo>> {
    let raw: String = page
        .evaluate(DETECT_FORMS_JS)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(e.to_string()))?
        .into_value()
        .map_err(|e| onecrawl_core::Error::Cdp(e.to_string()))?;
    let forms: Vec<FormInfo> =
        serde_json::from_str(&raw).map_err(|e| onecrawl_core::Error::Cdp(e.to_string()))?;
    Ok(forms)
}

/// Fill form fields using an explicit `name -> value` map.
pub async fn fill_form(
    page: &Page,
    form_selector: &str,
    values: &HashMap<String, String>,
) -> Result<FillResult> {
    let forms = detect_forms(page).await?;
    let form = forms
        .iter()
        .find(|f| f.selector == form_selector)
        .ok_or_else(|| {
            onecrawl_core::Error::Cdp(format!("form not found: {form_selector}"))
        })?;

    let mut filled: usize = 0;
    let mut skipped: usize = 0;
    let mut errors: Vec<String> = Vec::new();

    for field in &form.fields {
        if field.field_type == "submit" || field.field_type == "hidden" {
            skipped += 1;
            continue;
        }
        let val = values
            .get(&field.name)
            .or_else(|| values.get(&field.selector));
        match val {
            Some(v) => match set_field_value(page, field, v).await {
                Ok(()) => filled += 1,
                Err(e) => errors.push(format!("{}: {e}", field.selector)),
            },
            None => skipped += 1,
        }
    }

    Ok(FillResult {
        filled,
        skipped,
        errors,
    })
}

/// Smart auto-fill: match profile keys to field names/labels via fuzzy matching.
pub async fn auto_fill(
    page: &Page,
    form_selector: &str,
    profile: &HashMap<String, String>,
) -> Result<FillResult> {
    let forms = detect_forms(page).await?;
    let form = forms
        .iter()
        .find(|f| f.selector == form_selector)
        .ok_or_else(|| {
            onecrawl_core::Error::Cdp(format!("form not found: {form_selector}"))
        })?;

    let mut filled: usize = 0;
    let mut skipped: usize = 0;
    let mut errors: Vec<String> = Vec::new();

    for field in &form.fields {
        if field.field_type == "submit" || field.field_type == "hidden" {
            skipped += 1;
            continue;
        }
        if let Some(val) = match_profile_key(field, profile) {
            match set_field_value(page, field, &val).await {
                Ok(()) => filled += 1,
                Err(e) => errors.push(format!("{}: {e}", field.selector)),
            }
        } else {
            skipped += 1;
        }
    }

    Ok(FillResult {
        filled,
        skipped,
        errors,
    })
}

/// Submit a form by clicking the submit button or calling form.submit().
pub async fn submit_form(page: &Page, form_selector: &str) -> Result<()> {
    let js = format!(
        r#"(() => {{
            const form = document.querySelector('{sel}');
            if (!form) return 'not_found';
            const btn = form.querySelector('button[type="submit"], input[type="submit"]');
            if (btn) {{ btn.click(); return 'clicked'; }}
            form.submit();
            return 'submitted';
        }})()"#,
        sel = form_selector.replace('\'', "\\'"),
    );
    let res: String = page
        .evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(e.to_string()))?
        .into_value()
        .map_err(|e| onecrawl_core::Error::Cdp(e.to_string()))?;
    if res == "not_found" {
        return Err(onecrawl_core::Error::Cdp(format!(
            "form not found: {form_selector}"
        )));
    }
    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────

async fn set_field_value(page: &Page, field: &FormField, value: &str) -> Result<()> {
    let sel = &field.selector;
    let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");
    let js = match field.field_type.as_str() {
        "checkbox" => format!(
            r#"(() => {{
                const el = document.querySelector('{sel}');
                if (!el) return false;
                el.checked = {val};
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return true;
            }})()"#,
            sel = sel.replace('\'', "\\'"),
            val = if escaped == "true" || escaped == "1" {
                "true"
            } else {
                "false"
            },
        ),
        "select" => format!(
            r#"(() => {{
                const el = document.querySelector('{sel}');
                if (!el) return false;
                el.value = '{val}';
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return true;
            }})()"#,
            sel = sel.replace('\'', "\\'"),
            val = escaped,
        ),
        "radio" => format!(
            r#"(() => {{
                const el = document.querySelector('{sel}[value="{val}"]') ||
                           document.querySelector('{sel}');
                if (!el) return false;
                el.checked = true;
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return true;
            }})()"#,
            sel = sel.replace('\'', "\\'"),
            val = escaped,
        ),
        _ => format!(
            r#"(() => {{
                const el = document.querySelector('{sel}');
                if (!el) return false;
                el.value = '{val}';
                el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return true;
            }})()"#,
            sel = sel.replace('\'', "\\'"),
            val = escaped,
        ),
    };
    let ok: bool = page
        .evaluate(js)
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(e.to_string()))?
        .into_value()
        .map_err(|e| onecrawl_core::Error::Cdp(e.to_string()))?;
    if !ok {
        return Err(onecrawl_core::Error::Cdp(format!(
            "element not found: {sel}"
        )));
    }
    Ok(())
}

/// Match a profile key to a form field using fuzzy (lowercase-contains) matching.
fn match_profile_key(field: &FormField, profile: &HashMap<String, String>) -> Option<String> {
    let name_lower = field.name.to_lowercase();
    let label_lower = field.label.to_lowercase();
    let placeholder_lower = field.placeholder.to_lowercase();
    let type_lower = field.field_type.to_lowercase();

    for (key, val) in profile {
        let k = key.to_lowercase();
        if name_lower.contains(&k)
            || label_lower.contains(&k)
            || placeholder_lower.contains(&k)
            || k.contains(&name_lower)
        {
            return Some(val.clone());
        }
        // Match by field type (e.g. "email" key matches email input)
        if type_lower == k {
            return Some(val.clone());
        }
    }
    None
}
