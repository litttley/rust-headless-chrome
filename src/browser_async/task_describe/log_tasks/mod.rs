pub mod log_events;
pub mod log_enable;

use super::super::{DebugSession};
use super::super::page_message::{PageResponseWrapper};
use super::super::protocol::{target};
pub use log_enable::{LogEnableTask, LogEnableTaskBuilder};
use log::*;


#[derive(Debug)]
pub enum LogEvent {
    EntryAdded(log_events::LogEntryAdded),
}

pub fn handle_log_event(
        debug_session: &mut DebugSession,
        log_event: LogEvent,
        _maybe_session_id: Option<target::SessionID>,
        maybe_target_id: Option<target::TargetId>,
    ) -> Result<PageResponseWrapper, failure::Error> {
        match log_event {
            LogEvent::EntryAdded(_event) => {
                trace!("EntryAdded event.");
                Ok(PageResponseWrapper::default())
            }
        }
    }