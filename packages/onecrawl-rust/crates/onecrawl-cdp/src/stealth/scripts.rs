use super::fingerprint::Fingerprint;

/// Generate the JavaScript init script that applies all stealth patches.
///
/// This script is injected into every page via `Page.addScriptToEvaluateOnNewDocument`.
/// It mirrors the TypeScript stealth.ts patches.
pub fn get_stealth_init_script(fp: &Fingerprint) -> String {
    let languages_json = serde_json::to_string(&fp.languages).unwrap();

    format!(
        r#"(() => {{
  // === 1. navigator.webdriver ===
  Object.defineProperty(navigator, 'webdriver', {{
    get: () => false,
    configurable: true,
  }});

  // === 2. Chrome runtime mock ===
  if (!window.chrome) window.chrome = {{}};
  if (!window.chrome.runtime) {{
    window.chrome.runtime = {{
      connect: () => ({{ onMessage: {{ addListener: () => {{}}, removeListener: () => {{}} }}, postMessage: () => {{}}, disconnect: () => {{}} }}),
      sendMessage: (_msg, cb) => {{ if (cb) cb(); }},
    }};
  }}

  // === 3. Plugins mock ===
  Object.defineProperty(navigator, 'plugins', {{
    get: () => [
      {{ name: 'Chrome PDF Plugin', filename: 'internal-pdf-viewer', description: 'Portable Document Format' }},
      {{ name: 'Chrome PDF Viewer', filename: 'mhjfbmdgcfjbbpaeojofohoefgiehjai', description: '' }},
      {{ name: 'Native Client', filename: 'internal-nacl-plugin', description: '' }},
    ],
  }});

  // === 4. Languages ===
  Object.defineProperty(navigator, 'languages', {{
    get: () => {languages_json},
  }});
  Object.defineProperty(navigator, 'language', {{
    get: () => '{language}',
  }});

  // === 5. Platform ===
  Object.defineProperty(navigator, 'platform', {{
    get: () => '{platform}',
  }});

  // === 6. Hardware concurrency ===
  Object.defineProperty(navigator, 'hardwareConcurrency', {{
    get: () => {hw_concurrency},
  }});

  // === 7. Device memory ===
  Object.defineProperty(navigator, 'deviceMemory', {{
    get: () => {device_memory},
  }});

  // === 8. WebGL fingerprint ===
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

  // === 9. Canvas fingerprint noise ===
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

  // === 10. AudioContext fingerprint noise ===
  if (typeof AudioContext !== 'undefined') {{
    const origCreateOscillator = AudioContext.prototype.createOscillator;
    AudioContext.prototype.createOscillator = function() {{
      const osc = origCreateOscillator.call(this);
      osc.frequency.value += (Math.random() - 0.5) * 0.01;
      return osc;
    }};
  }}

  // === 11. Permissions mock ===
  const origQuery = navigator.permissions?.query;
  if (origQuery) {{
    navigator.permissions.query = (params) => {{
      if (params.name === 'notifications') {{
        return Promise.resolve({{ state: 'prompt', onchange: null }});
      }}
      return origQuery.call(navigator.permissions, params);
    }};
  }}

  // === 12. Window dimensions (fix headless outerWidth=0) ===
  if (window.outerWidth === 0) {{
    Object.defineProperty(window, 'outerWidth', {{ get: () => {viewport_width} }});
    Object.defineProperty(window, 'outerHeight', {{ get: () => {viewport_height} }});
  }}

  // === 13. Console.debug filter ===
  const origDebug = console.debug;
  console.debug = function() {{
    const args = Array.from(arguments);
    if (args.some(a => typeof a === 'string' && a.includes('Headless'))) return;
    return origDebug.apply(console, arguments);
  }};
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
