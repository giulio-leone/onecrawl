//! OneCrawl CDP — Browser automation via Chrome DevTools Protocol.
//!
//! Wraps `chromiumoxide` to provide high-level browser commands.

pub mod accessibility;
pub mod adaptive;
pub mod agent;
pub mod agent_auto;
pub mod agent_memory;
pub mod android;
pub mod annotated;
pub mod browser_pool;
pub mod computer_use;
pub mod adaptive_fetch;
pub mod advanced_emulation;
pub mod antibot;
pub mod benchmark;
pub mod bridge;
pub mod browser;
pub mod captcha;
pub mod console;
pub mod cookie;
pub mod cookie_jar;
pub mod coverage;
pub mod data_pipeline;
pub mod dialog;
pub mod dom_nav;
pub mod dom_observer;
pub mod durable;
pub mod reactor;
pub mod domain_blocker;
pub mod downloads;
pub mod element;
pub mod emulation;
pub mod event_bus;
pub mod events;
pub mod extract;
pub mod form_filler;
pub mod geofencing;
pub mod http_client;

pub mod har;
pub mod harness;
pub mod human;
pub mod iframe;
pub mod input;
pub mod ios;
pub mod intercept;
pub mod keyboard;
pub mod link_graph;
pub mod navigation;
pub mod network;
pub mod network_intel;
pub mod orchestrator;
pub mod network_log;
pub mod page;
pub mod page_watcher;
pub mod perf_monitor;
#[cfg(feature = "playwright")]
pub mod playwright_backend;
pub mod print;
pub mod proxy;
pub mod recording;
pub mod screencast;
pub mod proxy_health;
pub mod rate_limiter;
pub mod request_queue;
pub mod retry_queue;
pub mod robots;
pub mod scheduler;
pub mod screenshot;
pub mod screenshot_diff;
pub mod snapshot_diff;
pub mod spa;
pub mod selectors;
pub mod session_pool;
pub mod shell;
pub mod skills;
pub mod sitemap;
pub mod smart_actions;
pub mod snapshot;
pub mod spider;
pub mod stealth;
pub mod streaming;
pub mod structured_data;
pub mod tabs;
pub mod task_planner;
pub mod throttle;
pub mod tls_fingerprint;
pub mod tracing_cdp;
pub mod web_storage;
pub mod passkey_store;
pub mod safety;
pub mod webauthn;
pub mod websocket;
pub mod workers;
pub mod vrt;
pub mod pixel_diff;
pub mod workflow;

pub use browser_pool::{BrowserInstance, BrowserPool, BrowserStatus, SharedPool, new_shared_pool};
pub use smart_actions::SmartMatch;

pub use agent_memory::{AgentMemory, MemoryCategory, MemoryEntry, MemoryStats, DomainStrategy, PageVisit, ElementPattern};
pub use accessibility::AccessibilityAudit;
pub use adaptive::{ElementFingerprint, ElementMatch, TrackedElement};
pub use antibot::AntibotProfile;
pub use benchmark::{BenchmarkResult, BenchmarkSuite};
pub use bridge::PlaywrightBridge;
pub use browser::BrowserSession;
pub use console::ConsoleEntry;
pub use cookie::{Cookie, SetCookieParams};
pub use coverage::CoverageReport;
pub use dialog::DialogEvent;
pub use dom_observer::DomMutation;
pub use emulation::Viewport;
pub use events::{BrowserEvent, EventStream, EventType};
pub use har::HarRecorder;
pub use iframe::IframeInfo;
pub use ios::{IosClient, IosDevice, IosSessionConfig};
pub use android::{AndroidClient, AndroidSessionConfig};
pub use intercept::{InterceptAction, InterceptRule};
pub use network::ResourceType;
pub use network_intel::{ApiEndpoint, ApiSchema, ApiCategory, SdkStub, MockServerConfig, ReplaySequence};
pub use network_log::{NetworkEntry, NetworkSummary};
pub use print::DetailedPdfOptions;
pub use proxy::{ProxyConfig, ProxyPool, RotationStrategy};
pub use screenshot::{ImageFormat, PdfOptions, ScreenshotOptions};
pub use screencast::{ScreencastOptions, StreamResult};
pub use recording::{RecordingState, SharedRecording, new_shared_recording, VideoResult};
pub use stealth::{Fingerprint, generate_fingerprint, generate_fingerprint_with_real_ua, get_stealth_init_script, inject_persistent_stealth};
pub use throttle::NetworkProfile;
pub use tracing_cdp::PerformanceMetric;
pub use webauthn::{
    VirtualAuthenticator, VirtualCredential,
    PasskeyCredential, cdp_enable, cdp_create_authenticator,
    cdp_get_credentials, cdp_add_credential, save_passkeys, load_passkeys,
};
pub use passkey_store::{
    PasskeyVault,
    load_vault, save_vault, vault_add, vault_get, vault_remove,
    vault_clear_site, vault_list, vault_total, vault_path,
    import_bitwarden, import_cxf, import_1password_json,
};
pub use websocket::WsRecorder;
pub use skills::{SkillManifest, SkillRegistry, SkillTool};
pub use workers::ServiceWorkerInfo;
pub use vrt::{VrtComparisonResult, VrtStatus, VrtSuite, VrtSuiteResult, VrtTestCase};
pub use workflow::{Workflow, WorkflowResult, Step, Action, StepResult, StepStatus, AgentStepContext, AgentDecision};
pub use task_planner::{TaskPlan, PlannedStep, PlannedAction, TaskExecutionResult, TaskStatus, GoalCategory};
pub use safety::{SafetyCheck, SafetyPolicy, SafetyState};
pub use agent_auto::{AgentAuto, AgentAutoConfig, AgentAutoResult, AgentAutoState, AutoStep, StepStatus as AutoStepStatus, OutputFormat as AutoOutputFormat, agent_auto_run, agent_auto_plan};

pub use captcha::{CaptchaConfig, CaptchaDetection, CaptchaResult, SolverConfig, SolverService, solve_via_api, load_solver_config};
pub use cookie_jar::{CookieJar, StoredCookie};
pub use data_pipeline::{Pipeline, PipelineResult, PipelineStep};
pub use dom_nav::NavElement;
pub use domain_blocker::{BlockStats, BlockedDomain};
pub use downloads::DownloadInfo;
pub use extract::{ExtractFormat, ExtractResult, LinkInfo};
pub use geofencing::GeoProfile;
pub use http_client::{HttpRequest, HttpResponse};
pub use link_graph::{LinkEdge, LinkGraph, LinkNode, LinkStats};
pub use page_watcher::PageChange;
pub use perf_monitor::{CoreWebVitals, PerfSnapshot, PerfBudget, BudgetResult, Regression};
pub use proxy_health::{ProxyHealthConfig, ProxyHealthResult};
pub use rate_limiter::{RateLimitConfig, RateLimitState, RateLimitStats};
pub use request_queue::{QueueConfig, QueuedRequest, RequestResult};
pub use retry_queue::{QueueStats as RetryQueueStats, RetryConfig, RetryItem, RetryQueue};
pub use robots::{RobotsRule, RobotsTxt};
pub use scheduler::{ScheduledTask, Scheduler, TaskResult, TaskSchedule};
pub use screenshot_diff::{DiffResult, PixelDiffResult};
pub use snapshot_diff::AccessibilitySnapshotDiff;
pub use selectors::{ElementData, SelectorResult};
pub use session_pool::{PoolConfig, PoolStats, SessionInfo, SessionPool};
pub use shell::{ShellCommand, ShellHistory};
pub use snapshot::{DomSnapshot, SnapshotDiff};
pub use spider::{CrawlResult, CrawlState, CrawlSummary, SpiderConfig};
pub use streaming::{
    ExtractedItem, ExtractionResult, ExtractionRule, ExtractionSchema, PaginationConfig,
};
pub use structured_data::{
    JsonLdData, OpenGraphData, PageMetadata, StructuredDataResult, TwitterCardData,
};
pub use tabs::TabInfo;
pub use tls_fingerprint::BrowserFingerprint;

pub use durable::{CrashPolicy, DurableConfig, DurableSession, DurableState, DurableStatus, parse_duration};
pub use reactor::{EventFilter, Reactor, ReactorConfig, ReactorEvent, ReactorEventType, ReactorHandler, ReactorRule, ReactorStatus, RuleStatus};
pub use event_bus::{EventBus, EventBusConfig, BusEvent, BusStats, WebhookSubscription, DeliveryStatus, JournalEntry};

pub use orchestrator::{
    Orchestrator, Orchestration, OrchestrationResult, DeviceType, DeviceConfig,
    DeviceAction, OrchAction, OrchStep, ErrorPolicy,
    StepResult as OrchStepResult, DeviceActionResult,
    StepResultStatus, DeviceHandle,
};

// Re-export chromiumoxide::Page for downstream consumers
pub use chromiumoxide::Page;
