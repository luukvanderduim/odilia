use atspi::accessible::AccessibleProxy;
use atspi::events::Event;
use crate::state::ScreenReaderState;

/*
pub async fn get_event_accessible<'a>(state: &ScreenReaderState, event: Event) -> eyre::Result<zbus::Result<AccessibleProxy<'a>>> {
    let path = if let Some(path) = event.path() { path } else {return Err("Could not get path."); };
    let sender = if let Some(sender) = event.sender()? { sender } else { return Err("Could not get sender."); };
    Ok(state.accessible(sender, path))
}
*/

pub async fn dispatch(state: &ScreenReaderState, event: Event) -> eyre::Result<()> {
    // Dispatch based on member
    if let Some(member) = event.member() {
    match member.as_str() {
        "StateChanged" => state_changed::dispatch(state, event).await?,
        "TextCaretMoved" => text_caret_moved::dispatch(state, event).await?,
            member => tracing::debug!(member, "Ignoring event with unknown member"),
    }
    }
Ok(())
}

mod text_caret_moved {
use atspi::events::Event;
use crate::state::ScreenReaderState;
use std::cmp::{
  min,
  max
};

pub async fn tcm(state: &ScreenReaderState, event: Event) -> eyre::Result<()> {
  let mut last_caret_pos = state.last_caret_pos.lock().await;
  let current_caret_pos = event.detail1();

  let start = min(*last_caret_pos, current_caret_pos);
  let end = max(*last_caret_pos, current_caret_pos);

  let path = if let Some(path) = event.path() { path } else {return Ok(()); };
  let sender = if let Some(sender) = event.sender()? { sender } else { return Ok(()); };
  let accessible = state.text(sender, path).await?;
  let name = accessible.get_text(start, end).await?;
  // update caret position
  *last_caret_pos = event.detail1();
  std::mem::drop(last_caret_pos);
  if name.len() > 0 {
    state.speaker.say(speech_dispatcher::Priority::Text, format!("{name}"));
  }
  Ok(())
}

pub async fn dispatch(state: &ScreenReaderState, event: Event) -> eyre::Result<()> {
  // Dispatch based on kind
  match event.kind() {
    "" => tcm(state, event).await?,
    kind => tracing::debug!(kind, "Ignoring event with unknown kind"),
  }
  Ok(())
}

} // end of text_caret_moved

mod state_changed {
use atspi::events::Event;
use crate::state::ScreenReaderState;

pub async fn dispatch(state: &ScreenReaderState, event: Event) -> eyre::Result<()> {
    // Dispatch based on kind
    match event.kind() {
        "focused" => focused(state, event).await?,
            kind => tracing::debug!(kind, "Ignoring event with unknown kind"),
    }
    Ok(())
}

pub async fn focused(state: &ScreenReaderState, event: Event) -> zbus::Result<()> {
    // Speak the newly focused object
    let path = if let Some(path) = event.path() { path } else {return Ok(()); };
    let sender = if let Some(sender) = event.sender()? { sender } else { return Ok(()); };
    let accessible = state.accessible(sender, path).await?;

    let name_fut = accessible.name();
    let description_fut = accessible.description();
    let role_fut = accessible.get_localized_role_name();
    let (name, description, role) = tokio::try_join!(name_fut, description_fut, role_fut)?;

    state.speaker.say(speech_dispatcher::Priority::Text, format!("{name}, {role}. {description}"));
    Ok(())
}
}
