use super::fingerprint::Fingerprint;

/// Generate the JavaScript init script that applies all stealth patches.
///
/// This script is injected into every page via `Page.addScriptToEvaluateOnNewDocument`.
/// It mirrors the TypeScript stealth.ts patches.
pub fn get_stealth_init_script(fp: &Fingerprint) -> String {
    format!(
        r#"(() => {{
  // Idempotency guard: if this script already ran in this JS context, skip.
  // Required because Page.addScriptToEvaluateOnNewDocument may be registered
  // multiple times (once per CLI command that calls connect_to_session).
  // Without this guard, each re-run captures the PREVIOUS _ov as _nativeFnToString,
  // building a recursion chain that overflows the call stack.
  if (window.__onecrawl_stealth) return;
  window.__onecrawl_stealth = true;

  // === Preamble: mask all JS wrapper functions from Function.prototype.toString detection ===
  // Self-registration ensures creepjs does not detect hasToStringProxy.
  // All wrapped getters are registered so pixelscan sees native code strings.
  const _nativeFnToString = Function.prototype.toString;
  const _nativeWraps = new WeakMap();
  // Cross-frame registry: share [patched_fn, native_fn] pairs across realms.
  // Stored as non-enumerable Symbol property on Function.prototype so it is
  // invisible to string-key enumerations (Object.keys / getOwnPropertyNames)
  // while still accessible from any same-origin iframe via parent.Function.prototype.
  const _regSym = Symbol.for('__onecrawl_r');
  if (!Function.prototype[_regSym]) {{
    Object.defineProperty(Function.prototype, _regSym, {{
      value: [], writable: false, configurable: false, enumerable: false,
    }});
  }}
  const _patchReg = Function.prototype[_regSym];
  // Helper: register patched fn locally AND in the cross-frame list.
  const _reg = (patched, native) => {{ _nativeWraps.set(patched, native); _patchReg.push([patched, native]); }};
  (function() {{
    const _ov = {{toString() {{
      const n = _nativeWraps.get(this);
      if (n !== undefined) return _nativeFnToString.call(n);
      return _nativeFnToString.call(this);
    }}}}.toString;
    Object.defineProperty(Function.prototype, 'toString', {{
      value: _ov, writable: true, configurable: true, enumerable: false,
    }});
    // Self-mask: Function.prototype.toString.call(Function.prototype.toString) → native string
    _reg(Function.prototype.toString, _nativeFnToString);
    // Cross-realm fix: when running inside an iframe, sync all ancestor window patched
    // functions into this realm's _nativeWraps.  Without this, gW's cross-realm check
    // "scope=iframeWin, apiFunction=mainWin._wdFn" would expose wrapper source code.
    try {{
      let _p = window.parent;
      while (_p !== window) {{
        const _pr = _p.Function && _p.Function.prototype && _p.Function.prototype[_regSym];
        if (_pr) _pr.forEach(([fn, native]) => _nativeWraps.set(fn, native));
        if (_p.parent === _p) break;
        _p = _p.parent;
      }}
    }} catch(_e) {{}}
  }})();
  // Helper: get native getter from a prototype object
  const _nativeGet = (proto, prop) => {{
    try {{ const d = Object.getOwnPropertyDescriptor(proto, prop); return (d && d.get) || null; }}
    catch(e) {{ return null; }}
  }};

  // === 0. document.hidden / visibilityState (headless indicator) ===
  try {{
    const _origHidGetter = _nativeGet(Document.prototype, 'hidden');
    const _hidGet = Object.getOwnPropertyDescriptor({{get hidden(){{return false;}}}}, 'hidden').get;
    if (_origHidGetter) _reg(_hidGet, _origHidGetter);
    Object.defineProperty(document, 'hidden', {{ get: _hidGet, configurable: true }});
  }} catch(e) {{}}
  try {{
    const _origVSGetter = _nativeGet(Document.prototype, 'visibilityState');
    const _vsGet = Object.getOwnPropertyDescriptor({{get visibilityState(){{return 'visible';}}}}, 'visibilityState').get;
    if (_origVSGetter) _reg(_vsGet, _origVSGetter);
    Object.defineProperty(document, 'visibilityState', {{ get: _vsGet, configurable: true }});
  }} catch(e) {{}}
  try {{
    document.dispatchEvent(new Event('visibilitychange'));
  }} catch(e) {{}}

  // === 1. navigator.webdriver ===
  // setAutomationOverride(false) installs a native getter that returns false.
  // Real non-automated Chrome has no webdriver property (returns undefined).
  // Wrap the native getter so it returns undefined while still looking native
  // to Function.prototype.toString (via _nativeWraps).
  try {{
    const _nativeWdGet = _nativeGet(Navigator.prototype, 'webdriver');
    if (_nativeWdGet) {{
      const _wdWrapper = {{get webdriver() {{ void _nativeWdGet.call(this); return false; }}}};
      const _wdFn = Object.getOwnPropertyDescriptor(_wdWrapper, 'webdriver').get;
      _reg(_wdFn, _nativeWdGet);
      Object.defineProperty(Navigator.prototype, 'webdriver', {{
        get: _wdFn, configurable: true, enumerable: true,
      }});
    }}
  }} catch(e) {{}}

  // === 2. Chrome runtime mock (only when not already native to avoid overriding real methods) ===
  try {{
    if (!window.chrome) window.chrome = {{}};
    // Only install mock if window.chrome.runtime is absent or has no native connect.
    // In a real Chrome session, window.chrome.runtime.connect is a native function —
    // overriding it with JS would be detectable by hasToStringProxy checks.
    const _crRuntime = window.chrome.runtime;
    const _crConnectIsNative = _crRuntime && _crRuntime.connect &&
      Function.prototype.toString.call(_crRuntime.connect).includes('[native code]');
    if (!_crConnectIsNative) {{
      Object.defineProperty(window.chrome, 'runtime', {{
        value: {{ id: undefined }},
        writable: true,
        configurable: true,
      }});
    }}
  }} catch(e) {{}}

  // === 3. Plugins mock ===
  try {{
    const _origPluginsGetter = _nativeGet(Navigator.prototype, 'plugins');
    const _pluginArrayProto = _origPluginsGetter ? (() => {{
      try {{ return Object.getPrototypeOf(_origPluginsGetter.call(navigator)); }} catch(e) {{ return null; }}
    }})() : null;
    const _pluginsGetObj = {{get plugins() {{
      const arr = [
        {{ name: 'Chrome PDF Plugin', filename: 'internal-pdf-viewer', description: 'Portable Document Format', [Symbol.toStringTag]: 'Plugin' }},
        {{ name: 'Chrome PDF Viewer', filename: 'mhjfbmdgcfjbbpaeojofohoefgiehjai', description: '', [Symbol.toStringTag]: 'Plugin' }},
        {{ name: 'Native Client', filename: 'internal-nacl-plugin', description: '', [Symbol.toStringTag]: 'Plugin' }},
        {{ name: 'PDF Viewer', filename: 'mhjfbmdgcfjbbpaeojofohoefgiehjai', description: '', [Symbol.toStringTag]: 'Plugin' }},
        {{ name: 'Google Slides', filename: 'internal-nacl-plugin', description: '', [Symbol.toStringTag]: 'Plugin' }},
      ];
      arr.item = (n) => arr[n] || null;
      arr.namedItem = (n) => arr.find(p => p.name === n) || null;
      arr.refresh = () => {{}};
      if (_pluginArrayProto) try {{ Object.setPrototypeOf(arr, _pluginArrayProto); }} catch(e) {{}}
      return arr;
    }}}};
    const _pluginsGet = Object.getOwnPropertyDescriptor(_pluginsGetObj, 'plugins').get;
    if (_origPluginsGetter) _reg(_pluginsGet, _origPluginsGetter);
    Object.defineProperty(Navigator.prototype, 'plugins', {{ get: _pluginsGet, configurable: true }});
  }} catch(e) {{}}

  // === 4. navigator.language / navigator.languages ===
  // Set natively via CDP Emulation.setUserAgentOverride(acceptLanguage: ...) — no JS own-property
  // override needed, so no detectable languages mutation on navigator instance.

  // === 5. Platform ===
  try {{
    const _origPlatformGetter = _nativeGet(Navigator.prototype, 'platform');
    const _platformGet = Object.getOwnPropertyDescriptor({{get platform(){{return '{platform}';}}}}, 'platform').get;
    if (_origPlatformGetter) _reg(_platformGet, _origPlatformGetter);
    Object.defineProperty(Navigator.prototype, 'platform', {{ get: _platformGet, configurable: true }});
  }} catch(e) {{}}

  // === 6. Hardware concurrency ===
  try {{
    const _origHWCGetter = _nativeGet(Navigator.prototype, 'hardwareConcurrency');
    const _hwcGet = Object.getOwnPropertyDescriptor({{get hardwareConcurrency(){{return {hw_concurrency};}}}}, 'hardwareConcurrency').get;
    if (_origHWCGetter) _reg(_hwcGet, _origHWCGetter);
    Object.defineProperty(Navigator.prototype, 'hardwareConcurrency', {{ get: _hwcGet, configurable: true }});
  }} catch(e) {{}}

  // === 7. Device memory ===
  try {{
    const _origDMGetter = _nativeGet(Navigator.prototype, 'deviceMemory');
    const _dmGet = Object.getOwnPropertyDescriptor({{get deviceMemory(){{return {device_memory};}}}}, 'deviceMemory').get;
    if (_origDMGetter) _reg(_dmGet, _origDMGetter);
    Object.defineProperty(Navigator.prototype, 'deviceMemory', {{ get: _dmGet, configurable: true }});
  }} catch(e) {{}}

  // === 8. WebGL fingerprint (no vendor/renderer override — would mismatch worker scope) ===
  // WebGL params 0x9245/0x9246 are intentionally NOT overridden to keep
  // main-thread and OffscreenCanvas-in-worker consistent (prevents hasBadWebGL detection).

  // === 9. Canvas fingerprint noise — REMOVED (detectable via hasToStringProxy) ===
  // HTMLCanvasElement.prototype.toDataURL override is not used; native canvas is correct for stealth.

  // === 10. AudioContext fingerprint noise — REMOVED (detectable via hasToStringProxy) ===
  // AudioContext.prototype.createOscillator override is not used; native audio is correct for stealth.

  // === 11. Permissions mock (register in _nativeWraps to avoid hasToStringProxy detection) ===
  try {{
    const _origQuery = navigator.permissions && navigator.permissions.query;
    if (_origQuery && Function.prototype.toString.call(_origQuery).includes('[native code]')) {{
      const _permWrapper = {{
        query(params) {{
          if (params.name === 'notifications') {{
            return Promise.resolve({{ state: 'prompt', onchange: null }});
          }}
          return _origQuery.call(navigator.permissions, params);
        }}
      }};
      const _queryFn = _permWrapper.query;
      _reg(_queryFn, _origQuery);
      navigator.permissions.query = _queryFn;
    }}
  }} catch(e) {{}}

  // === 12. Window / screen dimensions (fix headless outerWidth=0, screen=800x600) ===
  try {{
    if (window.outerWidth === 0) {{
      const _origOWGetter = _nativeGet(window, 'outerWidth');
      const _owGet = Object.getOwnPropertyDescriptor({{get outerWidth(){{return {viewport_width};}}}}, 'outerWidth').get;
      if (_origOWGetter) _reg(_owGet, _origOWGetter);
      Object.defineProperty(window, 'outerWidth', {{ get: _owGet, configurable: true }});
    }}
  }} catch(e) {{}}
  try {{
    if (window.outerHeight === 0) {{
      const _origOHGetter = _nativeGet(window, 'outerHeight');
      const _ohGet = Object.getOwnPropertyDescriptor({{get outerHeight(){{return {viewport_height} + 85;}}}}, 'outerHeight').get;
      if (_origOHGetter) _reg(_ohGet, _origOHGetter);
      Object.defineProperty(window, 'outerHeight', {{ get: _ohGet, configurable: true }});
    }}
  }} catch(e) {{}}
  try {{
    const _origSW = _nativeGet(Screen.prototype, 'width');
    const _origSH = _nativeGet(Screen.prototype, 'height');
    const _origSAW = _nativeGet(Screen.prototype, 'availWidth');
    const _origSAH = _nativeGet(Screen.prototype, 'availHeight');
    const _swGet = Object.getOwnPropertyDescriptor({{get width(){{return {viewport_width};}}}}, 'width').get;
    const _shGet = Object.getOwnPropertyDescriptor({{get height(){{return {viewport_height};}}}}, 'height').get;
    const _sawGet = Object.getOwnPropertyDescriptor({{get availWidth(){{return {viewport_width};}}}}, 'availWidth').get;
    const _sahGet = Object.getOwnPropertyDescriptor({{get availHeight(){{return {viewport_height} - 40;}}}}, 'availHeight').get;
    if (_origSW) _reg(_swGet, _origSW);
    if (_origSH) _reg(_shGet, _origSH);
    if (_origSAW) _reg(_sawGet, _origSAW);
    if (_origSAH) _reg(_sahGet, _origSAH);
    Object.defineProperty(Screen.prototype, 'width', {{ get: _swGet, configurable: true }});
    Object.defineProperty(Screen.prototype, 'height', {{ get: _shGet, configurable: true }});
    Object.defineProperty(Screen.prototype, 'availWidth', {{ get: _sawGet, configurable: true }});
    Object.defineProperty(Screen.prototype, 'availHeight', {{ get: _sahGet, configurable: true }});
  }} catch(e) {{}}

  // === 13. Console.debug filter — REMOVED (detectable via hasToStringProxy) ===
  // console.debug override is not used; native console is correct for stealth.

  // === 14. navigator.userAgentData patch (fix uaDataIsBlank) ===
  try {{
    const _origUADGetter = _nativeGet(Navigator.prototype, 'userAgentData');
    const _ua = navigator.userAgent;
    const _platform = _ua.includes('Macintosh') || _ua.includes('Mac OS X') ? 'macOS' :
                      _ua.includes('Windows') ? 'Windows' : 'Linux';
    const _chromeVer = (_ua.match(/Chrome\/(\d+)/) || [])[1] || '134';
    const _brands = [
      {{ brand: 'Not)A;Brand', version: '99' }},
      {{ brand: 'Google Chrome', version: _chromeVer }},
      {{ brand: 'Chromium', version: _chromeVer }},
    ];
    const _uaData = {{
      brands: _brands,
      mobile: false,
      platform: _platform,
      getHighEntropyValues: async function(hints) {{
        const r = {{}};
        if (hints.includes('platform')) r.platform = _platform;
        if (hints.includes('brands')) r.brands = _brands;
        if (hints.includes('mobile')) r.mobile = false;
        if (hints.includes('uaFullVersion')) r.uaFullVersion = _chromeVer + '.0.0.0';
        if (hints.includes('fullVersionList')) r.fullVersionList = _brands.map(b => ({{ brand: b.brand, version: b.version + '.0.0.0' }}));
        return r;
      }},
      toJSON: function() {{ return {{ brands: _brands, mobile: false, platform: _platform }}; }},
    }};
    const _uadGetObj = {{get userAgentData(){{return _uaData;}}}};
    const _uadGet = Object.getOwnPropertyDescriptor(_uadGetObj, 'userAgentData').get;
    if (_origUADGetter) _reg(_uadGet, _origUADGetter);
    Object.defineProperty(Navigator.prototype, 'userAgentData', {{
      get: _uadGet,
      configurable: true,
    }});
  }} catch(e) {{}}

  // === 15. Fix hasKnownBgColor (ActiveText system color) ===
  // Use method shorthand so .arguments throws and no .prototype property (avoids hasToStringProxy).
  // Registered in _nativeWraps so Function.prototype.toString.call(_wrapGCS) returns native.
  try {{
    const _origGCS = window.getComputedStyle;
    const _gcsObj = {{
      getComputedStyle(el, pseudo) {{
        const style = _origGCS.call(window, el, pseudo);
        try {{
          if (el && el.style && el.style.backgroundColor.toLowerCase() === 'activetext') {{
            return new Proxy(style, {{
              get(t, p, r) {{
                if (p === 'backgroundColor') return 'rgb(0, 102, 204)';
                const v = Reflect.get(t, p, r);
                return typeof v === 'function' ? v.bind(t) : v;
              }}
            }});
          }}
        }} catch(e) {{}}
        return style;
      }}
    }};
    const _wrapGCS = _gcsObj.getComputedStyle;
    _reg(_wrapGCS, _origGCS);
    Object.defineProperty(window, 'getComputedStyle', {{
      value: _wrapGCS,
      configurable: true,
      writable: true,
    }});
  }} catch(e) {{}}
}})();"#,
        platform = fp.platform,
        hw_concurrency = fp.hardware_concurrency,
        device_memory = fp.device_memory,
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
///
/// `real_ua` – the actual browser User-Agent string (from `Browser.getVersion`). When provided
/// the fingerprint UA is set to match the real Chrome version, eliminating the version-mismatch
/// between the main page context (overridden) and Worker contexts (unpatched, reports real UA).
pub async fn inject_persistent_stealth(
    page: &chromiumoxide::Page,
    real_ua: Option<&str>,
) -> onecrawl_core::Result<()> {
    use chromiumoxide::cdp::browser_protocol::page::AddScriptToEvaluateOnNewDocumentParams;
    use chromiumoxide::cdp::browser_protocol::emulation::SetAutomationOverrideParams;
    use chromiumoxide::cdp::browser_protocol::network::SetExtraHttpHeadersParams;
    use crate::emulation;

    let fp = super::fingerprint::generate_fingerprint_with_real_ua(real_ua);
    let script = get_stealth_init_script(&fp);

    // Disable the automation flag at the native level so navigator.webdriver returns false
    // without any JS own-property override (invisible to creepjs lie detection).
    let _ = page
        .execute(SetAutomationOverrideParams::new(false))
        .await;

    page.execute(AddScriptToEvaluateOnNewDocumentParams::new(script))
        .await
        .map_err(|e| onecrawl_core::Error::Cdp(format!("addScriptToEvaluateOnNewDocument: {e}")))?;

    // Set UA + full Accept-Language list via CDP so navigator.language/languages are set
    // natively (no detectable JS own-property override).
    let accept_lang_cdp = fp.languages.join(","); // e.g. "it-IT,it,en-US,en"
    emulation::set_user_agent_with_lang(page, &fp.user_agent, Some(&accept_lang_cdp))
        .await?;

    // Force Accept-Language HTTP header to match navigator.languages (avoids inconsistency detection)
    let accept_lang_val = fp.languages.iter().enumerate().map(|(i, lang)| {
        if i == 0 { lang.clone() } else { format!("{};q={:.1}", lang, 1.0 - i as f64 * 0.1) }
    }).collect::<Vec<_>>().join(",");
    use chromiumoxide::cdp::browser_protocol::network::Headers;
    let _ = page
        .execute(
            SetExtraHttpHeadersParams::new(
                Headers::new(serde_json::json!({ "Accept-Language": accept_lang_val }))
            )
        )
        .await;

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

        // navigator.webdriver: JS wrapper overrides to return undefined
        assert!(script.contains("webdriver"));
        assert!(script.contains("window.chrome"));
        assert!(script.contains("'plugins'"));
        assert!(script.contains("languages"));
        assert!(script.contains("'platform'"));
        assert!(script.contains("hardwareConcurrency"));
        assert!(script.contains("deviceMemory"));
        assert!(script.contains("userAgentData"));
        assert!(script.contains("getComputedStyle"));
        assert!(script.contains("permissions.query"));
        assert!(script.contains("outerWidth"));
    }

    #[test]
    fn script_uses_fingerprint_values() {
        let fp = generate_fingerprint();
        let script = get_stealth_init_script(&fp);

        assert!(script.contains(&fp.platform));
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
