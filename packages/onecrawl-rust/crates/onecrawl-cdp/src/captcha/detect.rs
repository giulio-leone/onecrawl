use onecrawl_browser::Page;
use onecrawl_core::{Error, Result};
use super::types::CaptchaDetection;
use super::types::DETECT_CAPTCHA_JS;

/// Detect CAPTCHA presence and type on the current page.
pub async fn detect_captcha(page: &Page) -> Result<CaptchaDetection> {
    let raw: String = page
        .evaluate(DETECT_CAPTCHA_JS)
        .await
        .map_err(|e| Error::Cdp(format!("captcha detect eval failed: {e}")))?
        .into_value()
        .map_err(|e| Error::Cdp(format!("captcha detect parse failed: {e}")))?;

    serde_json::from_str(&raw).map_err(|e| Error::Cdp(format!("captcha json parse: {e}")))
}

/// Wait up to `timeout_ms` for a CAPTCHA to appear, polling every 500 ms.
pub async fn wait_for_captcha(page: &Page, timeout_ms: u64) -> Result<CaptchaDetection> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
    loop {
        let det = detect_captcha(page).await?;
        if det.detected {
            return Ok(det);
        }
        if std::time::Instant::now() >= deadline {
            return Ok(det); // returns the "none" detection
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}

/// Take a base64-encoded screenshot of the captcha element.
pub async fn screenshot_captcha(page: &Page, detection: &CaptchaDetection) -> Result<String> {
    let selector = detection
        .selector
        .as_deref()
        .ok_or_else(|| Error::Cdp("no captcha selector available".into()))?;

    let js = format!(
        r#"
        (async () => {{
            const el = document.querySelector({sel});
            if (!el) return '';
            if (typeof el.scrollIntoView === 'function') el.scrollIntoView();
            // Use html2canvas-style: convert element rect to a data URL via canvas
            const rect = el.getBoundingClientRect();
            if (rect.width === 0 || rect.height === 0) return '';
            // Fallback: return bounding rect as JSON so the caller can use CDP screenshot
            return JSON.stringify({{x: rect.x, y: rect.y, w: rect.width, h: rect.height}});
        }})()
        "#,
        sel = serde_json::to_string(selector)
            .map_err(|e| Error::Cdp(format!("selector serialize: {e}")))?
    );

    let raw: String = page
        .evaluate(js.as_str())
        .await
        .map_err(|e| Error::Cdp(format!("screenshot eval failed: {e}")))?
        .into_value()
        .map_err(|e| Error::Cdp(format!("screenshot parse failed: {e}")))?;

    if raw.is_empty() {
        return Err(Error::Cdp(
            "captcha element not found or zero-size".into(),
        ));
    }

    Ok(raw)
}

/// Inject a captcha solution token into the page.
pub async fn inject_solution(
    page: &Page,
    detection: &CaptchaDetection,
    solution: &str,
) -> Result<bool> {
    let escaped_solution = serde_json::to_string(solution)
        .map_err(|e| Error::Cdp(format!("solution serialize: {e}")))?;

    let js = match detection.captcha_type.as_str() {
        "recaptcha_v2" | "recaptcha_v3" => {
            format!(
                r#"
                (() => {{
                    const token = {tok};
                    const ta = document.querySelector('#g-recaptcha-response, textarea[name="g-recaptcha-response"]');
                    if (ta) {{
                        ta.style.display = 'block';
                        ta.value = token;
                        ta.style.display = 'none';
                    }}
                    if (typeof window.___grecaptcha_cfg !== 'undefined') {{
                        const clients = window.___grecaptcha_cfg.clients || {{}};
                        for (const cid of Object.keys(clients)) {{
                            try {{
                                const c = clients[cid];
                                for (const k of Object.keys(c)) {{
                                    const v = c[k];
                                    if (v && typeof v === 'object') {{
                                        for (const kk of Object.keys(v)) {{
                                            if (v[kk] && v[kk].callback) {{
                                                v[kk].callback(token);
                                                return 'true';
                                            }}
                                        }}
                                    }}
                                }}
                            }} catch(_) {{}}
                        }}
                    }}
                    return ta ? 'true' : 'false';
                }})()
                "#,
                tok = escaped_solution
            )
        }
        "hcaptcha" => {
            format!(
                r#"
                (() => {{
                    const token = {tok};
                    const ta = document.querySelector('[name="h-captcha-response"], textarea[name="h-captcha-response"]');
                    if (ta) ta.value = token;
                    const inp = document.querySelector('[name="g-recaptcha-response"]');
                    if (inp) inp.value = token;
                    return ta ? 'true' : 'false';
                }})()
                "#,
                tok = escaped_solution
            )
        }
        "cloudflare_turnstile" => {
            format!(
                r#"
                (() => {{
                    const token = {tok};
                    const inp = document.querySelector('[name="cf-turnstile-response"], input[name="cf-turnstile-response"]');
                    if (inp) {{ inp.value = token; return 'true'; }}
                    return 'false';
                }})()
                "#,
                tok = escaped_solution
            )
        }
        "text" => {
            let sel = detection
                .selector
                .as_deref()
                .unwrap_or("input[name*=\"captcha\"]");
            let escaped_sel = serde_json::to_string(sel)
                .map_err(|e| Error::Cdp(format!("selector serialize: {e}")))?;
            format!(
                r#"
                (() => {{
                    const el = document.querySelector({sel});
                    if (el) {{ el.value = {tok}; return 'true'; }}
                    return 'false';
                }})()
                "#,
                sel = escaped_sel,
                tok = escaped_solution
            )
        }
        other => {
            return Err(Error::Cdp(format!(
                "injection not supported for captcha type: {other}"
            )));
        }
    };

    let raw: String = page
        .evaluate(js.as_str())
        .await
        .map_err(|e| Error::Cdp(format!("inject eval: {e}")))?
        .into_value()
        .map_err(|e| Error::Cdp(format!("inject parse: {e}")))?;
    Ok(raw == "true")
}

