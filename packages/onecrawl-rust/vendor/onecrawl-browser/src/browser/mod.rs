//! Browser management — launching, connecting, and driving Chrome.

use futures::channel::mpsc::{unbounded, Sender};
use futures::channel::oneshot::channel as oneshot_channel;
use futures::SinkExt;

use onecrawl_protocol::cdp::browser_protocol::target::{
    CreateBrowserContextParams, CreateTargetParams, DisposeBrowserContextParams, TargetId,
    TargetInfo,
};
use onecrawl_protocol::cdp::IntoEventKind;
use onecrawl_browser_types::*;

use crate::async_process::Child;
use crate::cmd::{to_command_response, CommandMessage};
use crate::error::{CdpError, Result};
use crate::handler::browser::BrowserContext;
use crate::handler::HandlerMessage;
use crate::listeners::{EventListenerRequest, EventStream};
use crate::page::Page;
use onecrawl_protocol::cdp::browser_protocol::browser::{
    BrowserContextId, CloseReturns, GetVersionParams, GetVersionReturns,
};

// ── Sub-modules ──

mod launcher;
mod process;
mod cookies;

// ── Re-exports (public API must remain at crate::browser::*) ──

pub use launcher::{
    BrowserConfig, BrowserConfigBuilder, BrowserConnection, HeadlessMode, LAUNCH_TIMEOUT,
};

#[allow(deprecated)]
pub use launcher::default_executable;

/// A [`Browser`] is created when onecrawl_browser connects to a Chromium instance.
#[derive(Debug)]
pub struct Browser {
    /// The `Sender` to send messages to the connection handler that drives the
    /// websocket
    pub(super) sender: Sender<HandlerMessage>,
    /// How the spawned chromium instance was configured, if any
    pub(super) config: Option<BrowserConfig>,
    /// The spawned chromium instance
    pub(super) child: Option<Child>,
    /// The debug web socket url of the chromium instance
    pub(super) debug_ws_url: String,
    /// The context of the browser
    pub(super) browser_context: BrowserContext,
}

impl Browser {
    /// Request to fetch all existing browser targets.
    ///
    /// By default, only targets launched after the browser connection are tracked
    /// when connecting to a existing browser instance with the devtools websocket url
    /// This function fetches existing targets on the browser and adds them as pages internally
    ///
    /// The pages are not guaranteed to be ready as soon as the function returns
    /// You should wait a few millis if you need to use a page
    /// Returns [TargetInfo]
    pub async fn fetch_targets(&mut self) -> Result<Vec<TargetInfo>> {
        let (tx, rx) = oneshot_channel();

        self.sender
            .clone()
            .send(HandlerMessage::FetchTargets(tx))
            .await?;

        rx.await?
    }

    /// Request for the browser to close completely.
    ///
    /// If the browser was spawned by [`Browser::launch`], it is recommended to wait for the
    /// spawned instance exit, to avoid "zombie" processes ([`Browser::wait`],
    /// [`Browser::wait_sync`], [`Browser::try_wait`]).
    /// [`Browser::drop`] waits automatically if needed.
    pub async fn close(&mut self) -> Result<CloseReturns> {
        let (tx, rx) = oneshot_channel();

        self.sender
            .clone()
            .send(HandlerMessage::CloseBrowser(tx))
            .await?;

        rx.await?
    }

    /// If not launched as incognito this creates a new incognito browser
    /// context. After that this browser exists within the incognito session.
    /// New pages created while being in incognito mode will also run in the
    /// incognito context. Incognito contexts won't share cookies/cache with
    /// other browser contexts.
    pub async fn start_incognito_context(&mut self) -> Result<&mut Self> {
        if !self.is_incognito_configured() {
            let browser_context_id = self
                .create_browser_context(CreateBrowserContextParams::default())
                .await?;
            self.browser_context = BrowserContext::from(browser_context_id);
            self.sender
                .clone()
                .send(HandlerMessage::InsertContext(self.browser_context.clone()))
                .await?;
        }

        Ok(self)
    }

    /// If a incognito session was created with
    /// `Browser::start_incognito_context` this disposes this context.
    ///
    /// # Note This will also dispose all pages that were running within the
    /// incognito context.
    pub async fn quit_incognito_context(&mut self) -> Result<&mut Self> {
        if let Some(id) = self.browser_context.take() {
            self.dispose_browser_context(id.clone()).await?;
            self.sender
                .clone()
                .send(HandlerMessage::DisposeContext(BrowserContext::from(id)))
                .await?;
        }
        Ok(self)
    }

    /// Whether incognito mode was configured from the start
    fn is_incognito_configured(&self) -> bool {
        self.config
            .as_ref()
            .map(|c| c.incognito)
            .unwrap_or_default()
    }

    /// Returns the address of the websocket this browser is attached to
    pub fn websocket_address(&self) -> &String {
        &self.debug_ws_url
    }

    /// Whether the BrowserContext is incognito.
    pub fn is_incognito(&self) -> bool {
        self.is_incognito_configured() || self.browser_context.is_incognito()
    }

    /// The config of the spawned chromium instance if any.
    pub fn config(&self) -> Option<&BrowserConfig> {
        self.config.as_ref()
    }

    /// Create a new browser page
    pub async fn new_page(&self, params: impl Into<CreateTargetParams>) -> Result<Page> {
        let (tx, rx) = oneshot_channel();
        let mut params = params.into();
        if let Some(id) = self.browser_context.id() {
            if params.browser_context_id.is_none() {
                params.browser_context_id = Some(id.clone());
            }
        }

        self.sender
            .clone()
            .send(HandlerMessage::CreatePage(params, tx))
            .await?;

        rx.await?
    }

    /// Version information about the browser
    pub async fn version(&self) -> Result<GetVersionReturns> {
        Ok(self.execute(GetVersionParams::default()).await?.result)
    }

    /// Returns the user agent of the browser
    pub async fn user_agent(&self) -> Result<String> {
        Ok(self.version().await?.user_agent)
    }

    /// Call a browser method.
    pub async fn execute<T: Command>(&self, cmd: T) -> Result<CommandResponse<T::Response>> {
        let (tx, rx) = oneshot_channel();
        let method = cmd.identifier();
        let msg = CommandMessage::new(cmd, tx)?;

        self.sender
            .clone()
            .send(HandlerMessage::Command(Box::new(msg)))
            .await?;
        let resp = rx.await??;
        to_command_response::<T>(resp, method)
    }

    /// Return all of the pages of the browser
    pub async fn pages(&self) -> Result<Vec<Page>> {
        let (tx, rx) = oneshot_channel();
        self.sender
            .clone()
            .send(HandlerMessage::GetPages(tx))
            .await?;
        Ok(rx.await?)
    }

    /// Return page of given target_id
    pub async fn get_page(&self, target_id: TargetId) -> Result<Page> {
        let (tx, rx) = oneshot_channel();
        self.sender
            .clone()
            .send(HandlerMessage::GetPage(target_id, tx))
            .await?;
        rx.await?.ok_or(CdpError::NotFound)
    }

    /// Set listener for browser event
    pub async fn event_listener<T: IntoEventKind>(&self) -> Result<EventStream<T>> {
        let (tx, rx) = unbounded();
        self.sender
            .clone()
            .send(HandlerMessage::AddEventListener(
                EventListenerRequest::new::<T>(tx),
            ))
            .await?;

        Ok(EventStream::new(rx))
    }

    /// Creates a new empty browser context.
    pub async fn create_browser_context(
        &self,
        params: CreateBrowserContextParams,
    ) -> Result<BrowserContextId> {
        let response = self.execute(params).await?;
        Ok(response.result.browser_context_id)
    }

    /// Deletes a browser context.
    pub async fn dispose_browser_context(
        &self,
        browser_context_id: impl Into<BrowserContextId>,
    ) -> Result<()> {
        self.execute(DisposeBrowserContextParams::new(browser_context_id))
            .await?;

        Ok(())
    }
}
