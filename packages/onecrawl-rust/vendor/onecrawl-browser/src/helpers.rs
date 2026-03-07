//! DRY helpers for common CDP command patterns.

use onecrawl_browser_types::Command;

use crate::error::Result;
use crate::page::Page;

impl Page {
    /// Execute a CDP command and discard the response.
    /// Reduces boilerplate for fire-and-forget commands.
    pub(crate) async fn execute_void<T>(&self, cmd: T) -> Result<()>
    where
        T: Command,
        T::Response: std::fmt::Debug,
    {
        self.execute(cmd).await?;
        Ok(())
    }
}
