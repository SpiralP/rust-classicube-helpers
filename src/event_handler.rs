use std::{collections::HashMap, hash::Hash};

pub trait EventType {
  type EventType: Hash + Eq;
  fn event_type(&self) -> Self::EventType;
}

type Callback<E> = Box<dyn FnMut(&E) + 'static>;

#[derive(Default)]
pub struct EventHandler<E>
where
  E: EventType,
{
  callbacks: HashMap<E::EventType, Vec<Callback<E>>>,
}

impl<E> EventHandler<E>
where
  E: EventType,
{
  pub fn new() -> Self {
    Self {
      callbacks: HashMap::new(),
    }
  }

  pub fn on<F>(&mut self, event_type: E::EventType, callback: F)
  where
    F: FnMut(&E),
    F: 'static,
  {
    self
      .callbacks
      .entry(event_type)
      .or_insert_with(Vec::new)
      .push(Box::new(callback));
  }

  pub fn handle_event(&mut self, event: E) {
    if let Some(callbacks) = self.callbacks.get_mut(&event.event_type()) {
      for callback in callbacks {
        callback(&event);
      }
    }
  }
}

#[test]
fn test_listener() {
  #[derive(Debug)]
  enum CatEvent {
    Meow(&'static str),
    Purr(&'static str),
  }

  #[derive(PartialEq, Eq, Hash)]
  enum CatEventType {
    Meow,
    Purr,
  }

  impl EventType for CatEvent {
    type EventType = CatEventType;

    fn event_type(&self) -> CatEventType {
      match self {
        CatEvent::Meow(_) => CatEventType::Meow,
        CatEvent::Purr(_) => CatEventType::Purr,
      }
    }
  }

  let mut listener = EventHandler::new();
  listener.on(CatEventType::Meow, |e| {
    if let CatEvent::Meow(text) = e {
      println!("{:?}", text);
    }
  });

  listener.handle_event(CatEvent::Purr("purring"));
  listener.handle_event(CatEvent::Meow("meowing"));
}
