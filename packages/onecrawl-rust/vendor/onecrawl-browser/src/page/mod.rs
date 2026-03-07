//! Page type and core CDP execution methods.
//!
//! Sub-modules group related functionality:
//! - [`navigation`] — goto, reload, wait_for_navigation, url, activate, close
//! - [`evaluation`] — JS evaluate, expose_function, execution contexts
//! - [`dom`] — find_element(s), content, set_content
//! - [`capture`] — screenshot, pdf
//! - [`cookies`] — get/set/delete cookies
//! - [`emulation`] — user-agent, stealth, timezone, locale, geolocation
//! - [`frames`] — frame_name, frame_url, mainframe, frames
//! - [`protocol`] — enable/disable log, runtime, debugger, DOM, CSS

mod capture;
mod cookies;
mod dom;
mod emulation;
mod evaluation;
mod frames;
mod navigation;
mod protocol;

use std::sync::Arc;

use futures::channel::mpsc::unbounded;
use futures::SinkExt;

use onecrawl_protocol::cdp::browser_protocol::page::{
    CaptureScreenshotFormat, CaptureScreenshotParams, GetLayoutMetricsReturns, Viewport,
};
use onecrawl_protocol::cdp::browser_protocol::performance::{GetMetricsParams, Metric};
use onecrawl_protocol::cdp::browser_protocol::target::{SessionId, TargetId};
use onecrawl_protocol::cdp::IntoEventKind;
use onecrawl_browser_types::*;

use crate::error::{CdpError, Result};
use crate::handler::commandfuture::CommandFuture;
use crate::handler::httpfuture::HttpFuture;
use crate::handler::target::TargetMessage;
use crate::handler::PageInner;
use crate::layout::Point;
use crate::listeners::{EventListenerRequest, EventStream};

#[derive(Debug, Clone)]
pub struct Page {
    pub(super) inner: Arc<PageInner>,
}

impl Page {
    /// Execute a command and return the `Command::Response`
    pub async fn execute<T: Command>(&self, cmd: T) -> Result<CommandResponse<T::Response>> {
        self.command_future(cmd)?.await
    }

    /// Execute a command and return the `Command::Response`
    pub fn command_future<T: Command>(&self, cmd: T) -> Result<CommandFuture<T>> {
        self.inner.command_future(cmd)
    }

    /// Execute a command and return the `Command::Response`
    pub fn http_future<T: Command>(&self, cmd: T) -> Result<HttpFuture<T>> {
        self.inner.http_future(cmd)
    }

    /// Adds an event listener to the `Target` and returns the receiver part as
    /// `EventStream`
    ///
    /// An `EventStream` receives every `Event` the `Target` receives.
    /// All event listener get notified with the same event, so registering
    /// multiple listeners for the same event is possible.
    ///
    /// Custom events rely on being deserializable from the received json params
    /// in the `EventMessage`. Custom Events are caught by the `CdpEvent::Other`
    /// variant. If there are mulitple custom event listener is registered
    /// for the same event, identified by the `MethodType::method_id` function,
    /// the `Target` tries to deserialize the json using the type of the event
    /// listener. Upon success the `Target` then notifies all listeners with the
    /// deserialized event. This means, while it is possible to register
    /// different types for the same custom event, only the type of first
    /// registered event listener will be used. The subsequent listeners, that
    /// registered for the same event but with another type won't be able to
    /// receive anything and therefor will come up empty until all their
    /// preceding event listeners are dropped and they become the first (or
    /// longest) registered event listener for an event.
    ///
    /// # Example Listen for canceled animations
    /// ```no_run
    /// # use onecrawl_browser::page::Page;
    /// # use onecrawl_browser::error::Result;
    /// # use onecrawl_protocol::cdp::browser_protocol::animation::EventAnimationCanceled;
    /// # use futures::StreamExt;
    /// # async fn demo(page: Page) -> Result<()> {
    ///     let mut events = page.event_listener::<EventAnimationCanceled>().await?;
    ///     while let Some(event) = events.next().await {
    ///         //..
    ///     }
    ///     # Ok(())
    /// # }
    /// ```
    ///
    /// # Example Liste for a custom event
    ///
    /// ```no_run
    /// # use onecrawl_browser::page::Page;
    /// # use onecrawl_browser::error::Result;
    /// # use futures::StreamExt;
    /// # use serde::Deserialize;
    /// # use onecrawl_browser::types::{MethodId, MethodType};
    /// # use onecrawl_browser::cdp::CustomEvent;
    /// # async fn demo(page: Page) -> Result<()> {
    ///     #[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
    ///     struct MyCustomEvent {
    ///         name: String,
    ///     }
    ///    impl MethodType for MyCustomEvent {
    ///        fn method_id() -> MethodId {
    ///            "Custom.Event".into()
    ///        }
    ///    }
    ///    impl CustomEvent for MyCustomEvent {}
    ///    let mut events = page.event_listener::<MyCustomEvent>().await?;
    ///    while let Some(event) = events.next().await {
    ///        //..
    ///    }
    ///
    ///     # Ok(())
    /// # }
    /// ```
    pub async fn event_listener<T: IntoEventKind>(&self) -> Result<EventStream<T>> {
        let (tx, rx) = unbounded();
        self.inner
            .sender()
            .clone()
            .send(TargetMessage::AddEventListener(
                EventListenerRequest::new::<T>(tx),
            ))
            .await?;

        Ok(EventStream::new(rx))
    }

    /// The identifier of the `Target` this page is attached to
    pub fn target_id(&self) -> &TargetId {
        self.inner.target_id()
    }

    /// The identifier of the `Session` target of this page is attached to
    pub fn session_id(&self) -> &SessionId {
        self.inner.session_id()
    }

    /// The identifier of the `Session` target of this page is attached to
    pub fn opener_id(&self) -> &Option<TargetId> {
        self.inner.opener_id()
    }

    /// Returns the title of the document.
    pub async fn get_title(&self) -> Result<Option<String>> {
        let result = self.evaluate("document.title").await?;

        let title: String = result.into_value()?;

        if title.is_empty() {
            Ok(None)
        } else {
            Ok(Some(title))
        }
    }

    /// Retrieve current values of run-time metrics.
    pub async fn metrics(&self) -> Result<Vec<Metric>> {
        Ok(self
            .execute(GetMetricsParams::default())
            .await?
            .result
            .metrics)
    }

    /// Returns metrics relating to the layout of the page
    pub async fn layout_metrics(&self) -> Result<GetLayoutMetricsReturns> {
        self.inner.layout_metrics().await
    }

    /// Dispatches a `mousePressed`, `mouseReleased` event and sends a `click`
    /// at the `point`'s coordinate.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use onecrawl_browser::page::Page;
    /// # use onecrawl_browser::error::Result;
    /// # use onecrawl_browser::layout::Point;
    /// # async fn demo(page: Page) -> Result<()> {
    ///     // Click somewhere on the page
    ///     page.click(Point::new(120., 160.)).await?;
    ///
    ///     // Using the `ElementHandle`
    ///     let element = page.find_element("button").await?;
    ///     element.click().await?;
    ///
    ///     // Using the `ElementHandle` to click at specific point
    ///     let point = element.clickable_point().await?;
    ///     page.click(point).await?;
    ///
    ///     // Dispatching a `click` at a position with custom options using `DispatchMouseEventParams`
    ///     use onecrawl_protocol::cdp::browser_protocol::input::{
    ///         DispatchMouseEventParams, DispatchMouseEventType, MouseButton,
    ///     };
    ///
    ///     let cmd = DispatchMouseEventParams::builder()
    ///         .x(120.)
    ///         .y(160.)
    ///         .button(MouseButton::Left)
    ///         .click_count(1);
    ///
    ///         page.execute(
    ///             cmd.clone()
    ///                 .r#type(DispatchMouseEventType::MousePressed)
    ///                 .build()
    ///                 .unwrap(),
    ///         )
    ///         .await?;
    ///
    ///         page.execute(
    ///             cmd.r#type(DispatchMouseEventType::MouseReleased)
    ///                 .build()
    ///                 .unwrap(),
    ///         )
    ///         .await?;
    ///
    ///     # Ok(())
    /// # }
    /// ```
    pub async fn click(&self, point: Point) -> Result<&Self> {
        self.inner.click(point).await?;
        Ok(self)
    }

    /// Dispatches a `mousemove` event and moves the mouse to the position of
    /// the `point` where `Point.x` is the horizontal position of the mouse and
    /// `Point.y` the vertical position of the mouse.
    pub async fn move_mouse(&self, point: Point) -> Result<&Self> {
        self.inner.move_mouse(point).await?;
        Ok(self)
    }
}

impl From<Arc<PageInner>> for Page {
    fn from(inner: Arc<PageInner>) -> Self {
        Self { inner }
    }
}

pub(crate) fn validate_cookie_url(url: &str) -> Result<()> {
    if url.starts_with("data:") {
        Err(CdpError::msg("Data URL page can not have cookie"))
    } else if url == "about:blank" {
        Err(CdpError::msg("Blank page can not have cookie"))
    } else {
        Ok(())
    }
}

/// Page screenshot parameters with extra options.
#[derive(Debug, Default)]
pub struct ScreenshotParams {
    /// Chrome DevTools Protocol screenshot options.
    pub cdp_params: CaptureScreenshotParams,
    /// Take full page screenshot.
    pub full_page: Option<bool>,
    /// Make the background transparent (png only).
    pub omit_background: Option<bool>,
}

impl ScreenshotParams {
    pub fn builder() -> ScreenshotParamsBuilder {
        Default::default()
    }

    pub(crate) fn full_page(&self) -> bool {
        self.full_page.unwrap_or(false)
    }

    pub(crate) fn omit_background(&self) -> bool {
        self.omit_background.unwrap_or(false)
            && self
                .cdp_params
                .format
                .as_ref()
                .is_none_or(|f| f == &CaptureScreenshotFormat::Png)
    }
}

/// Page screenshot parameters builder with extra options.
#[derive(Debug, Default)]
pub struct ScreenshotParamsBuilder {
    cdp_params: CaptureScreenshotParams,
    full_page: Option<bool>,
    omit_background: Option<bool>,
}

impl ScreenshotParamsBuilder {
    /// Image compression format (defaults to png).
    pub fn format(mut self, format: impl Into<CaptureScreenshotFormat>) -> Self {
        self.cdp_params.format = Some(format.into());
        self
    }

    /// Compression quality from range [0..100] (jpeg only).
    pub fn quality(mut self, quality: impl Into<i64>) -> Self {
        self.cdp_params.quality = Some(quality.into());
        self
    }

    /// Capture the screenshot of a given region only.
    pub fn clip(mut self, clip: impl Into<Viewport>) -> Self {
        self.cdp_params.clip = Some(clip.into());
        self
    }

    /// Capture the screenshot from the surface, rather than the view (defaults to true).
    pub fn from_surface(mut self, from_surface: impl Into<bool>) -> Self {
        self.cdp_params.from_surface = Some(from_surface.into());
        self
    }

    /// Capture the screenshot beyond the viewport (defaults to false).
    pub fn capture_beyond_viewport(mut self, capture_beyond_viewport: impl Into<bool>) -> Self {
        self.cdp_params.capture_beyond_viewport = Some(capture_beyond_viewport.into());
        self
    }

    /// Full page screen capture.
    pub fn full_page(mut self, full_page: impl Into<bool>) -> Self {
        self.full_page = Some(full_page.into());
        self
    }

    /// Make the background transparent (png only)
    pub fn omit_background(mut self, omit_background: impl Into<bool>) -> Self {
        self.omit_background = Some(omit_background.into());
        self
    }

    pub fn build(self) -> ScreenshotParams {
        ScreenshotParams {
            cdp_params: self.cdp_params,
            full_page: self.full_page,
            omit_background: self.omit_background,
        }
    }
}

impl From<CaptureScreenshotParams> for ScreenshotParams {
    fn from(cdp_params: CaptureScreenshotParams) -> Self {
        Self {
            cdp_params,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum MediaTypeParams {
    /// Default CSS media type behavior for page and print
    #[default]
    Null,
    /// Force screen CSS media type for page and print
    Screen,
    /// Force print CSS media type for page and print
    Print,
}
impl From<MediaTypeParams> for String {
    fn from(media_type: MediaTypeParams) -> Self {
        match media_type {
            MediaTypeParams::Null => "null".to_string(),
            MediaTypeParams::Screen => "screen".to_string(),
            MediaTypeParams::Print => "print".to_string(),
        }
    }
}
