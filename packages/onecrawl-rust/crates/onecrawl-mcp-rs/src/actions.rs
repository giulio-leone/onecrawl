//! Compile-time action enums for each super-tool.
//!
//! Each enum maps 1:1 to the string-based actions in the dispatcher.
//! `FromStr` handles parsing; `match` on the enum is exhaustive.

use crate::helpers::mcp_err;
use rmcp::ErrorData as McpError;

macro_rules! action_enum {
    (
        $name:ident, $tool:expr,
        [ $( $variant:ident => $str:expr ),+ $(,)? ]
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name {
            $( $variant, )+
        }

        impl $name {
            pub fn parse(s: &str) -> Result<Self, McpError> {
                match s {
                    $( $str => Ok(Self::$variant), )+
                    other => Err(mcp_err(format!(
                        "unknown {} action: '{}'. Available: {}",
                        $tool,
                        other,
                        [ $( $str ),+ ].join(", ")
                    )))
                }
            }
        }
    };
}

action_enum!(BrowserAction, "browser", [
    Goto => "goto",
    Click => "click",
    Type => "type",
    Screenshot => "screenshot",
    Pdf => "pdf",
    Back => "back",
    Forward => "forward",
    Reload => "reload",
    Wait => "wait",
    Evaluate => "evaluate",
    Snapshot => "snapshot",
    Css => "css",
    Xpath => "xpath",
    FindText => "find_text",
    Text => "text",
    Html => "html",
    Markdown => "markdown",
    Structured => "structured",
    Stream => "stream",
    DetectForms => "detect_forms",
    FillForm => "fill_form",
    SnapshotDiff => "snapshot_diff",
    ParseA11y => "parse_a11y",
    ParseSelector => "parse_selector",
    ParseText => "parse_text",
    ParseLinks => "parse_links",
    NewTab => "new_tab",
    ListTabs => "list_tabs",
    SwitchTab => "switch_tab",
    CloseTab => "close_tab",
    ObserveMutations => "observe_mutations",
    GetMutations => "get_mutations",
    StopMutations => "stop_mutations",
    WaitForEvent => "wait_for_event",
    CookiesGet => "cookies_get",
    CookiesSet => "cookies_set",
    CookiesClear => "cookies_clear",
    StorageGet => "storage_get",
    StorageSet => "storage_set",
    ExportSession => "export_session",
    ImportSession => "import_session",
    InterceptEnable => "intercept_enable",
    InterceptAddRule => "intercept_add_rule",
    InterceptRemoveRule => "intercept_remove_rule",
    InterceptList => "intercept_list",
    InterceptDisable => "intercept_disable",
    BlockRequests => "block_requests",
    ConsoleStart => "console_start",
    ConsoleGet => "console_get",
    ConsoleClear => "console_clear",
    DialogHandle => "dialog_handle",
    DialogGet => "dialog_get",
    ErrorsGet => "errors_get",
    EmulateDevice => "emulate_device",
    EmulateGeolocation => "emulate_geolocation",
    EmulateTimezone => "emulate_timezone",
    EmulateMedia => "emulate_media",
    EmulateNetwork => "emulate_network",
    Drag => "drag",
    Hover => "hover",
    Keyboard => "keyboard",
    Select => "select",
    Upload => "upload",
    DownloadWait => "download_wait",
    DownloadList => "download_list",
    DownloadSetDir => "download_set_dir",
    ShadowQuery => "shadow_query",
    ShadowText => "shadow_text",
    DeepQuery => "deep_query",
    // Page context
    ContextSet => "context_set",
    ContextGet => "context_get",
    ContextList => "context_list",
    ContextClear => "context_clear",
    ContextTransfer => "context_transfer",
    // Smart form mapping
    FormInfer => "form_infer",
    FormAutoFill => "form_auto_fill",
    FormValidate => "form_validate",
    // Self-healing selector recovery
    SelectorHeal => "selector_heal",
    SelectorAlternatives => "selector_alternatives",
    SelectorValidate => "selector_validate",
    // Event-driven reaction system
    EventSubscribe => "event_subscribe",
    EventUnsubscribe => "event_unsubscribe",
    EventPoll => "event_poll",
    EventClear => "event_clear",
    // Service Worker & PWA
    SwRegister => "sw_register",
    SwUnregister => "sw_unregister",
    SwList => "sw_list",
    SwUpdate => "sw_update",
    CacheList => "cache_list",
    CacheClear => "cache_clear",
    PushSimulate => "push_simulate",
    OfflineMode => "offline_mode",
    // Session configuration
    SetMode => "set_mode",
    SetStealth => "set_stealth",
    SessionInfo => "session_info",
    SpaNavWatch => "spa_nav_watch",
    FrameworkDetect => "framework_detect",
    VirtualScrollDetect => "virtual_scroll_detect",
    VirtualScrollExtract => "virtual_scroll_extract",
    WaitHydration => "wait_hydration",
    WaitAnimation => "wait_animation",
    WaitNetworkIdle => "wait_network_idle",
    TriggerLazyLoad => "trigger_lazy_load",
    HealthCheck => "health_check",
    CircuitBreaker => "circuit_breaker",
    StateInspect => "state_inspect",
    FormWizardTrack => "form_wizard_track",
    DynamicImportWait => "dynamic_import_wait",
    ParallelExec => "parallel_exec",
    // Enhanced agentic: token budget, compact state, page assertions
    TokenBudget => "token_budget",
    CompactState => "compact_state",
    PageAssertions => "page_assertions",
]);

action_enum!(CrawlAction, "crawl", [
    Spider => "spider",
    Robots => "robots",
    Sitemap => "sitemap",
    DomSnapshot => "dom_snapshot",
    DomCompare => "dom_compare",
]);

action_enum!(AgentAction, "agent", [
    ExecuteChain => "execute_chain",
    ElementScreenshot => "element_screenshot",
    ApiCaptureStart => "api_capture_start",
    ApiCaptureSummary => "api_capture_summary",
    IframeList => "iframe_list",
    IframeSnapshot => "iframe_snapshot",
    IframeEvalCdp => "iframe_eval_cdp",
    IframeClickCdp => "iframe_click_cdp",
    IframeFrames => "iframe_frames",
    ConnectRemote => "connect_remote",
    SafetySet => "safety_set",
    SafetyStatus => "safety_status",
    SkillsList => "skills_list",
    ScreencastStart => "screencast_start",
    ScreencastStop => "screencast_stop",
    ScreencastFrame => "screencast_frame",
    RecordingStart => "recording_start",
    RecordingStop => "recording_stop",
    RecordingStatus => "recording_status",
    StreamCapture => "stream_capture",
    StreamToDisk => "stream_to_disk",
    RecordingEncode => "recording_encode",
    RecordingCapture => "recording_capture",
    IosDevices => "ios_devices",
    IosConnect => "ios_connect",
    IosNavigate => "ios_navigate",
    IosTap => "ios_tap",
    IosScreenshot => "ios_screenshot",
    IosPinch => "ios_pinch",
    IosLongPress => "ios_long_press",
    IosDoubleTap => "ios_double_tap",
    IosOrientation => "ios_orientation",
    IosScroll => "ios_scroll",
    IosScript => "ios_script",
    IosCookies => "ios_cookies",
    IosAppLaunch => "ios_app_launch",
    IosAppKill => "ios_app_kill",
    IosAppState => "ios_app_state",
    IosLock => "ios_lock",
    IosUnlock => "ios_unlock",
    IosHome => "ios_home",
    IosButton => "ios_button",
    IosBattery => "ios_battery",
    IosInfo => "ios_info",
    IosSimulator => "ios_simulator",
    IosUrl => "ios_url",
    IosTitle => "ios_title",
    // Task decomposition engine
    TaskDecompose => "task_decompose",
    TaskPlan => "task_plan",
    TaskStatus => "task_status",
    // Vision/LLM observation layer
    VisionDescribe => "vision_describe",
    VisionLocate => "vision_locate",
    VisionCompare => "vision_compare",
    // Accessibility & WCAG
    WcagAudit => "wcag_audit",
    AriaTree => "aria_tree",
    ContrastCheck => "contrast_check",
    LandmarkNav => "landmark_nav",
    FocusOrder => "focus_order",
    AltTextAudit => "alt_text_audit",
    HeadingStructure => "heading_structure",
    RoleValidate => "role_validate",
    KeyboardTrapDetect => "keyboard_trap_detect",
    ScreenReaderSim => "screen_reader_sim",
    // Autonomous agent loop
    AgentLoop => "agent_loop",
    GoalAssert => "goal_assert",
    AnnotatedObserve => "annotated_observe",
    // Session context, auto-chain, structured reasoning
    SessionContext => "session_context",
    AutoChain => "auto_chain",
    Think => "think",
    // Enhanced agentic: plan execute, page summary, error context
    PlanExecute => "plan_execute",
    PageSummary => "page_summary",
    ErrorContext => "error_context",
]);

action_enum!(StealthAction, "stealth", [
    Inject => "inject",
    Test => "test",
    Fingerprint => "fingerprint",
    BlockDomains => "block_domains",
    DetectCaptcha => "detect_captcha",
    SolveCaptcha => "solve_captcha",
    // Human behavior simulation
    HumanDelay => "human_delay",
    HumanMouse => "human_mouse",
    HumanType => "human_type",
    HumanScroll => "human_scroll",
    HumanProfile => "human_profile",
    StealthMax => "stealth_max",
    StealthScore => "stealth_score",
    TlsApply => "tls_apply",
    WebrtcBlock => "webrtc_block",
    BatterySpoof => "battery_spoof",
    SensorBlock => "sensor_block",
    CanvasAdvanced => "canvas_advanced",
    TimezoneSync => "timezone_sync",
    FontProtect => "font_protect",
    BehaviorSim => "behavior_sim",
    BehaviorStop => "behavior_stop",
    StealthRotate => "stealth_rotate",
    DetectionAudit => "detection_audit",
    // Enhanced agentic: stealth status report
    StealthStatus => "stealth_status",
]);

action_enum!(DataAction, "data", [
    Pipeline => "pipeline",
    HttpGet => "http_get",
    HttpPost => "http_post",
    Links => "links",
    Graph => "graph",
    NetCapture => "net_capture",
    NetAnalyze => "net_analyze",
    NetSdk => "net_sdk",
    NetMock => "net_mock",
    NetReplay => "net_replay",
    // Structured data pipeline
    ExtractSchema => "extract_schema",
    ExtractTables => "extract_tables",
    ExtractEntities => "extract_entities",
    ClassifyContent => "classify_content",
    TransformJson => "transform_json",
    ExportCsv => "export_csv",
    ExtractMetadata => "extract_metadata",
    ExtractFeeds => "extract_feeds",
    // WebSocket & Real-Time Protocol
    WsConnect => "ws_connect",
    WsIntercept => "ws_intercept",
    WsSend => "ws_send",
    WsMessages => "ws_messages",
    WsClose => "ws_close",
    SseListen => "sse_listen",
    SseMessages => "sse_messages",
    GraphqlSubscribe => "graphql_subscribe",
    // Enhanced agentic: compact extraction
    ExtractCompact => "extract_compact",
]);

action_enum!(SecureAction, "secure", [
    Encrypt => "encrypt",
    Decrypt => "decrypt",
    Pkce => "pkce",
    Totp => "totp",
    KvSet => "kv_set",
    KvGet => "kv_get",
    KvList => "kv_list",
    PasskeyEnable => "passkey_enable",
    PasskeyAdd => "passkey_add",
    PasskeyList => "passkey_list",
    PasskeyLog => "passkey_log",
    PasskeyDisable => "passkey_disable",
    PasskeyRemove => "passkey_remove",
    // Authentication flows
    AuthOauth2 => "auth_oauth2",
    AuthSession => "auth_session",
    AuthFormLogin => "auth_form_login",
    AuthMfa => "auth_mfa",
    AuthStatus => "auth_status",
    AuthLogout => "auth_logout",
    CredentialStore => "credential_store",
    CredentialGet => "credential_get",
]);

action_enum!(ComputerAction, "computer", [
    Act => "act",
    Observe => "observe",
    Batch => "batch",
    SmartFind => "smart_find",
    SmartClick => "smart_click",
    SmartFill => "smart_fill",
    PoolList => "pool_list",
    PoolStatus => "pool_status",
    // Multi-browser fleet
    FleetSpawn => "fleet_spawn",
    FleetBroadcast => "fleet_broadcast",
    FleetCollect => "fleet_collect",
    FleetDestroy => "fleet_destroy",
    FleetStatus => "fleet_status",
    FleetBalance => "fleet_balance",
    // Enhanced computer use
    ComputerUse => "computer_use",
    GoalExecute => "goal_execute",
    StepVerify => "step_verify",
    AutoRecover => "auto_recover",
    // Annotated screenshot & adaptive retry
    AnnotatedScreenshot => "annotated_screenshot",
    AdaptiveRetry => "adaptive_retry",
    // Coordinate click, multi-page sync, input replay
    ClickAtCoords => "click_at_coords",
    MultiPageSync => "multi_page_sync",
    InputReplay => "input_replay",
    // Enhanced agentic: element info
    ElementInfo => "element_info",
]);

action_enum!(MemoryAction, "memory", [
    Store => "store",
    Recall => "recall",
    Search => "search",
    Forget => "forget",
    DomainStrategy => "domain_strategy",
    Stats => "stats",
]);

action_enum!(AutomateAction, "automate", [
    WorkflowValidate => "workflow_validate",
    WorkflowRun => "workflow_run",
    Plan => "plan",
    Execute => "execute",
    Patterns => "patterns",
    RateLimit => "rate_limit",
    Retry => "retry",
    // Error recovery
    RetryAdapt => "retry_adapt",
    ErrorClassify => "error_classify",
    RecoverySuggest => "recovery_suggest",
    ErrorHistory => "error_history",
    // Session checkpoints/resume
    CheckpointSave => "checkpoint_save",
    CheckpointRestore => "checkpoint_restore",
    CheckpointList => "checkpoint_list",
    CheckpointDelete => "checkpoint_delete",
    // Extended workflow DSL
    WorkflowWhile => "workflow_while",
    WorkflowForEach => "workflow_for_each",
    WorkflowIf => "workflow_if",
    WorkflowVariable => "workflow_variable",
    // Long-running harness
    ReconnectCdp => "reconnect_cdp",
    GcTabs => "gc_tabs",
    Watchdog => "watchdog",
    // Enhanced agentic: batch execute
    BatchExecute => "batch_execute",
    // Standalone workflow execution engine
    WorkflowExecute => "workflow_execute",
    WorkflowStatus => "workflow_status",
]);

action_enum!(PerfAction, "perf", [
    Audit => "audit",
    Budget => "budget",
    Compare => "compare",
    Trace => "trace",
    VrtRun => "vrt_run",
    VrtCompare => "vrt_compare",
    VrtUpdate => "vrt_update",
    PixelDiff => "pixel_diff",
]);

// ──────────────── Tests ─────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_action_parse_valid() {
        assert_eq!(BrowserAction::parse("goto").unwrap(), BrowserAction::Goto);
        assert_eq!(BrowserAction::parse("emulate_network").unwrap(), BrowserAction::EmulateNetwork);
        assert_eq!(BrowserAction::parse("intercept_enable").unwrap(), BrowserAction::InterceptEnable);
    }

    #[test]
    fn browser_action_parse_invalid() {
        assert!(BrowserAction::parse("nonexistent").is_err());
    }

    #[test]
    fn crawl_action_parse() {
        assert_eq!(CrawlAction::parse("spider").unwrap(), CrawlAction::Spider);
        assert_eq!(CrawlAction::parse("dom_compare").unwrap(), CrawlAction::DomCompare);
    }

    #[test]
    fn agent_action_parse() {
        assert_eq!(AgentAction::parse("execute_chain").unwrap(), AgentAction::ExecuteChain);
        assert_eq!(AgentAction::parse("ios_screenshot").unwrap(), AgentAction::IosScreenshot);
    }

    #[test]
    fn all_tool_actions_parse() {
        assert_eq!(StealthAction::parse("inject").unwrap(), StealthAction::Inject);
        assert_eq!(DataAction::parse("pipeline").unwrap(), DataAction::Pipeline);
        assert_eq!(SecureAction::parse("encrypt").unwrap(), SecureAction::Encrypt);
        assert_eq!(ComputerAction::parse("act").unwrap(), ComputerAction::Act);
        assert_eq!(MemoryAction::parse("store").unwrap(), MemoryAction::Store);
        assert_eq!(AutomateAction::parse("plan").unwrap(), AutomateAction::Plan);
        assert_eq!(AutomateAction::parse("retry_adapt").unwrap(), AutomateAction::RetryAdapt);
        assert_eq!(AutomateAction::parse("error_classify").unwrap(), AutomateAction::ErrorClassify);
        assert_eq!(AutomateAction::parse("recovery_suggest").unwrap(), AutomateAction::RecoverySuggest);
        assert_eq!(AutomateAction::parse("error_history").unwrap(), AutomateAction::ErrorHistory);
        assert_eq!(PerfAction::parse("audit").unwrap(), PerfAction::Audit);
    }

    #[test]
    fn error_message_contains_available() {
        let err = BrowserAction::parse("bad").unwrap_err();
        let msg = err.message.to_string();
        assert!(msg.contains("unknown browser action"));
        assert!(msg.contains("goto"));
    }

    #[test]
    fn all_browser_actions_count() {
        let actions = [
            "goto", "click", "type", "screenshot", "pdf", "back", "forward",
            "reload", "wait", "evaluate", "snapshot", "css", "xpath", "find_text",
            "text", "html", "markdown", "structured", "stream", "detect_forms",
            "fill_form", "snapshot_diff", "parse_a11y", "parse_selector",
            "parse_text", "parse_links", "new_tab", "list_tabs", "switch_tab",
            "close_tab", "observe_mutations", "get_mutations", "stop_mutations",
            "wait_for_event", "cookies_get", "cookies_set", "cookies_clear",
            "storage_get", "storage_set", "export_session", "import_session",
            "intercept_enable", "intercept_add_rule", "intercept_remove_rule",
            "intercept_list", "intercept_disable", "block_requests",
            "console_start", "console_get", "console_clear", "dialog_handle",
            "dialog_get", "errors_get", "emulate_device", "emulate_geolocation",
            "emulate_timezone", "emulate_media", "emulate_network",
            "drag", "hover", "keyboard", "select",
            "upload", "download_wait", "download_list", "download_set_dir",
            "shadow_query", "shadow_text", "deep_query",
            "context_set", "context_get", "context_list", "context_clear",
            "context_transfer", "form_infer", "form_auto_fill", "form_validate",
            "selector_heal", "selector_alternatives", "selector_validate",
            "event_subscribe", "event_unsubscribe", "event_poll", "event_clear",
        ];
        assert_eq!(actions.len(), 84);
        for a in &actions {
            assert!(BrowserAction::parse(a).is_ok(), "failed to parse: {a}");
        }
    }

    // ── Self-Healing Selector action variants ──

    #[test]
    fn browser_selector_variants() {
        assert_eq!(BrowserAction::parse("selector_heal").unwrap(), BrowserAction::SelectorHeal);
        assert_eq!(BrowserAction::parse("selector_alternatives").unwrap(), BrowserAction::SelectorAlternatives);
        assert_eq!(BrowserAction::parse("selector_validate").unwrap(), BrowserAction::SelectorValidate);
    }

    // ── Event-Driven Reaction action variants ──

    #[test]
    fn browser_event_variants() {
        assert_eq!(BrowserAction::parse("event_subscribe").unwrap(), BrowserAction::EventSubscribe);
        assert_eq!(BrowserAction::parse("event_unsubscribe").unwrap(), BrowserAction::EventUnsubscribe);
        assert_eq!(BrowserAction::parse("event_poll").unwrap(), BrowserAction::EventPoll);
        assert_eq!(BrowserAction::parse("event_clear").unwrap(), BrowserAction::EventClear);
    }

    // ── Agent Task/Vision action variants ──

    #[test]
    fn agent_task_variants() {
        assert_eq!(AgentAction::parse("task_decompose").unwrap(), AgentAction::TaskDecompose);
        assert_eq!(AgentAction::parse("task_plan").unwrap(), AgentAction::TaskPlan);
        assert_eq!(AgentAction::parse("task_status").unwrap(), AgentAction::TaskStatus);
    }

    #[test]
    fn agent_vision_variants() {
        assert_eq!(AgentAction::parse("vision_describe").unwrap(), AgentAction::VisionDescribe);
        assert_eq!(AgentAction::parse("vision_locate").unwrap(), AgentAction::VisionLocate);
        assert_eq!(AgentAction::parse("vision_compare").unwrap(), AgentAction::VisionCompare);
    }

    // ── Automate Checkpoint action variants ──

    #[test]
    fn automate_checkpoint_variants() {
        assert_eq!(AutomateAction::parse("checkpoint_save").unwrap(), AutomateAction::CheckpointSave);
        assert_eq!(AutomateAction::parse("checkpoint_restore").unwrap(), AutomateAction::CheckpointRestore);
        assert_eq!(AutomateAction::parse("checkpoint_list").unwrap(), AutomateAction::CheckpointList);
        assert_eq!(AutomateAction::parse("checkpoint_delete").unwrap(), AutomateAction::CheckpointDelete);
    }

    // ── Automate Workflow Control action variants ──

    #[test]
    fn automate_workflow_control_variants() {
        assert_eq!(AutomateAction::parse("workflow_while").unwrap(), AutomateAction::WorkflowWhile);
        assert_eq!(AutomateAction::parse("workflow_for_each").unwrap(), AutomateAction::WorkflowForEach);
        assert_eq!(AutomateAction::parse("workflow_if").unwrap(), AutomateAction::WorkflowIf);
        assert_eq!(AutomateAction::parse("workflow_variable").unwrap(), AutomateAction::WorkflowVariable);
    }
}
