//! Chrome process lifecycle — wait, kill, try_wait, and Drop.

use std::io;

use crate::async_process::{Child, ExitStatus};

use super::Browser;

impl Browser {
    /// Asynchronously wait for the spawned chromium instance to exit completely.
    ///
    /// The instance is spawned by [`Browser::launch`]. `wait` is usually called after
    /// [`Browser::close`]. You can call this explicitly to collect the process and avoid
    /// "zombie" processes.
    ///
    /// This call has no effect if this [`Browser`] did not spawn any chromium instance (e.g.
    /// connected to an existing browser through [`Browser::connect`])
    pub async fn wait(&mut self) -> io::Result<Option<ExitStatus>> {
        if let Some(child) = self.child.as_mut() {
            Ok(Some(child.wait().await?))
        } else {
            Ok(None)
        }
    }

    /// If the spawned chromium instance has completely exited, wait for it.
    ///
    /// The instance is spawned by [`Browser::launch`]. `try_wait` is usually called after
    /// [`Browser::close`]. You can call this explicitly to collect the process and avoid
    /// "zombie" processes.
    ///
    /// This call has no effect if this [`Browser`] did not spawn any chromium instance (e.g.
    /// connected to an existing browser through [`Browser::connect`])
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        if let Some(child) = self.child.as_mut() {
            child.try_wait()
        } else {
            Ok(None)
        }
    }

    /// Get the spawned chromium instance
    ///
    /// The instance is spawned by [`Browser::launch`]. The result is a [`async_process::Child`]
    /// value. It acts as a compat wrapper for an `async-std` or `tokio` child process.
    ///
    /// You may use [`async_process::Child::as_mut_inner`] to retrieve the concrete implementation
    /// for the selected runtime.
    ///
    /// This call has no effect if this [`Browser`] did not spawn any chromium instance (e.g.
    /// connected to an existing browser through [`Browser::connect`])
    pub fn get_mut_child(&mut self) -> Option<&mut Child> {
        self.child.as_mut()
    }

    /// Forcibly kill the spawned chromium instance
    ///
    /// The instance is spawned by [`Browser::launch`]. `kill` will automatically wait for the child
    /// process to exit to avoid "zombie" processes.
    ///
    /// This method is provided to help if the browser does not close by itself. You should prefer
    /// to use [`Browser::close`].
    ///
    /// This call has no effect if this [`Browser`] did not spawn any chromium instance (e.g.
    /// connected to an existing browser through [`Browser::connect`])
    pub async fn kill(&mut self) -> Option<io::Result<()>> {
        match self.child.as_mut() {
            Some(child) => Some(child.kill().await),
            None => None,
        }
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            if let Ok(Some(_)) = child.try_wait() {
                // Already exited, do nothing. Usually occurs after using the method close or kill.
            } else {
                // We set the `kill_on_drop` property for the child process, so no need to explicitely
                // kill it here. It can't really be done anyway since the method is async.
                //
                // On Unix, the process will be reaped in the background by the runtime automatically
                // so it won't leave any resources locked. It is, however, a better practice for the user to
                // do it himself since the runtime doesn't provide garantees as to when the reap occurs, so we
                // warn him here.
                tracing::warn!("Browser was not closed manually, it will be killed automatically in the background");
            }
        }
    }
}
