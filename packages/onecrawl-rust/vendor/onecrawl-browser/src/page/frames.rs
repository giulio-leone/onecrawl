//! Frame query methods for Page.

use futures::channel::oneshot::channel as oneshot_channel;
use futures::SinkExt;

use onecrawl_protocol::cdp::browser_protocol::page::FrameId;

use crate::error::Result;
use crate::handler::target::{GetName, GetParent, GetUrl, TargetMessage};

use super::Page;

impl Page {
    /// Returns the name of the frame
    pub async fn frame_name(&self, frame_id: FrameId) -> Result<Option<String>> {
        let (tx, rx) = oneshot_channel();
        self.inner
            .sender()
            .clone()
            .send(TargetMessage::Name(GetName {
                frame_id: Some(frame_id),
                tx,
            }))
            .await?;
        Ok(rx.await?)
    }

    /// Returns the current url of the frame
    pub async fn frame_url(&self, frame_id: FrameId) -> Result<Option<String>> {
        let (tx, rx) = oneshot_channel();
        self.inner
            .sender()
            .clone()
            .send(TargetMessage::Url(GetUrl {
                frame_id: Some(frame_id),
                tx,
            }))
            .await?;
        Ok(rx.await?)
    }

    /// Returns the parent id of the frame
    pub async fn frame_parent(&self, frame_id: FrameId) -> Result<Option<FrameId>> {
        let (tx, rx) = oneshot_channel();
        self.inner
            .sender()
            .clone()
            .send(TargetMessage::Parent(GetParent { frame_id, tx }))
            .await?;
        Ok(rx.await?)
    }

    /// Return the main frame of the page
    pub async fn mainframe(&self) -> Result<Option<FrameId>> {
        let (tx, rx) = oneshot_channel();
        self.inner
            .sender()
            .clone()
            .send(TargetMessage::MainFrame(tx))
            .await?;
        Ok(rx.await?)
    }

    /// Return the frames of the page
    pub async fn frames(&self) -> Result<Vec<FrameId>> {
        let (tx, rx) = oneshot_channel();
        self.inner
            .sender()
            .clone()
            .send(TargetMessage::AllFrames(tx))
            .await?;
        Ok(rx.await?)
    }
}
