use super::fingerprint::Fingerprint;

/// Generate the JavaScript init script that applies all stealth patches.
///
/// This script is injected into every page via `Page.addScriptToEvaluateOnNewDocument`.
/// It mirrors the TypeScript stealth.ts patches.
pub fn get_stealth_init_script(fp: &Fingerprint) -> String {
    let languages_json = serde_json::to_string(&fp.languages).unwrap();

    format!(
        r#"(() => {{
  // Canary: confirm this script executed
  window.__onecrawl_stealth = true;

  // === 0. document.hidden / visibilityState (headless indicator) ===
  try {{
    Object.defineProperty(document, 'hidden', {{ get: () => false, configurable: true }});
  }} catch(e) {{}}
  try {{
    Object.defineProperty(document, 'visibilityState', {{ get: () => 'visible', configurable: true }});
  }} catch(e) {{}}
  try {{
    document.dispatchEvent(new Event('visibilitychange'));
  }} catch(e) {{}}

  // === 1. navigator.webdriver ===
  try {{
    Object.defineProperty(navigator, 'webdriver', {{
      get: () => false,
      configurable: true,
    }});
  }} catch(e) {{}}

  // === 2. Chrome runtime mock ===
  try {{
    if (!window.chrome) window.chrome = {{}};
    if (!window.chrome.runtime) {{
      Object.defineProperty(window.chrome, 'runtime', {{
        value: {{
          connect: () => ({{ onMessage: {{ addListener: () => {{}}, removeListener: () => {{}} }}, postMessage: () => {{}}, disconnect: () => {{}} }}),
          sendMessage: (_msg, cb) => {{ if (cb) cb(); }},
        }},
        writable: true,
        configurable: true,
      }});
    }}
  }} catch(e) {{}}

  // === 3. Plugins mock ===
  try {{
    Object.defineProperty(navigator, 'plugins', {{
      get: () => {{
        const arr = [
          {{ name: 'Chrome PDF Plugin', filename: 'internal-pdf-viewer', description: 'Portable Document Format' }},
          {{ name: 'Chrome PDF Viewer', filename: 'mhjfbmdgcfjbbpaeojofohoefgiehjai', description: '' }},
          {{ name: 'Native Client', filename: 'internal-nacl-plugin', description: '' }},
          {{ name: 'PDF Viewer', filename: 'mhjfbmdgcfjbbpaeojofohoefgiehjai', description: '' }},
          {{ name: 'Google Slides', filename: 'internal-nacl-plugin', description: '' }},
        ];
        arr.item = (n) => arr[n] || null;
        arr.namedItem = (n) => arr.find(p => p.name === n) || null;
        arr.refresh = () => {{}};
        return arr;
      }},
      configurable: true,
    }});
  }} catch(e) {{}}

  // === 4. Languages ===
  try {{
    Object.defineProperty(navigator, 'languages', {{
      get: () => {languages_json},
      configurable: true,
    }});
    Object.defineProperty(navigator, 'language', {{
      get: () => '{language}',
      configurable: true,
    }});
  }} catch(e) {{}}

  // === 5. Platform ===
  try {{
    Object.defineProperty(navigator, 'platform', {{
      get: () => '{platform}',
      configurable: true,
    }});
  }} catch(e) {{}}

  // === 6. Hardware concurrency ===
  try {{
    Object.defineProperty(navigator, 'hardwareConcurrency', {{
      get: () => {hw_concurrency},
      configurable: true,
    }});
  }} catch(e) {{}}

  // === 7. Device memory ===
  try {{
    Object.defineProperty(navigator, 'deviceMemory', {{
      get: () => {device_memory},
      configurable: true,
    }});
  }} catch(e) {{}}

  // === 8. WebGL fingerprint ===
  try {{
    const origGetParameter = WebGLRenderingContext.prototype.getParameter;
    WebGLRenderingContext.prototype.getParameter = function(param) {{
      if (param === 0x9245) return '{webgl_vendor}';
      if (param === 0x9246) return '{webgl_renderer}';
      return origGetParameter.call(this, param);
    }};
    if (typeof WebGL2RenderingContext !== 'undefined') {{
      const origGetParameter2 = WebGL2RenderingContext.prototype.getParameter;
      WebGL2RenderingContext.prototype.getParameter = function(param) {{
        if (param === 0x9245) return '{webgl_vendor}';
        if (param === 0x9246) return '{webgl_renderer}';
        return origGetParameter2.call(this, param);
      }};
    }}
  }} catch(e) {{}}

  // === 9. Canvas fingerprint noise ===
  try {{
    const origToDataURL = HTMLCanvasElement.prototype.toDataURL;
    HTMLCanvasElement.prototype.toDataURL = function(type) {{
      const ctx = this.getContext('2d');
      if (ctx) {{
        const imgData = ctx.getImageData(0, 0, this.width > 0 ? this.width : 1, this.height > 0 ? this.height : 1);
        for (let i = 0; i < imgData.data.length; i += 4) {{
          imgData.data[i] = imgData.data[i] ^ (Math.random() > 0.5 ? 1 : 0);
        }}
        ctx.putImageData(imgData, 0, 0);
      }}
      return origToDataURL.apply(this, arguments);
    }};
  }} catch(e) {{}}

  // === 10. AudioContext fingerprint noise ===
  try {{
    if (typeof AudioContext !== 'undefined') {{
      const origCreateOscillator = AudioContext.prototype.createOscillator;
      AudioContext.prototype.createOscillator = function() {{
        const osc = origCreateOscillator.call(this);
        osc.frequency.value += (Math.random() - 0.5) * 0.01;
        return osc;
      }};
    }}
  }} catch(e) {{}}

  // === 11. Permissions mock ===
  try {{
    const origQuery = navigator.permissions?.query;
    if (origQuery) {{
      navigator.permissions.query = (params) => {{
        if (params.name === 'notifications') {{
          return Promise.resolve({{ state: 'prompt', onchange: null }});
        }}
        return origQuery.call(navigator.permissions, params);
      }};
    }}
  }} catch(e) {{}}

  // === 12. Window / screen dimensions (fix headless outerWidth=0, screen=800x600) ===
  try {{
    if (window.outerWidth === 0) {{
      Object.defineProperty(window, 'outerWidth', {{ get: () => {viewport_width}, configurable: true }});
    }}
  }} catch(e) {{}}
  try {{
    if (window.outerHeight === 0) {{
      Object.defineProperty(window, 'outerHeight', {{ get: () => {viewport_height} + 85, configurable: true }});
    }}
  }} catch(e) {{}}
  try {{
    Object.defineProperty(window.screen, 'width', {{ get: () => {viewport_width}, configurable: true }});
    Object.defineProperty(window.screen, 'height', {{ get: () => {viewport_height}, configurable: true }});
    Object.defineProperty(window.screen, 'availWidth', {{ get: () => {viewport_width}, configurable: true }});
    Object.defineProperty(window.screen, 'availHeight', {{ get: () => {viewport_height} - 40, configurable: true }});
  }} catch(e) {{}}

  // === 13. Console.debug filter ===
  try {{
    const origDebug = console.debug;
    console.debug = function() {{
      const args = Array.from(arguments);
      if (args.some(a => typeof a === 'string' && a.includes('Headless'))) return;
      return origDebug.apply(console, arguments);
    }};
  }} catch(e) {{}}
}})();"#,
        languages_json = languages_json,
        language = fp.language,
        platform = fp.platform,
        hw_concurrency = fp.hardware_concurrency,
        device_memory = fp.device_memory,
        webgl_vendor = fp.webgl_vendor,
        webgl_renderer = fp.webgl_renderer,
        viewport_width = fp.viewport_width,
        viewport_height = fp.viewport_height,
    )
}

/// Generate the CDP User-Agent override command parameters.
pub fn get_ua_override(fp: &Fingerprint) -> (String, String, String) {
    (
        fp.user_agent.clone(),
        fp.language.clone(),
        fp.platform.clone(),
    )
}

/// Register the stealth init script to run before every page's scripts in the browser session.
///
/// Uses `Page.addScriptToEvaluateOnNewDocument` which is persistent for the lifetime
/// of the browser session (survives navigations and new tabs).
pub async fn inject_persistent_stealth(
    page: &chromiumoxide::Page,
) -> onecrawl_core::Result<()> {
    use chromiumoxide::cdp::browser_protocol::page::AddScriptToEvaluateOnNewDocumentParams;
    use crate::emulation;

    let fp = super::fingerprint::generate_fingerprint();
    let script = get_stealth_init_script(&fp);

    page.execute(AddScriptToEvaluateOnNewDocumentParams::new(script))
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("addScriptToEvaluateOnNewDocument: {e}")))?;

    emulation::set_user_agent(page, &fp.user_agent)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stealth::generate_fingerprint;

    #[test]
    fn script_contains_all_patches() {
        let fp = generate_fingerprint();
        let script = get_stealth_init_script(&fp);

        assert!(script.contains("navigator.webdriver"));
        assert!(script.contains("chrome.runtime"));
        assert!(script.contains("'plugins'"));
        assert!(script.contains("'languages'"));
        assert!(script.contains("'platform'"));
        assert!(script.contains("hardwareConcurrency"));
        assert!(script.contains("deviceMemory"));
        assert!(script.contains("WebGLRenderingContext"));
        assert!(script.contains("HTMLCanvasElement"));
        assert!(script.contains("AudioContext"));
        assert!(script.contains("permissions.query"));
        assert!(script.contains("outerWidth"));
        assert!(script.contains("console.debug"));
    }

    #[test]
    fn script_uses_fingerprint_values() {
        let fp = generate_fingerprint();
        let script = get_stealth_init_script(&fp);

        assert!(script.contains(&fp.language));
        assert!(script.contains(&fp.platform));
        assert!(script.contains(&fp.webgl_vendor));
        assert!(script.contains(&fp.webgl_renderer));
        assert!(script.contains(&fp.hardware_concurrency.to_string()));
    }

    #[test]
    fn ua_override_matches_fingerprint() {
        let fp = generate_fingerprint();
        let (ua, lang, platform) = get_ua_override(&fp);
        assert_eq!(ua, fp.user_agent);
        assert_eq!(lang, fp.language);
        assert_eq!(platform, fp.platform);
    }
}
