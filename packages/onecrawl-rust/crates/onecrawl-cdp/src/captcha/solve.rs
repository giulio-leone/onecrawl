use chromiumoxide::Page;
use onecrawl_core::{Error, Result};

// ---------------------------------------------------------------------------
// Browser-native Turnstile solver (free — no external API)
// ---------------------------------------------------------------------------

/// Solve a Cloudflare Turnstile challenge using browser-native interaction.
///
/// Strategy:
/// 1. Find the Turnstile iframe
/// 2. Click the checkbox inside it using human-like behavior
/// 3. Wait for the challenge to auto-clear (stealth Chrome passes verification)
///
/// Returns `true` if the challenge was solved within `timeout_ms`.
pub async fn solve_turnstile_native(page: &Page, timeout_ms: u64) -> Result<bool> {
    use crate::human;

    // Step 1: Detect Turnstile iframe
    let iframe_sel: String = page
        .evaluate(
            r#"(() => {
                const cf = document.querySelector('.cf-turnstile iframe, iframe[src*="challenges.cloudflare"]');
                if (!cf) return '';
                // Tag the iframe for reliable selector
                cf.setAttribute('data-onecrawl-turnstile', '1');
                return '[data-onecrawl-turnstile="1"]';
            })()"#,
        )
        .await
        .map_err(|e| Error::Cdp(format!("turnstile detect: {e}")))?
        .into_value()
        .map_err(|e| Error::Cdp(format!("turnstile parse: {e}")))?;

    if iframe_sel.is_empty() {
        return Err(Error::Cdp(
            "No Turnstile iframe found on page".into(),
        ));
    }

    // Step 2: Click the Turnstile checkbox with human-like behavior
    human::pre_action_delay().await;
    let _ = human::human_click(page, &iframe_sel).await;
    human::post_action_delay().await;

    // Step 3: Wait for CF clearance
    Ok(human::wait_for_cf_clearance(page, timeout_ms).await)
}

// ---------------------------------------------------------------------------
// reCAPTCHA audio solver (free — uses local Whisper for transcription)
// ---------------------------------------------------------------------------

/// Solve a reCAPTCHA v2 challenge using the audio fallback + local Whisper STT.
///
/// Strategy:
/// 1. Click "I'm not a robot" checkbox
/// 2. Switch to audio challenge
/// 3. Download the audio file URL
/// 4. Transcribe using local `whisper` CLI (must be installed: `pip install openai-whisper`)
/// 5. Submit the transcription
///
/// Returns the transcription text if successful.
pub async fn solve_recaptcha_audio(page: &Page) -> Result<String> {
    use crate::human;

    // Step 1: Click the reCAPTCHA checkbox
    let checkbox_sel = r#"iframe[src*="recaptcha/api2/anchor"], iframe[title*="reCAPTCHA"]"#;
    human::human_click(page, checkbox_sel).await.map_err(|e| {
        Error::Cdp(format!("recaptcha checkbox click: {e}"))
    })?;

    // Brief wait for challenge popup to appear
    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

    // Step 2: Switch to audio challenge
    // The challenge opens in a new iframe
    let audio_btn_js = r#"(() => {
        // Find the challenge iframe
        const frames = document.querySelectorAll('iframe[src*="recaptcha/api2/bframe"]');
        for (const f of frames) {
            try {
                const doc = f.contentDocument || f.contentWindow?.document;
                if (!doc) continue;
                const btn = doc.querySelector('#recaptcha-audio-button, .rc-button-audio');
                if (btn) { btn.click(); return 'clicked'; }
            } catch(_) {}
        }
        return 'not_found';
    })()"#;

    let result: String = page
        .evaluate(audio_btn_js)
        .await
        .map_err(|e| Error::Cdp(format!("audio button: {e}")))?
        .into_value()
        .unwrap_or_default();

    if result != "clicked" {
        // Try cross-origin approach: click via selector in main frame
        let _ = page
            .evaluate(
                r#"document.querySelector('#recaptcha-audio-button, .rc-button-audio')?.click()"#,
            )
            .await;
    }

    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

    // Step 3: Get the audio URL
    let audio_url: String = page
        .evaluate(
            r#"(() => {
                const links = document.querySelectorAll('a.rc-audiochallenge-tdownload-link, audio source, #audio-source');
                for (const el of links) {
                    const href = el.href || el.src || el.getAttribute('src');
                    if (href) return href;
                }
                // Try inside iframes
                for (const f of document.querySelectorAll('iframe[src*="recaptcha"]')) {
                    try {
                        const doc = f.contentDocument || f.contentWindow?.document;
                        if (!doc) continue;
                        const link = doc.querySelector('.rc-audiochallenge-tdownload-link, audio source');
                        if (link) return link.href || link.src || '';
                    } catch(_) {}
                }
                return '';
            })()"#,
        )
        .await
        .map_err(|e| Error::Cdp(format!("audio url: {e}")))?
        .into_value()
        .unwrap_or_default();

    if audio_url.is_empty() {
        return Err(Error::Cdp(
            "Could not find reCAPTCHA audio URL. Challenge may be in a cross-origin iframe.".into(),
        ));
    }

    // Step 4: Download audio via page fetch and transcribe with local Whisper
    let audio_b64: String = page
        .evaluate(format!(
            r#"(async () => {{
                const resp = await fetch({url});
                const blob = await resp.blob();
                return new Promise(resolve => {{
                    const reader = new FileReader();
                    reader.onload = () => resolve(reader.result.split(',')[1]);
                    reader.readAsDataURL(blob);
                }});
            }})()"#,
            url = serde_json::to_string(&audio_url).unwrap_or_default()
        ))
        .await
        .map_err(|e| Error::Cdp(format!("audio download: {e}")))?
        .into_value()
        .unwrap_or_default();

    if audio_b64.is_empty() {
        return Err(Error::Cdp("Failed to download audio file".into()));
    }

    // Save audio to temp file and run Whisper
    let tmp_dir = std::env::temp_dir();
    let audio_path = tmp_dir.join("onecrawl_recaptcha_audio.mp3");
    let text_path = tmp_dir.join("onecrawl_recaptcha_audio.txt");

    // Decode base64 and save
    use std::io::Write;
    let audio_bytes = base64_decode(&audio_b64)?;
    std::fs::File::create(&audio_path)
        .and_then(|mut f| f.write_all(&audio_bytes))
        .map_err(|e| Error::Cdp(format!("save audio: {e}")))?;

    // Run Whisper CLI (must be installed: pip install openai-whisper)
    let output = std::process::Command::new("whisper")
        .args([
            audio_path.to_str().unwrap_or(""),
            "--model",
            "base",
            "--language",
            "en",
            "--output_format",
            "txt",
            "--output_dir",
            tmp_dir.to_str().unwrap_or("/tmp"),
        ])
        .output()
        .map_err(|e| Error::Cdp(format!(
            "whisper command failed (is it installed? `pip install openai-whisper`): {e}"
        )))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Cdp(format!("whisper failed: {stderr}")));
    }

    let transcription = std::fs::read_to_string(&text_path)
        .map_err(|e| Error::Cdp(format!("read whisper output: {e}")))?
        .trim()
        .to_string();

    // Cleanup temp files
    let _ = std::fs::remove_file(&audio_path);
    let _ = std::fs::remove_file(&text_path);

    if transcription.is_empty() {
        return Err(Error::Cdp("Whisper produced empty transcription".into()));
    }

    // Step 5: Submit the transcription
    let submit_js = format!(
        r#"(() => {{
            const input = document.querySelector('#audio-response, input[id="audio-response"]');
            if (!input) {{
                // Try inside iframe
                for (const f of document.querySelectorAll('iframe[src*="recaptcha"]')) {{
                    try {{
                        const doc = f.contentDocument || f.contentWindow?.document;
                        if (!doc) continue;
                        const inp = doc.querySelector('#audio-response');
                        if (inp) {{ inp.value = {text}; return 'filled'; }}
                    }} catch(_) {{}}
                }}
                return 'not_found';
            }}
            input.value = {text};
            return 'filled';
        }})()"#,
        text = serde_json::to_string(&transcription).unwrap_or_default()
    );

    let fill_result: String = page
        .evaluate(submit_js)
        .await
        .map_err(|e| Error::Cdp(format!("fill audio response: {e}")))?
        .into_value()
        .unwrap_or_default();

    if fill_result == "filled" {
        // Click verify button
        let _ = page
            .evaluate(
                r#"(() => {
                    const btn = document.querySelector('#recaptcha-verify-button, .rc-button-default');
                    if (btn) { btn.click(); return 'clicked'; }
                    for (const f of document.querySelectorAll('iframe[src*="recaptcha"]')) {
                        try {
                            const doc = f.contentDocument || f.contentWindow?.document;
                            if (!doc) continue;
                            const b = doc.querySelector('#recaptcha-verify-button, .rc-button-default');
                            if (b) { b.click(); return 'clicked'; }
                        } catch(_) {}
                    }
                    return 'not_found';
                })()"#,
            )
            .await;
    }

    Ok(transcription)
}

/// Simple base64 decoder (standard alphabet, no padding required).
pub(super) fn base64_decode(input: &str) -> Result<Vec<u8>> {
    const TABLE: [u8; 128] = {
        let mut t = [255u8; 128];
        let mut i = 0u8;
        while i < 26 { t[(b'A' + i) as usize] = i; i += 1; }
        i = 0;
        while i < 26 { t[(b'a' + i) as usize] = 26 + i; i += 1; }
        i = 0;
        while i < 10 { t[(b'0' + i) as usize] = 52 + i; i += 1; }
        t[b'+' as usize] = 62;
        t[b'/' as usize] = 63;
        t
    };

    let bytes: Vec<u8> = input.bytes().filter(|&b| b != b'=' && b != b'\n' && b != b'\r').collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);

    for chunk in bytes.chunks(4) {
        let mut buf = 0u32;
        let len = chunk.len();
        for (i, &b) in chunk.iter().enumerate() {
            let val = if (b as usize) < 128 { TABLE[b as usize] } else { 255 };
            if val == 255 {
                return Err(Error::Cdp(format!("invalid base64 char: {b}")));
            }
            buf |= (val as u32) << (18 - 6 * i);
        }
        if len > 1 { out.push((buf >> 16) as u8); }
        if len > 2 { out.push((buf >> 8) as u8); }
        if len > 3 { out.push(buf as u8); }
    }

    Ok(out)
}

