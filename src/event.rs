use std::time::Duration;

use color_eyre::eyre::Result;
use crossterm::event::{self, Event};

/// Polls for the next input event, timing out after `timeout`. The timeout
/// doubles as the render loop's animation tick (e.g. for spinners).
pub fn next_event(timeout: Duration) -> Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}
