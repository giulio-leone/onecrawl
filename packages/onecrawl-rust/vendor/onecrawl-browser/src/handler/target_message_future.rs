use futures::channel::{
    mpsc,
    oneshot::{self, channel as oneshot_channel},
};
use pin_project_lite::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error::{CdpError, Result};
use crate::handler::target::TargetMessage;
use crate::ArcHttpRequest;

pin_project! {
    pub struct TargetMessageFuture<T> {
        #[pin]
        rx_request: oneshot::Receiver<T>,
    }
}

impl<T> TargetMessageFuture<T> {
    pub fn new(
        sender: &mpsc::UnboundedSender<TargetMessage>,
        message: TargetMessage,
        rx_request: oneshot::Receiver<T>,
    ) -> Result<Self> {
        // Send eagerly — unbounded channel never blocks
        sender
            .unbounded_send(message)
            .map_err(|e| CdpError::from(e.into_send_error()))?;
        Ok(Self { rx_request })
    }

    pub fn wait_for_navigation(sender: &mpsc::UnboundedSender<TargetMessage>) -> Result<TargetMessageFuture<ArcHttpRequest>> {
        let (tx, rx_request) = oneshot_channel();

        let message = TargetMessage::WaitForNavigation(tx);

        TargetMessageFuture::new(sender, message, rx_request)
    }
}

impl<T> Future for TargetMessageFuture<T> {
    type Output = Result<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        this.rx_request.as_mut().poll(cx).map_err(Into::into)
    }
}
