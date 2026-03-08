use futures::channel::{
    mpsc,
    oneshot::{self, channel as oneshot_channel},
};
use pin_project_lite::pin_project;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::cmd::{to_command_response, CommandMessage};
use crate::error::{CdpError, Result};
use crate::handler::target::TargetMessage;
use onecrawl_protocol::cdp::browser_protocol::target::SessionId;
use onecrawl_browser_types::{Command, CommandResponse, MethodId, Response};

pin_project! {
    pub struct CommandFuture<T, M = Result<Response>> {
        #[pin]
        rx_command: oneshot::Receiver<M>,
        #[pin]
        delay: futures_timer::Delay,

        method: MethodId,

        _marker: PhantomData<T>
    }
}

impl<T: Command> CommandFuture<T> {
    pub fn new(
        cmd: T,
        sender: &mpsc::UnboundedSender<TargetMessage>,
        session: Option<SessionId>,
    ) -> Result<Self> {
        let (tx, rx_command) = oneshot_channel::<Result<Response>>();
        let method = cmd.identifier();

        let message = TargetMessage::Command(CommandMessage::with_session(
            cmd, tx, session,
        )?);

        // Send eagerly — unbounded channel never blocks
        sender
            .unbounded_send(message)
            .map_err(|e| CdpError::from(e.into_send_error()))?;

        let delay = futures_timer::Delay::new(std::time::Duration::from_millis(
            crate::handler::REQUEST_TIMEOUT,
        ));

        Ok(Self {
            rx_command,
            delay,
            method,
            _marker: PhantomData,
        })
    }
}

impl<T> Future for CommandFuture<T>
where
    T: Command,
{
    type Output = Result<CommandResponse<T::Response>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        if this.delay.poll(cx).is_ready() {
            Poll::Ready(Err(CdpError::Timeout))
        } else {
            match this.rx_command.as_mut().poll(cx) {
                Poll::Ready(Ok(Ok(response))) => {
                    // Move the method out — this future won't be polled again after Ready.
                    Poll::Ready(to_command_response::<T>(response, std::mem::take(this.method)))
                }
                Poll::Ready(Ok(Err(e))) => Poll::Ready(Err(e)),
                Poll::Ready(Err(e)) => Poll::Ready(Err(e.into())),
                Poll::Pending => Poll::Pending,
            }
        }
    }
}
