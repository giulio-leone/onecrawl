//! OneCrawl CDP — Browser automation via Chrome DevTools Protocol.
//!
//! Wraps `chromiumoxide` to provide high-level browser commands.

pub mod accessibility;
pub mod advanced_emulation;
pub mod benchmark;
pub mod bridge;
pub mod browser;
pub mod console;
pub mod cookie;
pub mod cookie_jar;
pub mod coverage;
pub mod dialog;
pub mod dom_nav;
pub mod dom_observer;
pub mod downloads;
pub mod element;
pub mod emulation;
pub mod events;
pub mod extract;
pub mod geofencing;
pub mod har;
pub mod iframe;
pub mod input;
pub mod intercept;
pub mod keyboard;
pub mod navigation;
pub mod network;
pub mod network_log;
pub mod page;
pub mod page_watcher;
#[cfg(feature = "playwright")]
pub mod playwright_backend;
pub mod print;
pub mod proxy;
pub mod request_queue;
pub mod screenshot;
pub mod screenshot_diff;
pub mod selectors;
pub mod stealth;
pub mod tabs;
pub mod throttle;
pub mod tracing_cdp;
pub mod web_storage;
pub mod webauthn;
pub mod websocket;
pub mod workers;

pub use accessibility::AccessibilityAudit;
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
pub use intercept::{InterceptAction, InterceptRule};
pub use network::ResourceType;
pub use network_log::{NetworkEntry, NetworkSummary};
pub use print::DetailedPdfOptions;
pub use proxy::{ProxyConfig, ProxyPool, RotationStrategy};
pub use screenshot::{ImageFormat, PdfOptions, ScreenshotOptions};
pub use stealth::{Fingerprint, generate_fingerprint, get_stealth_init_script};
pub use throttle::NetworkProfile;
pub use tracing_cdp::PerformanceMetric;
pub use webauthn::{VirtualAuthenticator, VirtualCredential};
pub use websocket::WsRecorder;
pub use workers::ServiceWorkerInfo;

pub use cookie_jar::{CookieJar, StoredCookie};
pub use dom_nav::NavElement;
pub use downloads::DownloadInfo;
pub use extract::{ExtractFormat, ExtractResult, LinkInfo};
pub use geofencing::GeoProfile;
pub use request_queue::{QueueConfig, QueuedRequest, RequestResult};
pub use screenshot_diff::DiffResult;
pub use selectors::{ElementData, SelectorResult};
pub use page_watcher::PageChange;
pub use tabs::TabInfo;

// Re-export chromiumoxide::Page for downstream consumers
pub use chromiumoxide::Page;
