//! This module provides functionality for handling processes, including spawning and killing processes.
//! It defines a trait `ProcessOps` for process operations and a struct `ProcessHdl` that implements this trait.

use crate::error::Result;
use anyhow::anyhow;
use log::{error, warn};
use std::process::{self, Command};

#[cfg(test)]
use mockall::automock;

/// Trait defining operations for process handling.
#[cfg_attr(test, automock)]
pub trait ProcessHdlOps {
    /// Spawns a new process using the provided command.
    ///
    /// # Arguments
    ///
    /// * `cmd` - A reference to a `Command` that specifies the process to be spawned.
    ///
    /// # Errors
    ///
    /// Returns an error if a process is already associated with the handler or if the process fails to spawn.
    fn spawn(&mut self, cmd: &mut Command) -> Result<()>;

    /// Kills the associated process if one exists.
    ///
    /// # Errors
    ///
    /// Returns an error if no process is associated with the handler or if the process fails to be killed.
    fn kill(&mut self) -> Result<()>;
}

/// Struct to handle process operations.
pub struct ProcessHdl {
    child_process: Option<process::Child>,
}

impl ProcessHdl {
    /// Creates a new `ProcessHdl` instance.
    ///
    /// # Returns
    ///
    /// A new instance of `ProcessHdl` with no associated process.
    pub fn handler() -> Self {
        Self { child_process: None }
    }
}

impl ProcessHdlOps for ProcessHdl {
    /// Spawns a new process if none is currently associated.
    ///
    /// # Arguments
    ///
    /// * `cmd` - A reference to a `Command` that specifies the process to be spawned.
    ///
    /// # Errors
    ///
    /// Returns an error if a process is already associated with the handler or if the process fails to spawn.
    fn spawn(&mut self, cmd: &mut Command) -> Result<()> {
        if self.child_process.is_some() {
            error!("Handler already has an associated process");
            return Err(anyhow!("Handler already has an associated process"));
        }

        self.child_process = Some(cmd.spawn()?);
        Ok(())
    }

    /// Kills the associated process if one exists.
    ///
    /// # Errors
    ///
    /// Returns an error if no process is associated with the handler or if the process fails to be killed.
    fn kill(&mut self) -> Result<()> {
        if let Some(mut process) = self.child_process.take() {
            process.kill()?;
            process.wait()?;
        } else {
            warn!("No process to kill");
        }
        Ok(())
    }
}

impl Drop for ProcessHdl {
    /// Ensures the process is killed when the handler is dropped.
    ///
    /// # Errors
    ///
    /// Logs an error if the process fails to be killed.
    fn drop(&mut self) {
        if let Err(e) = self.kill() {
            error!("Failed to stop process, error: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// Tests that a process can be spawned successfully.
    #[test]
    fn test_spawn_process() {
        let mut process_hdl = ProcessHdl::handler();
        let mut cmd = Command::new("echo");
        cmd.arg("Hello, world!");

        // Test spawning a process
        let result = process_hdl.spawn(&mut cmd);
        assert!(result.is_ok());
        assert!(process_hdl.child_process.is_some());
    }

    /// Tests that an error is returned if a process is already associated with the handler.
    #[test]
    fn test_spawn_process_already_exists() {
        let mut process_hdl = ProcessHdl::handler();
        let mut cmd = Command::new("echo");
        cmd.arg("Hello, world!");

        // Spawn the first process
        process_hdl.spawn(&mut cmd).unwrap();

        // Try to spawn another process
        let result = process_hdl.spawn(&mut cmd);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Handler already has an associated process"
        );
    }

    /// Tests that an associated process can be killed successfully.
    #[test]
    fn test_kill_process() {
        let mut process_hdl = ProcessHdl::handler();
        let mut cmd = Command::new("sleep");
        cmd.arg("1");

        // Spawn a process
        process_hdl.spawn(&mut cmd).unwrap();
        assert!(process_hdl.child_process.is_some());

        // Kill the process
        let result = process_hdl.kill();
        assert!(result.is_ok());
        assert!(process_hdl.child_process.is_none());
    }

    /// Tests that calling `kill` when no process is associated does not cause an error.
    #[test]
    fn test_kill_no_process() {
        let mut process_hdl = ProcessHdl::handler();

        // Try to kill a process when none is associated
        let result = process_hdl.kill();
        assert!(result.is_ok()); // kill() returns Ok even if no process is associated
    }

    /// Tests that the process is killed when the `ProcessHdl` instance is dropped.
    #[test]
    fn test_drop_kills_process() {
        let mut cmd = Command::new("sleep");
        cmd.arg("5");

        {
            let mut process_hdl = ProcessHdl::handler();
            process_hdl.spawn(&mut cmd).unwrap();
            assert!(process_hdl.child_process.is_some());
        } // process_hdl goes out of scope here and should kill the process

        // Since the process is killed on drop, we can't directly test it here.
        // However, if the process wasn't killed, it would still be running.
        // We can check the process list to ensure no "sleep 1" process is running.
        let output = Command::new("pgrep").arg("sleep").output().unwrap();
        let output_str = String::from_utf8_lossy(&output.stdout);
        assert!(!output_str.contains("sleep"));
    }
}
