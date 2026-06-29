/// An event dispatched to listeners
#[derive(Debug)]
pub struct Event {
    pub type_: String,
    pub data: Option<Box<dyn std::any::Any + Send + Sync>>,
}

impl Event {
    /// Create a new event with type and optional data
    pub fn new(type_: &str, data: Option<Box<dyn std::any::Any + Send + Sync>>) -> Self {
        Self {
            type_: type_.to_string(),
            data,
        }
    }

    /// Create an event with typed data
    pub fn with_data<T: 'static + Send + Sync>(type_: &str, data: T) -> Self {
        Self {
            type_: type_.to_string(),
            data: Some(Box::new(data)),
        }
    }

    /// Try to extract typed data from the event
    pub fn data<T: 'static + Send + Sync>(&self) -> Option<&T> {
        self.data.as_ref().and_then(|d| d.downcast_ref::<T>())
    }
}

impl Clone for Event {
    fn clone(&self) -> Self {
        Self {
            type_: self.type_.clone(),
            // Box<dyn Any> doesn't implement Clone, so we can only clone if data is None
            // This is a known limitation - for shared data, users should use Arc
            data: None,
        }
    }
}
