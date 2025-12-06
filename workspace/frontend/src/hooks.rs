/// API fetch state enum
#[derive(Clone, PartialEq)]
pub enum FetchState<T> {
    NotStarted,
    Loading,
    Success(T),
    Error(String),
}

impl<T> Default for FetchState<T> {
    fn default() -> Self {
        Self::NotStarted
    }
}

impl<T> FetchState<T> {
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    pub fn data(&self) -> Option<&T> {
        match self {
            Self::Success(data) => Some(data),
            _ => None,
        }
    }

    pub fn error(&self) -> Option<&String> {
        match self {
            Self::Error(err) => Some(err),
            _ => None,
        }
    }
}

// Helper pattern for loading component data
// Due to Yew hook limitations, use this inline in components:
//
// Example:
// ```
// let fetch_state = use_state(|| FetchState::Loading);
// let toast_ctx = use_context::<ToastContext>().unwrap();
//
// let refetch = use_callback(...); // see below
//
// use_effect_with((), |_| { refetch.emit(()); || () });
// ```
