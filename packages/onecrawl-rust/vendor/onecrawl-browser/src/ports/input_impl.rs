use async_trait::async_trait;
use onecrawl_protocol::cdp::browser_protocol::input::{
    DispatchKeyEventParams, DispatchKeyEventType, DispatchMouseEventParams,
    DispatchMouseEventType, MouseButton,
};

use crate::error::Result;
use crate::layout::Point;
use crate::page::Page;
use super::InputPort;

#[async_trait]
impl InputPort for Page {
    async fn click_at(&self, x: f64, y: f64) -> Result<()> {
        self.click(Point::new(x, y)).await?;
        Ok(())
    }

    async fn move_mouse_to(&self, x: f64, y: f64) -> Result<()> {
        self.move_mouse(Point::new(x, y)).await?;
        Ok(())
    }

    async fn mouse_down(&self, x: f64, y: f64) -> Result<()> {
        let mut params =
            DispatchMouseEventParams::new(DispatchMouseEventType::MousePressed, x, y);
        params.button = Some(MouseButton::Left);
        params.click_count = Some(1);
        self.execute(params).await?;
        Ok(())
    }

    async fn mouse_up(&self, x: f64, y: f64) -> Result<()> {
        let mut params =
            DispatchMouseEventParams::new(DispatchMouseEventType::MouseReleased, x, y);
        params.button = Some(MouseButton::Left);
        params.click_count = Some(1);
        self.execute(params).await?;
        Ok(())
    }

    async fn type_keyboard(&self, text: &str) -> Result<()> {
        for ch in text.chars() {
            let ch_str = ch.to_string();

            let mut down = DispatchKeyEventParams::new(DispatchKeyEventType::KeyDown);
            down.text = Some(ch_str.clone());
            down.key = Some(ch_str.clone());
            self.execute(down).await?;

            let mut up = DispatchKeyEventParams::new(DispatchKeyEventType::KeyUp);
            up.key = Some(ch_str);
            self.execute(up).await?;
        }
        Ok(())
    }

    async fn press_keyboard_key(&self, key: &str) -> Result<()> {
        let mut down = DispatchKeyEventParams::new(DispatchKeyEventType::KeyDown);
        down.key = Some(key.to_string());
        self.execute(down).await?;

        let mut up = DispatchKeyEventParams::new(DispatchKeyEventType::KeyUp);
        up.key = Some(key.to_string());
        self.execute(up).await?;

        Ok(())
    }
}
