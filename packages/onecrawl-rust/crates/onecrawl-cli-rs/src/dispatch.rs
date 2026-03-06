use crate::cli::*;
use crate::commands;

pub(crate) async fn dispatch(command: Commands) {
    match command {

        // ── System ──────────────────────────────────────────────────
        Commands::Health => {
            println!("✅ OneCrawl Rust CLI v{}", env!("CARGO_PKG_VERSION"));
            println!("   Crates: core, crypto, parser, storage, cdp");
            println!("   Runtime: Tokio async");
        }
        Commands::Info => {
            println!("OneCrawl v{}", env!("CARGO_PKG_VERSION"));
            println!("Arch: {}", std::env::consts::ARCH);
            println!("OS: {}", std::env::consts::OS);
            println!("Rust: compiled native binary");
        }

        // ── Offline Commands (untouched) ────────────────────────────
        Commands::Crypto { action } => commands::crypto::handle(action),
        Commands::Parse { action } => commands::parse::handle(action),
        Commands::Storage { action } => commands::storage::handle(action).await,

        // ── Session ─────────────────────────────────────────────────
        Commands::Session { action } => commands::session::handle(action).await,

        // ── Navigation ──────────────────────────────────────────────
        Commands::Navigate { url, wait, wait_cf } => commands::browser::navigate(&url, wait, wait_cf).await,
        Commands::Back => commands::browser::back().await,
        Commands::Forward => commands::browser::forward().await,
        Commands::Reload => commands::browser::reload().await,

        // ── Content ─────────────────────────────────────────────────
        Commands::Get { what, selector, arg } => {
            let resolved_sel = selector.as_deref().map(onecrawl_cdp::accessibility::resolve_ref);
            commands::browser::get(&what, resolved_sel.as_deref(), arg.as_deref()).await
        }
        Commands::Eval { expression } => commands::browser::eval(&expression).await,
        Commands::SetContent { html } => commands::browser::set_content(&html).await,

        // ── Element Interaction ─────────────────────────────────────
        Commands::Click { selector } => commands::browser::click(&onecrawl_cdp::accessibility::resolve_ref(&selector)).await,
        Commands::Dblclick { selector } => commands::browser::dblclick(&onecrawl_cdp::accessibility::resolve_ref(&selector)).await,
        Commands::Type { selector, text } => commands::browser::type_text(&onecrawl_cdp::accessibility::resolve_ref(&selector), &text).await,
        Commands::Fill { selector, text } => commands::browser::fill(&onecrawl_cdp::accessibility::resolve_ref(&selector), &text).await,
        Commands::Focus { selector } => commands::browser::focus(&onecrawl_cdp::accessibility::resolve_ref(&selector)).await,
        Commands::Hover { selector } => commands::browser::hover(&onecrawl_cdp::accessibility::resolve_ref(&selector)).await,
        Commands::ScrollIntoView { selector } => {
            commands::browser::scroll_into_view(&onecrawl_cdp::accessibility::resolve_ref(&selector)).await
        }
        Commands::Check { selector } => commands::browser::check(&onecrawl_cdp::accessibility::resolve_ref(&selector)).await,
        Commands::Uncheck { selector } => commands::browser::uncheck(&onecrawl_cdp::accessibility::resolve_ref(&selector)).await,
        Commands::SelectOption { selector, value } => {
            commands::browser::select_option(&onecrawl_cdp::accessibility::resolve_ref(&selector), &value).await
        }
        Commands::Tap { selector } => commands::browser::tap(&onecrawl_cdp::accessibility::resolve_ref(&selector)).await,
        Commands::Drag { from, to } => commands::browser::drag(&onecrawl_cdp::accessibility::resolve_ref(&from), &onecrawl_cdp::accessibility::resolve_ref(&to)).await,
        Commands::Upload {
            selector,
            file_path,
        } => commands::browser::upload(&onecrawl_cdp::accessibility::resolve_ref(&selector), &file_path).await,
        Commands::BoundingBox { selector } => commands::browser::bounding_box(&onecrawl_cdp::accessibility::resolve_ref(&selector)).await,

        // ── Keyboard ────────────────────────────────────────────────
        Commands::PressKey { key } => commands::browser::press_key(&key).await,
        Commands::KeyDown { key } => commands::browser::key_down(&key).await,
        Commands::KeyUp { key } => commands::browser::key_up(&key).await,
        Commands::KeyboardShortcut { keys } => commands::browser::keyboard_shortcut(&keys).await,

        // ── Screenshot / PDF ────────────────────────────────────────
        Commands::Screenshot {
            output,
            full,
            element,
            format,
            quality,
            annotate,
        } => {
            commands::browser::screenshot(&output, full, element.as_deref(), &format, quality, annotate).await
        }
        Commands::Pdf {
            output,
            landscape,
            scale,
        } => commands::browser::pdf(&output, landscape, scale).await,

        // ── Cookies ─────────────────────────────────────────────────
        Commands::Cookie { action } => match action {
            CookieAction::Get { name, json } => {
                commands::browser::cookie_get(name.as_deref(), json).await
            }
            CookieAction::Set {
                name,
                value,
                domain,
                path,
            } => {
                commands::browser::cookie_set(&name, &value, domain.as_deref(), path.as_deref())
                    .await
            }
            CookieAction::Delete { name, domain } => {
                commands::browser::cookie_delete(&name, &domain).await
            }
            CookieAction::Clear => commands::browser::cookie_clear().await,
            CookieAction::Export { output } => {
                commands::browser::cookie_export(output.as_deref()).await
            }
            CookieAction::Import { path } => {
                commands::browser::cookie_import(&path).await
            }
        },

        // ── Emulation ───────────────────────────────────────────────
        Commands::Emulate { action } => match action {
            EmulateAction::Viewport {
                width,
                height,
                scale,
            } => commands::browser::emulate_viewport(width, height, scale).await,
            EmulateAction::Device { name } => commands::browser::emulate_device(&name).await,
            EmulateAction::UserAgent { ua } => commands::browser::emulate_user_agent(&ua).await,
            EmulateAction::Geolocation { lat, lon, accuracy } => {
                commands::browser::emulate_geolocation(lat, lon, accuracy).await
            }
            EmulateAction::ColorScheme { scheme } => {
                commands::browser::emulate_color_scheme(&scheme).await
            }
            EmulateAction::Clear => commands::browser::emulate_clear().await,
        },

        // ── Network ─────────────────────────────────────────────────
        Commands::Network { action } => match action {
            NetworkAction::Block { types } => commands::browser::network_block(&types).await,
        },

        // ── HAR ─────────────────────────────────────────────────────
        Commands::Har { action } => match action {
            HarAction::Start => commands::browser::har_start().await,
            HarAction::Drain => commands::browser::har_drain().await,
            HarAction::Export { output } => commands::browser::har_export(&output).await,
        },

        // ── WebSocket ───────────────────────────────────────────────
        Commands::Ws { action } => match action {
            WsAction::Start => commands::browser::ws_start().await,
            WsAction::Drain => commands::browser::ws_drain().await,
            WsAction::Export { output } => commands::browser::ws_export(&output).await,
            WsAction::Connections => commands::browser::ws_connections().await,
        },

        // ── Coverage ────────────────────────────────────────────────
        Commands::Coverage { action } => match action {
            CoverageAction::JsStart => commands::browser::coverage_js_start().await,
            CoverageAction::JsStop => commands::browser::coverage_js_stop().await,
            CoverageAction::CssStart => commands::browser::coverage_css_start().await,
            CoverageAction::CssReport => commands::browser::coverage_css_report().await,
        },

        // ── Accessibility ───────────────────────────────────────────
        Commands::Accessibility { action } => match action {
            AccessibilityAction::Tree => commands::browser::a11y_tree().await,
            AccessibilityAction::Element { selector } => {
                commands::browser::a11y_element(&selector).await
            }
            AccessibilityAction::Audit => commands::browser::a11y_audit().await,
        },

        // ── Throttle ────────────────────────────────────────────────
        Commands::Throttle { action } => match action {
            ThrottleAction::Set { profile } => commands::browser::throttle_set(&profile).await,
            ThrottleAction::Custom {
                download_kbps,
                upload_kbps,
                latency_ms,
            } => commands::browser::throttle_custom(download_kbps, upload_kbps, latency_ms).await,
            ThrottleAction::Clear => commands::browser::throttle_clear().await,
        },

        // ── Performance ─────────────────────────────────────────────
        Commands::Perf { action } => match action {
            PerfAction::TraceStart => commands::browser::perf_trace_start().await,
            PerfAction::TraceStop => commands::browser::perf_trace_stop().await,
            PerfAction::Metrics => commands::browser::perf_metrics().await,
            PerfAction::Timing => commands::browser::perf_timing().await,
            PerfAction::Resources => commands::browser::perf_resources().await,
        },

        // ── Console ─────────────────────────────────────────────────
        Commands::Console { action } => match action {
            ConsoleAction::Start => commands::browser::console_start().await,
            ConsoleAction::Drain => commands::browser::console_drain().await,
            ConsoleAction::Clear => commands::browser::console_clear().await,
        },

        // ── Dialog ──────────────────────────────────────────────────
        Commands::Dialog { action } => match action {
            DialogAction::SetHandler {
                accept,
                prompt_text,
            } => commands::browser::dialog_set_handler(accept, prompt_text.as_deref()).await,
            DialogAction::History => commands::browser::dialog_history().await,
            DialogAction::Clear => commands::browser::dialog_clear().await,
        },

        // ── Worker ──────────────────────────────────────────────────
        Commands::Worker { action } => match action {
            WorkerAction::List => commands::browser::worker_list().await,
            WorkerAction::Unregister => commands::browser::worker_unregister().await,
            WorkerAction::Info => commands::browser::worker_info().await,
        },

        // ── DOM Observer ────────────────────────────────────────────
        Commands::Dom { action } => match action {
            DomAction::Observe { selector } => {
                commands::browser::dom_observe(selector.as_deref()).await
            }
            DomAction::Mutations => commands::browser::dom_mutations().await,
            DomAction::Stop => commands::browser::dom_stop().await,
            DomAction::Snapshot { selector } => {
                commands::browser::dom_snapshot(selector.as_deref()).await
            }
        },

        // ── Iframe ──────────────────────────────────────────────────
        Commands::Iframe { action } => match action {
            IframeAction::List => commands::browser::iframe_list().await,
            IframeAction::Eval { index, expression } => {
                commands::browser::iframe_eval(index, &expression).await
            }
            IframeAction::Content { index } => commands::browser::iframe_content(index).await,
        },

        // ── Network Log ─────────────────────────────────────────────
        Commands::NetworkLog { action } => match action {
            NetworkLogAction::Start => commands::browser::network_log_start().await,
            NetworkLogAction::Drain => commands::browser::network_log_drain().await,
            NetworkLogAction::Summary => commands::browser::network_log_summary().await,
            NetworkLogAction::Stop => commands::browser::network_log_stop().await,
            NetworkLogAction::Export { path } => commands::browser::network_log_export(&path).await,
        },

        // ── Page Watcher ────────────────────────────────────────────
        Commands::PageWatcher { action } => match action {
            PageWatcherAction::Start => commands::browser::page_watcher_start().await,
            PageWatcherAction::Drain => commands::browser::page_watcher_drain().await,
            PageWatcherAction::Stop => commands::browser::page_watcher_stop().await,
            PageWatcherAction::State => commands::browser::page_watcher_state().await,
        },

        // ── Print (Enhanced) ────────────────────────────────────────
        Commands::Print { action } => match action {
            PrintAction::Pdf {
                output,
                landscape,
                background,
                scale,
                paper_width,
                paper_height,
                margins,
                page_ranges,
                header,
                footer,
            } => {
                commands::browser::print_pdf(
                    &output,
                    landscape,
                    background,
                    scale,
                    paper_width,
                    paper_height,
                    margins.as_deref(),
                    page_ranges,
                    header,
                    footer,
                )
                .await
            }
            PrintAction::Metrics => commands::browser::print_metrics().await,
        },

        // ── Web Storage ─────────────────────────────────────────────
        Commands::WebStorage { action } => match action {
            WebStorageAction::LocalGet => commands::browser::web_storage_local_get().await,
            WebStorageAction::LocalSet { key, value } => {
                commands::browser::web_storage_local_set(&key, &value).await
            }
            WebStorageAction::LocalClear => commands::browser::web_storage_local_clear().await,
            WebStorageAction::SessionGet => commands::browser::web_storage_session_get().await,
            WebStorageAction::SessionSet { key, value } => {
                commands::browser::web_storage_session_set(&key, &value).await
            }
            WebStorageAction::SessionClear => commands::browser::web_storage_session_clear().await,
            WebStorageAction::IndexeddbList => {
                commands::browser::web_storage_indexeddb_list().await
            }
            WebStorageAction::ClearAll => commands::browser::web_storage_clear_all().await,
        },

        // ── Auth / Passkey ────────────────────────────────────────────
        Commands::Auth { action } => match action {
            AuthAction::PasskeyEnable {
                protocol,
                transport,
            } => commands::browser::passkey_enable(&protocol, &transport).await,
            AuthAction::PasskeyAdd {
                credential_id,
                rp_id,
                user_handle,
            } => {
                commands::browser::passkey_add(&credential_id, &rp_id, user_handle.as_deref())
                    .await
            }
            AuthAction::PasskeyList => commands::browser::passkey_list().await,
            AuthAction::PasskeyLog => commands::browser::passkey_log().await,
            AuthAction::PasskeyDisable => commands::browser::passkey_disable().await,
            AuthAction::PasskeyRemove { credential_id } => {
                commands::browser::passkey_remove(&credential_id).await
            }
            AuthAction::PasskeyRegister { output, timeout_secs } => {
                commands::browser::passkey_register(&output, timeout_secs).await
            }
            AuthAction::PasskeySetFile { file } => {
                commands::browser::passkey_set_file(&file).await
            }
            AuthAction::VaultList => {
                commands::browser::passkey_vault_list();
            }
            AuthAction::VaultSave { input } => {
                commands::browser::passkey_vault_save(&input);
            }
            AuthAction::VaultRemove { credential_id } => {
                commands::browser::passkey_vault_remove(&credential_id);
            }
            AuthAction::VaultClearSite { rp_id } => {
                commands::browser::passkey_vault_clear_site(&rp_id);
            }
            AuthAction::VaultExport { rp_id, output } => {
                commands::browser::passkey_vault_export(&rp_id, &output);
            }
            AuthAction::ImportBitwarden { input, vault } => {
                commands::browser::passkey_import_bitwarden(&input, vault);
            }
            AuthAction::ImportOnePassword { input, vault } => {
                commands::browser::passkey_import_1password(&input, vault);
            }
            AuthAction::ImportCxf { input, vault } => {
                commands::browser::passkey_import_cxf(&input, vault);
            }
        },

        // ── Stealth ─────────────────────────────────────────────────
        Commands::Stealth { action } => match action {
            StealthAction::Inject => commands::browser::stealth_inject().await,
        },

        // ── Anti-Bot ────────────────────────────────────────────────
        Commands::Antibot { action } => match action {
            AntibotAction::Inject { level } => commands::browser::antibot_inject(&level).await,
            AntibotAction::Test => commands::browser::antibot_test().await,
            AntibotAction::Profiles => commands::browser::antibot_profiles().await,
        },

        // ── Adaptive Element Tracker ────────────────────────────────
        Commands::Adaptive { action } => match action {
            AdaptiveAction::Fingerprint { selector } => {
                commands::browser::adaptive_fingerprint(&selector).await
            }
            AdaptiveAction::Relocate { fingerprint_json } => {
                commands::browser::adaptive_relocate(&fingerprint_json).await
            }
            AdaptiveAction::Track { selectors, save } => {
                commands::browser::adaptive_track(&selectors, save.as_deref()).await
            }
            AdaptiveAction::RelocateAll { fingerprints_json } => {
                commands::browser::adaptive_relocate_all(&fingerprints_json).await
            }
            AdaptiveAction::Save { fingerprints, path } => {
                commands::browser::adaptive_save(&fingerprints, &path).await
            }
            AdaptiveAction::Load { path } => commands::browser::adaptive_load(&path).await,
        },

        // ── Wait ────────────────────────────────────────────────────
        Commands::Wait { ms } => commands::browser::wait_ms(ms).await,
        Commands::WaitForSelector { selector, timeout } => {
            commands::browser::wait_for_selector(&selector, timeout).await
        }
        Commands::WaitForUrl { url, timeout } => {
            commands::browser::wait_for_url(&url, timeout).await
        }
        Commands::WaitForText { text, timeout } => {
            commands::browser::wait_for_text(&text, timeout).await
        }
        Commands::WaitForLoad { state, timeout } => {
            commands::browser::wait_for_load(&state, timeout).await
        }
        Commands::WaitForFunction { expression, timeout } => {
            commands::browser::wait_for_function(&expression, timeout).await
        }

        // ── State Checks ───────────────────────────────────────────
        Commands::Is { check, selector } => {
            commands::browser::is_check(&check, &onecrawl_cdp::accessibility::resolve_ref(&selector)).await
        }

        // ── Scroll ─────────────────────────────────────────────────
        Commands::Scroll { direction, pixels, selector } => {
            let resolved_sel = selector.as_deref().map(onecrawl_cdp::accessibility::resolve_ref);
            commands::browser::scroll(&direction, pixels, resolved_sel.as_deref()).await
        }

        // ── Keyboard ───────────────────────────────────────────────
        Commands::Keyboard { action } => match action {
            KeyboardAction::Type { text } => commands::browser::keyboard_type(&text).await,
            KeyboardAction::InsertText { text } => commands::browser::keyboard_insert_text(&text).await,
        },

        // ── Mouse ──────────────────────────────────────────────────
        Commands::Mouse { action } => match action {
            MouseAction::Move { x, y } => commands::browser::mouse_move(x, y).await,
            MouseAction::Down { button } => commands::browser::mouse_down(&button).await,
            MouseAction::Up { button } => commands::browser::mouse_up(&button).await,
            MouseAction::Wheel { dy, dx } => commands::browser::mouse_wheel(dy, dx).await,
        },

        // ── Find ───────────────────────────────────────────────────
        Commands::Find { action } => commands::browser::find_action(action).await,

        // ── Diff ───────────────────────────────────────────────────
        Commands::Diff { action } => match action {
            DiffAction::Snapshot { baseline, .. } => commands::browser::diff_snapshot(baseline.as_deref()).await,
            DiffAction::Screenshot { baseline, .. } => commands::browser::diff_screenshot(Some(&baseline)).await,
            DiffAction::Url { url1, url2, .. } => commands::browser::diff_url(&url1, &url2).await,
        },

        // ── Errors ─────────────────────────────────────────────────
        Commands::Errors { clear } => commands::browser::page_errors(clear).await,

        // ── Highlight ──────────────────────────────────────────────
        Commands::Highlight { selector } => {
            commands::browser::highlight(&onecrawl_cdp::accessibility::resolve_ref(&selector)).await
        }

        // ── Auth State ─────────────────────────────────────────────
        Commands::AuthState { action } => match action {
            AuthStateAction::Save { path } => commands::browser::auth_state_save(&path).await,
            AuthStateAction::Load { path } => commands::browser::auth_state_load(&path).await,
            AuthStateAction::List => commands::browser::auth_state_list().await,
            AuthStateAction::Show { path } => commands::browser::auth_state_show(&path).await,
            AuthStateAction::Rename { old, new } => commands::browser::auth_state_rename(&old, &new).await,
            AuthStateAction::Clear { all, name } => {
                if all {
                    commands::browser::auth_state_clean().await;
                } else if let Some(n) = name {
                    commands::browser::auth_state_clear(&n).await;
                } else {
                    eprintln!("Specify --all or a state name");
                }
            }
            AuthStateAction::Clean { .. } => commands::browser::auth_state_clean().await,
        },

        // ── Window ─────────────────────────────────────────────────
        Commands::Window { action } => match action {
            WindowAction::New => commands::browser::window_new().await,
        },

        // ── Set ────────────────────────────────────────────────────
        Commands::Set { action } => match action {
            SetAction::Viewport { width, height } => commands::browser::emulate_viewport(width, height, 1.0).await,
            SetAction::Device { name } => commands::browser::emulate_device(&name).await,
            SetAction::Geo { lat, lng } => commands::browser::emulate_geolocation(lat, lng, 1.0).await,
            SetAction::Offline { state } => commands::browser::set_offline(&state).await,
            SetAction::Headers { json } => commands::browser::set_extra_headers(&json).await,
            SetAction::Credentials { username, password } => commands::browser::set_credentials(&username, &password).await,
            SetAction::Media { scheme } => commands::browser::emulate_color_scheme(&scheme).await,
        },

        // ── Route / Unroute / Requests / Close ─────────────────────
        Commands::Route { pattern, status, body, content_type, block } => {
            commands::browser::route_add(&pattern, status, body.as_deref(), &content_type, block).await
        }
        Commands::Unroute { pattern } => commands::browser::route_remove(&pattern).await,
        Commands::Requests { filter, limit, failed } => {
            commands::browser::requests_list(filter.as_deref(), limit, failed).await
        }
        Commands::Close { all } => commands::browser::close_page(all).await,

        // ── Pages ───────────────────────────────────────────────────
        Commands::NewPage { url } => commands::browser::new_page(url.as_deref()).await,

        // ── Proxy ───────────────────────────────────────────────────
        Commands::Proxy { action } => match action {
            ProxyAction::CreatePool { json } => commands::browser::proxy_create_pool(&json).await,
            ProxyAction::ChromeArgs { json } => commands::browser::proxy_chrome_args(&json).await,
            ProxyAction::Next { json } => commands::browser::proxy_next(&json).await,
        },

        // ── Proxy Health ────────────────────────────────────────────
        Commands::ProxyHealth { action } => match action {
            ProxyHealthAction::Check {
                proxy,
                test_url,
                timeout,
            } => commands::browser::proxy_health_check(&proxy, test_url.as_deref(), timeout).await,
            ProxyHealthAction::CheckAll { proxies_json } => {
                commands::browser::proxy_health_check_all(&proxies_json).await
            }
            ProxyHealthAction::Rank { results_json } => {
                commands::browser::proxy_health_rank(&results_json);
            }
            ProxyHealthAction::Filter {
                results_json,
                min_score,
            } => {
                commands::browser::proxy_health_filter(&results_json, min_score);
            }
        },

        // ── Request Interception ────────────────────────────────────
        Commands::Intercept { action } => match action {
            InterceptCommandAction::Set { rules_json } => {
                commands::browser::intercept_set(&rules_json).await
            }
            InterceptCommandAction::Log => commands::browser::intercept_log().await,
            InterceptCommandAction::Clear => commands::browser::intercept_clear().await,
        },

        // ── Advanced Emulation ──────────────────────────────────────
        Commands::AdvancedEmulation { action } => match action {
            AdvancedEmulationAction::Orientation { alpha, beta, gamma } => {
                commands::browser::adv_emulation_orientation(alpha, beta, gamma).await
            }
            AdvancedEmulationAction::Permission { name, state } => {
                commands::browser::adv_emulation_permission(&name, &state).await
            }
            AdvancedEmulationAction::Battery { level, charging } => {
                commands::browser::adv_emulation_battery(level, charging).await
            }
            AdvancedEmulationAction::Connection {
                effective_type,
                downlink,
                rtt,
            } => commands::browser::adv_emulation_connection(&effective_type, downlink, rtt).await,
            AdvancedEmulationAction::CpuCores { n } => {
                commands::browser::adv_emulation_cpu_cores(n).await
            }
            AdvancedEmulationAction::Memory { gb } => {
                commands::browser::adv_emulation_memory(gb).await
            }
            AdvancedEmulationAction::NavigatorInfo => {
                commands::browser::adv_emulation_navigator_info().await
            }
        },

        // ── Tab Management ──────────────────────────────────────────
        Commands::Tab { action } => match action {
            TabAction::List => commands::browser::tab_list().await,
            TabAction::New { url } => commands::browser::tab_new(&url).await,
            TabAction::Close { index } => commands::browser::tab_close(index).await,
            TabAction::Switch { index } => commands::browser::tab_switch(index).await,
            TabAction::Count => commands::browser::tab_count_cmd().await,
        },

        // ── Download Management ─────────────────────────────────────
        Commands::Download { action } => match action {
            DownloadAction::SetPath { path } => commands::browser::download_set_path(&path).await,
            DownloadAction::List => commands::browser::download_list().await,
            DownloadAction::Fetch { url } => commands::browser::download_fetch(&url).await,
            DownloadAction::Wait { timeout } => commands::browser::download_wait(timeout).await,
            DownloadAction::Clear => commands::browser::download_clear().await,
        },

        // ── Screenshot Diff ─────────────────────────────────────────
        Commands::ScreenshotDiff { action } => match action {
            ScreenshotDiffAction::Compare { baseline, current } => {
                commands::browser::screenshot_diff_compare(&baseline, &current).await
            }
            ScreenshotDiffAction::Regression { baseline_path } => {
                commands::browser::screenshot_diff_regression(&baseline_path).await
            }
        },

        // ── Geofencing ─────────────────────────────────────────────
        Commands::Geo { action } => match action {
            GeoAction::Apply { profile } => commands::browser::geo_apply(&profile).await,
            GeoAction::Presets => commands::browser::geo_presets().await,
            GeoAction::Current => commands::browser::geo_current().await,
        },

        // ── Cookie Jar ─────────────────────────────────────────────
        Commands::CookieJar { action } => match action {
            CookieJarAction::Export { output } => {
                commands::browser::cookie_jar_export(output.as_deref()).await
            }
            CookieJarAction::Import { path } => commands::browser::cookie_jar_import(&path).await,
            CookieJarAction::Clear => commands::browser::cookie_jar_clear().await,
        },

        // ── Request Queue ──────────────────────────────────────────
        Commands::Request { action } => match action {
            RequestAction::Execute { json } => commands::browser::request_execute(&json).await,
            RequestAction::Batch {
                json,
                concurrency,
                delay,
            } => commands::browser::request_batch(&json, concurrency, delay).await,
        },

        // ── Benchmark ───────────────────────────────────────────────
        Commands::Bench { action } => match action {
            BenchAction::Run { iterations, module } => {
                commands::browser::bench_run(iterations, module.as_deref()).await
            }
            BenchAction::Report { format } => commands::browser::bench_report(&format).await,
        },

        // ── Smart Selectors ─────────────────────────────────────────
        Commands::Select { action } => match action {
            SelectAction::Css { selector } => commands::browser::select_css(&selector).await,
            SelectAction::Xpath { expression } => {
                commands::browser::select_xpath(&expression).await
            }
            SelectAction::Text { text, tag } => {
                commands::browser::select_text(&text, tag.as_deref()).await
            }
            SelectAction::Regex { pattern, tag } => {
                commands::browser::select_regex(&pattern, tag.as_deref()).await
            }
            SelectAction::AutoSelector { selector } => {
                commands::browser::select_auto(&selector).await
            }
        },

        // ── DOM Navigation ──────────────────────────────────────────
        Commands::Nav { action } => match action {
            NavAction::Parent { selector } => commands::browser::nav_parent(&selector).await,
            NavAction::Children { selector } => commands::browser::nav_children(&selector).await,
            NavAction::NextSibling { selector } => {
                commands::browser::nav_next_sibling(&selector).await
            }
            NavAction::PrevSibling { selector } => {
                commands::browser::nav_prev_sibling(&selector).await
            }
            NavAction::Siblings { selector } => commands::browser::nav_siblings(&selector).await,
            NavAction::Similar { selector } => commands::browser::nav_similar(&selector).await,
            NavAction::Above { selector, limit } => {
                commands::browser::nav_above(&selector, limit).await
            }
            NavAction::Below { selector, limit } => {
                commands::browser::nav_below(&selector, limit).await
            }
        },

        // ── Content Extraction ──────────────────────────────────────
        Commands::Extract { action } => match action {
            ExtractAction::Content {
                format,
                selector,
                output,
            } => {
                commands::browser::extract_content(&format, selector.as_deref(), output.as_deref())
                    .await
            }
            ExtractAction::Metadata => commands::browser::extract_metadata().await,
        },

        // ── Spider / Crawl ──────────────────────────────────────────
        Commands::Spider { action } => match action {
            SpiderAction::Crawl {
                start_url,
                max_depth,
                max_pages,
                concurrency,
                delay,
                same_domain,
                selector,
                format,
                output,
                output_format,
            } => {
                commands::browser::spider_crawl(
                    &start_url,
                    max_depth,
                    max_pages,
                    concurrency,
                    delay,
                    same_domain,
                    selector.as_deref(),
                    &format,
                    output.as_deref(),
                    &output_format,
                )
                .await
            }
            SpiderAction::Resume { state_file } => {
                commands::browser::spider_resume(&state_file).await
            }
            SpiderAction::Summary { results_file } => {
                commands::browser::spider_summary(&results_file)
            }
        },

        // ── Robots.txt ─────────────────────────────────────────────
        Commands::Robots { action } => match action {
            RobotsAction::Parse { source } => commands::browser::robots_parse(&source).await,
            RobotsAction::Check {
                url,
                path,
                user_agent,
            } => commands::browser::robots_check(&url, &path, &user_agent).await,
            RobotsAction::Sitemaps { url } => commands::browser::robots_sitemaps(&url).await,
        },

        // ── Link Graph ─────────────────────────────────────────────
        Commands::Graph { action } => match action {
            GraphAction::Extract { base_url } => {
                commands::browser::graph_extract(base_url.as_deref()).await
            }
            GraphAction::Build { edges_json } => commands::browser::graph_build(&edges_json),
            GraphAction::Analyze { graph_json } => commands::browser::graph_analyze(&graph_json),
            GraphAction::Export {
                graph_json,
                output_path,
            } => commands::browser::graph_export(&graph_json, &output_path),
        },

        // ── Interactive Shell ──────────────────────────────────────
        Commands::Shell => commands::browser::shell_repl().await,

        // ── Domain Blocker ─────────────────────────────────────────
        Commands::Domain { action } => match action {
            DomainAction::Block { domains } => commands::browser::domain_block(&domains).await,
            DomainAction::BlockCategory { category } => {
                commands::browser::domain_block_category(&category).await
            }
            DomainAction::Unblock => commands::browser::domain_unblock().await,
            DomainAction::Stats => commands::browser::domain_stats().await,
            DomainAction::List => commands::browser::domain_list().await,
            DomainAction::Categories => commands::browser::domain_categories(),
        },

        // ── Streaming Extractor ────────────────────────────────────
        Commands::StreamExtract {
            item_selector,
            field,
            paginate,
            max_pages,
            output,
            format,
        } => {
            commands::browser::stream_extract(
                &item_selector,
                &field,
                paginate.as_deref(),
                max_pages,
                output.as_deref(),
                &format,
            )
            .await
        }

        // ── HTTP Client ────────────────────────────────────────────
        Commands::Http { action } => match action {
            HttpAction::Get { url } => commands::browser::http_get(&url).await,
            HttpAction::Post {
                url,
                body,
                content_type,
            } => commands::browser::http_post(&url, &body, &content_type).await,
            HttpAction::Head { url } => commands::browser::http_head(&url).await,
            HttpAction::Fetch { json } => commands::browser::http_fetch(&json).await,
            HttpAction::Adaptive {
                url,
                retries,
                no_escalate,
                user_agent,
            } => {
                commands::browser::http_adaptive(&url, retries, no_escalate, user_agent.as_deref())
                    .await
            }
        },

        // ── TLS Fingerprint ──────────────────────────────────────────
        Commands::Fingerprint { action } => match action {
            FingerprintAction::Apply { name } => {
                commands::browser::fingerprint_apply(&name).await;
            }
            FingerprintAction::Detect => commands::browser::fingerprint_detect().await,
            FingerprintAction::List => commands::browser::fingerprint_list(),
        },

        // ── Page Snapshot ────────────────────────────────────────────
        Commands::Snapshot { action } => match action {
            SnapshotAction::Take { output } => {
                commands::browser::snapshot_take(output.as_deref()).await;
            }
            SnapshotAction::Compare { path1, path2 } => {
                commands::browser::snapshot_compare(&path1, &path2);
            }
            SnapshotAction::Watch {
                interval,
                selector,
                count,
            } => {
                commands::browser::snapshot_watch(interval, selector.as_deref(), count).await;
            }
            SnapshotAction::Agent {
                json,
                interactive_only,
                cursor,
                compact,
                depth,
                selector,
            } => {
                commands::browser::snapshot_agent(json, interactive_only, cursor, compact, depth, selector.as_deref()).await;
            }
        },

        // ── Rate Limiter ──────────────────────────────────────────────
        Commands::Ratelimit { action } => match action {
            RateLimitAction::Set { preset, config } => {
                commands::browser::ratelimit_set(preset.as_deref(), config.as_deref());
            }
            RateLimitAction::Stats => {
                commands::browser::ratelimit_stats();
            }
            RateLimitAction::Reset => {
                commands::browser::ratelimit_reset();
            }
        },

        // ── Retry Queue ───────────────────────────────────────────────
        Commands::Retry { action } => match action {
            RetryAction::Enqueue {
                url,
                operation,
                payload,
            } => {
                commands::browser::retry_enqueue(&url, &operation, payload.as_deref());
            }
            RetryAction::Next => {
                commands::browser::retry_next();
            }
            RetryAction::Success { id } => {
                commands::browser::retry_success(&id);
            }
            RetryAction::Fail { id, error } => {
                commands::browser::retry_fail(&id, &error);
            }
            RetryAction::Stats => {
                commands::browser::retry_stats();
            }
            RetryAction::Clear => {
                commands::browser::retry_clear();
            }
            RetryAction::Save { path } => {
                commands::browser::retry_save(&path);
            }
            RetryAction::Load { path } => {
                commands::browser::retry_load(&path);
            }
        },

        // ── Data Pipeline ────────────────────────────────────────────
        Commands::Pipeline { action } => match action {
            PipelineAction::Run {
                pipeline_json,
                data_json,
                output,
                format,
            } => {
                commands::browser::pipeline_run(
                    &pipeline_json,
                    &data_json,
                    output.as_deref(),
                    &format,
                );
            }
            PipelineAction::Validate { pipeline_json } => {
                commands::browser::pipeline_validate(&pipeline_json);
            }
            PipelineAction::Save {
                pipeline_json,
                path,
            } => {
                commands::browser::pipeline_save_file(&pipeline_json, &path);
            }
            PipelineAction::Load { path } => {
                commands::browser::pipeline_load_file(&path);
            }
        },

        // ── Structured Data ──────────────────────────────────────────
        Commands::Structured { action } => match action {
            StructuredAction::ExtractAll => {
                commands::browser::structured_extract_all().await;
            }
            StructuredAction::JsonLd => {
                commands::browser::structured_json_ld().await;
            }
            StructuredAction::OpenGraph => {
                commands::browser::structured_open_graph().await;
            }
            StructuredAction::TwitterCard => {
                commands::browser::structured_twitter_card().await;
            }
            StructuredAction::Metadata => {
                commands::browser::structured_metadata().await;
            }
            StructuredAction::Validate { data_json } => {
                commands::browser::structured_validate(&data_json);
            }
        },

        // ── Captcha ─────────────────────────────────────────────────
        Commands::Captcha { action } => match action {
            CaptchaAction::Detect => {
                commands::browser::captcha_detect().await;
            }
            CaptchaAction::Wait { timeout } => {
                commands::browser::captcha_wait(timeout).await;
            }
            CaptchaAction::Screenshot => {
                commands::browser::captcha_screenshot().await;
            }
            CaptchaAction::Inject { solution } => {
                commands::browser::captcha_inject(&solution).await;
            }
            CaptchaAction::Solve { timeout, api } => {
                commands::browser::captcha_solve(timeout, api).await;
            }
            CaptchaAction::Check => {
                commands::browser::stealth_check().await;
            }
            CaptchaAction::Types => {
                commands::browser::captcha_types();
            }
        },

        Commands::Schedule { action } => match action {
            ScheduleAction::Add {
                name,
                task_type,
                config,
                interval,
                delay,
                max_runs,
            } => {
                commands::browser::schedule_add(
                    &name, &task_type, &config, interval, delay, max_runs,
                );
            }
            ScheduleAction::Remove { id } => {
                commands::browser::schedule_remove(&id);
            }
            ScheduleAction::Pause { id } => {
                commands::browser::schedule_pause(&id);
            }
            ScheduleAction::Resume { id } => {
                commands::browser::schedule_resume(&id);
            }
            ScheduleAction::List => {
                commands::browser::schedule_list();
            }
            ScheduleAction::Stats => {
                commands::browser::schedule_stats();
            }
            ScheduleAction::Save { path } => {
                commands::browser::schedule_save(&path);
            }
            ScheduleAction::Load { path } => {
                commands::browser::schedule_load(&path);
            }
        },

        Commands::Pool { action } => match action {
            PoolAction::Add { name, tags } => {
                let tags = if tags.is_empty() { None } else { Some(tags) };
                commands::browser::pool_add(&name, tags);
            }
            PoolAction::Next => {
                commands::browser::pool_next();
            }
            PoolAction::Stats => {
                commands::browser::pool_stats();
            }
            PoolAction::Cleanup => {
                commands::browser::pool_cleanup();
            }
            PoolAction::Save { path } => {
                commands::browser::pool_save(&path);
            }
            PoolAction::Load { path } => {
                commands::browser::pool_load(&path);
            }
        },

        // ── Server ──────────────────────────────────────────────────
        Commands::Serve { port, bind: _ } => {
            onecrawl_server::serve::start_server(port).await.unwrap();
        }

        // ── MCP ─────────────────────────────────────────────────────
        Commands::Mcp { transport } => {
            match transport.as_str() {
                "stdio" => {
                    onecrawl_mcp_rs::start_stdio(
                        "/tmp/onecrawl-mcp-store".into(),
                        "onecrawl-default-key".into(),
                    )
                    .await
                    .unwrap();
                }
                other => {
                    eprintln!("unsupported transport: {other}");
                    eprintln!("available: stdio");
                    std::process::exit(1);
                }
            }
        }

        // ── Daemon ─────────────────────────────────────────────────
        Commands::Daemon { action } => match action {
            DaemonAction::Start { headless } => commands::daemon::daemon_start(headless).await,
            DaemonAction::Stop => commands::daemon::daemon_stop().await,
            DaemonAction::Status => commands::daemon::daemon_status().await,
            DaemonAction::Exec {
                command,
                args,
                session,
            } => commands::daemon::daemon_exec(&command, args, session).await,
            DaemonAction::Run { headless } => {
                if let Err(e) = commands::daemon::server::start_daemon(headless).await {
                    eprintln!("daemon fatal: {e}");
                    std::process::exit(1);
                }
            }
        },

        // ── Skills ────────────────────────────────────────────────
        Commands::Skills { action } => match action {
            SkillsAction::List => commands::skills::skills_list(),
            SkillsAction::Info { name } => commands::skills::skills_info(&name),
            SkillsAction::Discover { path } => commands::skills::skills_discover(&path),
        },

        // ── Live Streaming ───────────────────────────────────────────
        Commands::Stream { action } => match action {
            StreamAction::Start { width, height, format, quality } => {
                commands::browser::stream_start(width, height, &format, quality).await
            }
            StreamAction::Stop => commands::browser::stream_stop().await,
            StreamAction::Frame { output } => commands::browser::stream_frame(&output).await,
        },

        // ── Video Recording ─────────────────────────────────────────
        Commands::Record { action } => match action {
            RecordAction::Start { output, fps } => {
                commands::browser::recording_start(&output, fps).await
            }
            RecordAction::Stop => commands::browser::recording_stop().await,
            RecordAction::Status => commands::browser::recording_status().await,
        },

        // ── Version ─────────────────────────────────────────────────
        Commands::Version => {
            println!("onecrawl {}", env!("CARGO_PKG_VERSION"));
            println!();
            println!("Components:");
            println!("  core      onecrawl-core");
            println!("  crypto    onecrawl-crypto (AES-256-GCM, PKCE, TOTP, PBKDF2)");
            println!("  parser    onecrawl-parser (lol_html, scraper)");
            println!("  storage   onecrawl-storage (sled, encrypted KV)");
            println!("  cdp       onecrawl-cdp (63 modules)");
            println!("  server    onecrawl-server (axum, 21 endpoints)");
            println!("  mcp       onecrawl-mcp (43 tools, 10 namespaces)");
            println!();
            println!("Profile: {}", if cfg!(debug_assertions) { "debug" } else { "release" });
        }

        // ── iOS / Mobile Safari ────────────────────────────────────
        Commands::Ios { action } => match action {
            IosAction::Devices => commands::ios::devices(),
            IosAction::Connect { wda_url, udid } => {
                commands::ios::connect(&wda_url, udid.as_deref()).await
            }
            IosAction::Navigate { url } => commands::ios::navigate(&url).await,
            IosAction::Tap { x, y } => commands::ios::tap(x, y).await,
            IosAction::Screenshot { output } => commands::ios::screenshot(&output).await,
            IosAction::Disconnect => commands::ios::disconnect().await,
        },
        }
}
