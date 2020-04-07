type Callback<E> = Box<dyn FnMut(&E) + 'static>;

#[derive(Default)]
pub struct CallbackHandler<E> {
    callbacks: Vec<Callback<E>>,
    simulating: bool,
}

impl<E> CallbackHandler<E> {
    pub fn new() -> Self {
        Self {
            callbacks: Vec::new(),
            simulating: false,
        }
    }

    pub fn on<F>(&mut self, callback: F)
    where
        F: FnMut(&E),
        F: 'static,
    {
        self.callbacks.push(Box::new(callback));
    }

    pub fn handle_event(&mut self, event: E) {
        if self.simulating {
            return;
        }
        self.simulating = true;
        for callback in &mut self.callbacks {
            callback(&event);
        }
        self.simulating = false;
    }
}
