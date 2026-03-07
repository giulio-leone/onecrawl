use std::collections::VecDeque;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::ready;

use async_tungstenite::tungstenite::Message as WsMessage;
use async_tungstenite::{tungstenite::protocol::WebSocketConfig, WebSocketStream};
use futures::stream::Stream;
use futures::task::{Context, Poll};
use futures::{SinkExt, StreamExt};

use onecrawl_protocol::cdp::browser_protocol::target::SessionId;
use onecrawl_browser_types::{CallId, EventMessage, Message, MethodCall, MethodId};

use crate::error::CdpError;
use crate::error::Result;

cfg_if::cfg_if! {
    if #[cfg(feature = "async-std-runtime")] {
       use async_tungstenite::async_std::ConnectStream;
    } else if #[cfg(feature = "tokio-runtime")] {
        use async_tungstenite::tokio::ConnectStream;
    }
}

/// Tracks the state of the current outbound message on the WebSocket sink.
#[derive(Debug)]
enum FlushState {
    /// No message is in flight.
    Idle,
    /// A message has been enqueued via `start_send` and awaits `poll_ready`.
    Pending(MethodCall),
    /// `poll_ready` succeeded; the sink needs a `poll_flush`.
    NeedsFlush,
}

/// Exchanges the messages with the websocket
#[must_use = "streams do nothing unless polled"]
#[derive(Debug)]
pub struct Connection<T: EventMessage> {
    /// Queue of commands to send.
    pending_commands: VecDeque<MethodCall>,
    /// The websocket of the chromium instance
    ws: WebSocketStream<ConnectStream>,
    /// The identifier for a specific command
    next_id: usize,
    /// Outbound message state machine.
    flush_state: FlushState,
    _marker: PhantomData<T>,
}

impl<T: EventMessage + Unpin> Connection<T> {
    pub async fn connect(debug_ws_url: impl AsRef<str>) -> Result<Self> {
        let config = WebSocketConfig::default()
            .max_message_size(None)
            .max_frame_size(None);

        cfg_if::cfg_if! {
            if #[cfg(feature = "async-std-runtime")] {
               let (ws, _) = async_tungstenite::async_std::connect_async_with_config(debug_ws_url.as_ref(), Some(config)).await?;
            } else if #[cfg(feature = "tokio-runtime")] {
                 let (ws, _) = async_tungstenite::tokio::connect_async_with_config(debug_ws_url.as_ref(), Some(config)).await?;
            }
        }

        Ok(Self {
            pending_commands: Default::default(),
            ws,
            next_id: 0,
            flush_state: FlushState::Idle,
            _marker: Default::default(),
        })
    }
}

impl<T: EventMessage> Connection<T> {
    fn next_call_id(&mut self) -> CallId {
        let id = CallId::new(self.next_id);
        self.next_id = self.next_id.wrapping_add(1);
        id
    }

    /// Queue in the command to send over the socket and return the id for this
    /// command
    pub fn submit_command(
        &mut self,
        method: &MethodId,
        session_id: Option<SessionId>,
        params: serde_json::Value,
    ) -> serde_json::Result<CallId> {
        let id = self.next_call_id();
        let call = MethodCall {
            id,
            method: method.clone(),
            session_id: session_id.map(Into::into),
            params,
        };
        self.pending_commands.push_back(call);
        Ok(id)
    }

    /// Flush any processed message and start sending the next over the conn
    /// sink.
    fn start_send_next(&mut self, cx: &mut Context<'_>) -> Result<()> {
        if matches!(self.flush_state, FlushState::NeedsFlush) {
            if let Poll::Ready(Ok(())) = self.ws.poll_flush_unpin(cx) {
                self.flush_state = FlushState::Idle;
            }
        }
        if matches!(self.flush_state, FlushState::Idle) {
            if let Some(cmd) = self.pending_commands.pop_front() {
                tracing::trace!("Sending {:?}", cmd);
                let msg = serde_json::to_string(&cmd)?;
                self.ws.start_send_unpin(msg.into())?;
                self.flush_state = FlushState::Pending(cmd);
            }
        }
        Ok(())
    }
}

impl<T: EventMessage + Unpin> Stream for Connection<T> {
    type Item = Result<Message<T>>;

    /// Drives the send/flush state machine, then reads inbound messages.
    ///
    /// Flow: queue next command → wait for sink readiness → flush → read WS.
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let pin = self.get_mut();

        // ── Outbound: send queued commands ──
        loop {
            if let Err(err) = pin.start_send_next(cx) {
                return Poll::Ready(Some(Err(err)));
            }

            if let FlushState::Pending(call) = std::mem::replace(&mut pin.flush_state, FlushState::Idle) {
                if pin.ws.poll_ready_unpin(cx).is_ready() {
                    pin.flush_state = FlushState::NeedsFlush;
                    continue;
                } else {
                    pin.flush_state = FlushState::Pending(call);
                }
            }

            break;
        }

        // ── Inbound: read from the WebSocket ──
        match ready!(pin.ws.poll_next_unpin(cx)) {
            Some(Ok(WsMessage::Text(text))) => {
                let ready = match serde_json::from_str::<Message<T>>(&text) {
                    Ok(msg) => {
                        tracing::trace!("Received {:?}", msg);
                        Ok(msg)
                    }
                    Err(err) => {
                        let msg = text.as_str().to_string();
                        tracing::debug!(target: "onecrawl_browser::conn::raw_ws::parse_errors", msg, "Failed to parse raw WS message {}", err);
                        Err(CdpError::InvalidMessage(text.as_str().to_string(), err))
                    }
                };
                Poll::Ready(Some(ready))
            }
            Some(Ok(WsMessage::Close(_))) => Poll::Ready(None),
            // ignore ping and pong
            Some(Ok(WsMessage::Ping(_))) | Some(Ok(WsMessage::Pong(_))) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Some(Ok(msg)) => Poll::Ready(Some(Err(CdpError::UnexpectedWsMessage(msg)))),
            Some(Err(err)) => Poll::Ready(Some(Err(CdpError::Ws(err)))),
            None => {
                // ws connection closed
                Poll::Ready(None)
            }
        }
    }
}
