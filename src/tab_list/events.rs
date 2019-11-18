use super::TabListEntry;
use crate::EventType;

#[derive(Debug)]
pub enum TabListEvent {
  Added(TabListEntry),

  // TODO maybe have old version?
  Changed(TabListEntry),

  Removed(u8),

  Disconnected,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum TabListEventType {
  /// new TabList entry
  Added,
  /// TabList entry changed
  Changed,
  /// TabList entry removed
  Removed,

  /// self disconnected, so TabList is cleared
  Disconnected,
}

impl EventType for TabListEvent {
  type EventType = TabListEventType;

  fn event_type(&self) -> Self::EventType {
    match self {
      TabListEvent::Added(_) => TabListEventType::Added,
      TabListEvent::Changed(_) => TabListEventType::Changed,
      TabListEvent::Removed(_) => TabListEventType::Removed,
      TabListEvent::Disconnected => TabListEventType::Disconnected,
    }
  }
}
