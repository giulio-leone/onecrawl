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
pub mod coverage;
pub mod dialog;
pub mod dom_observer;
pub mod downloads;
pub mod element;
pub mod emulation;
pub mod events;
pub mod har;
pub mod iframe;
pub mod input;
pub mod intercept;
pub mod keyboard;
pub mod navigation;
pub mod network;
pub mod page;
pub mod tabs;
#[cfg(feature = "playwright")]
pub mod playwright_backend;
pub mod print;
pub mod proxy;
pub mod screenshot;
pub mod screenshot_diff;
pub mod stealth;
pub mod throttle;
pub mod tracing_cdp;
pub mod web_storage;
pub mod webauthn;
pub mod websocket;
pub mod workers;
pub mod geofencing;
pub mod cookie_jar;
pub mod request_queue;

pub use accessibility::AccessibilityAudit;
pub use benchmark::{BenchmarkResult, BenchmarkSuite};
pub use browser::BrowserSession;
pub use bridge::PlaywrightBridge;
pub use console::ConsoleEntry;
pub use cookie::{Cookie, SetCookieParams};
pub use coverage::CoverageReport;
pub use dialog::DialogEvent;
pub use emulation::Viewport;
pub use events::{BrowserEvent, EventStream, EventType};
pub use har::HarRecorder;
pub use network::ResourceType;
pub use screenshot::{ImageFormat, PdfOptions, ScreenshotOptions};
pub use stealth::{Fingerprint, generate_fingerprint, get_stealth_init_script};
pub use throttle::NetworkProfile;
pub use tracing_cdp::PerformanceMetric;
pub use websocket::WsRecorder;
pub use workers::ServiceWorkerInfo;
pub use dom_observer::DomMutation;
pub use iframe::IframeInfo;
pub use intercept::{InterceptAction, InterceptRule};
pub use print::DetailedPdfOptions;
pub use proxy::{ProxyConfig, ProxyPool, RotationStrategy};
pub use webauthn::{VirtualAuthenticator, VirtualCredential};

pub use tabs::TabInfo;
pub use downloads::DownloadInfo;
pub use screenshot_diff::DiffResult;
pub use geofencing::GeoProfile;
pub use cookie_jar::{CookieJar, StoredCookie};
pub use request_queue::{QueuedRequest, RequestResult, QueueConfig};

// Re-export chromiumoxide::Page for downstream consumers
pub use chromiumoxide::Page;
