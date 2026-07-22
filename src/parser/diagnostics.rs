use std::cell::RefCell;

thread_local! {
    static SINK: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

pub fn warn(message: impl Into<String>) {
    let uncaptured = SINK.with(|sink| {
        let message = message.into();
        match sink.borrow_mut().as_mut() {
            Some(messages) => {
                messages.push(message);
                None
            }
            None => Some(message),
        }
    });

    if let Some(message) = uncaptured {
        eprintln!("Warning: {message}");
    }
}

pub fn capture<T>(run: impl FnOnce() -> T) -> (T, Vec<String>) {
    let previous = SINK.with(|sink| sink.replace(Some(Vec::new())));
    debug_assert!(previous.is_none());

    let value = run();
    let messages = SINK.with(|sink| sink.replace(previous)).unwrap_or_default();
    (value, messages)
}

pub fn discard<T>(run: impl FnOnce() -> T) -> T {
    let previous = SINK.with(|sink| sink.replace(Some(Vec::new())));
    let value = run();
    SINK.with(|sink| {
        sink.replace(previous);
    });
    value
}
