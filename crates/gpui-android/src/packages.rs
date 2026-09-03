//! Optional platform packages: deep links and media-session controls.
//!
//! The Java host (`GpuiActivity.onNewIntent`, `GpuiMediaSession`) forwards
//! inbound intents / transport controls to Rust through the JNI bridges in
//! `android::jni`. This module is the framework-side dispatch table for those
//! events: apps register handlers, and the `notify_*` entry points deliver to
//! them.
//!
//! When no handler is registered the notifies are observable no-ops (a
//! `debug!` log) rather than silent drops, so a missing registration is easy
//! to spot in logcat without crashing a handler-less host build.

pub mod deeplink {
    //! Inbound deep-link dispatch (`gpui://…` intents).
    //!
    //! Register with [`set_deep_link_handler`]; the JNI bridge calls
    //! [`notify_deep_link`] for every non-empty URL the activity receives.

    use std::sync::Mutex;

    type DeepLinkHandler = Box<dyn Fn(String) + Send + Sync + 'static>;

    static HANDLER: Mutex<Option<DeepLinkHandler>> = Mutex::new(None);

    /// Register the deep-link handler, replacing any previous one.
    pub fn set_deep_link_handler(handler: DeepLinkHandler) {
        *HANDLER.lock().unwrap() = Some(handler);
    }

    /// Remove the deep-link handler, if any.
    pub fn clear_deep_link_handler() {
        *HANDLER.lock().unwrap() = None;
    }

    /// Whether a deep-link handler is currently registered.
    pub fn has_deep_link_handler() -> bool {
        HANDLER.lock().unwrap().is_some()
    }

    /// Deliver an inbound deep-link URL to the registered handler.
    ///
    /// Empty URLs are ignored (the JNI bridge already filters them, this is a
    /// second layer of defence). With no handler registered the URL is logged
    /// at debug level and dropped.
    pub fn notify_deep_link(url: &str) {
        if url.is_empty() {
            return;
        }
        if let Some(handler) = HANDLER.lock().unwrap().as_ref() {
            handler(url.to_string());
        } else {
            log::debug!("deeplink: no handler registered, dropping {url:?}");
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::sync::{Arc, Mutex as StdMutex};

        #[test]
        fn handler_receives_non_empty_urls_and_empty_is_ignored() {
            let seen: Arc<StdMutex<Vec<String>>> = Arc::new(StdMutex::new(Vec::new()));
            let seen_clone = Arc::clone(&seen);
            set_deep_link_handler(Box::new(move |url| {
                seen_clone.lock().unwrap().push(url);
            }));
            assert!(has_deep_link_handler());

            notify_deep_link("gpui://video_player?watch=abc");
            notify_deep_link("");
            assert_eq!(
                *seen.lock().unwrap(),
                vec!["gpui://video_player?watch=abc".to_string()]
            );

            clear_deep_link_handler();
            assert!(!has_deep_link_handler());
            // No handler: must not panic, must not deliver.
            notify_deep_link("gpui://dropped");
            assert_eq!(seen.lock().unwrap().len(), 1);
        }

        #[test]
        fn registering_replaces_the_previous_handler() {
            set_deep_link_handler(Box::new(|_| {}));
            let seen: Arc<StdMutex<Vec<String>>> = Arc::new(StdMutex::new(Vec::new()));
            let seen_clone = Arc::clone(&seen);
            set_deep_link_handler(Box::new(move |url| {
                seen_clone.lock().unwrap().push(url);
            }));
            notify_deep_link("gpui://second");
            assert_eq!(*seen.lock().unwrap(), vec!["gpui://second".to_string()]);
            clear_deep_link_handler();
        }
    }
}

pub mod media_session {
    //! Media-session transport controls (play/pause/stop/next/previous/seek).
    //!
    //! Register with [`set_media_handler`]; the JNI bridges call
    //! [`notify_action`] / [`notify_seek`]. Unknown action strings arriving
    //! from Java map to `None` via [`parse_media_action`] and are logged and
    //! dropped instead of reaching the handler.

    use std::sync::Mutex;

    /// Transport-control action from the OS media session.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum MediaAction {
        Play,
        Pause,
        Stop,
        Next,
        Previous,
    }

    /// A media-session event delivered to the app handler.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum MediaEvent {
        Action(MediaAction),
        Seek(u64),
    }

    type MediaHandler = Box<dyn Fn(MediaEvent) + Send + Sync + 'static>;

    static HANDLER: Mutex<Option<MediaHandler>> = Mutex::new(None);

    /// Parse a media-action string from the Java `GpuiMediaSession` bridge.
    ///
    /// Returns `None` for unknown actions so the bridge can warn and drop
    /// them instead of misdispatching.
    pub fn parse_media_action(action: &str) -> Option<MediaAction> {
        match action {
            "play" => Some(MediaAction::Play),
            "pause" => Some(MediaAction::Pause),
            "stop" => Some(MediaAction::Stop),
            "next" => Some(MediaAction::Next),
            "previous" => Some(MediaAction::Previous),
            _ => None,
        }
    }

    /// Register the media-event handler, replacing any previous one.
    pub fn set_media_handler(handler: MediaHandler) {
        *HANDLER.lock().unwrap() = Some(handler);
    }

    /// Remove the media-event handler, if any.
    pub fn clear_media_handler() {
        *HANDLER.lock().unwrap() = None;
    }

    /// Whether a media-event handler is currently registered.
    pub fn has_media_handler() -> bool {
        HANDLER.lock().unwrap().is_some()
    }

    fn dispatch(event: MediaEvent) {
        if let Some(handler) = HANDLER.lock().unwrap().as_ref() {
            handler(event);
        } else {
            log::debug!("media_session: no handler registered, dropping {event:?}");
        }
    }

    /// Deliver a transport-control action to the registered handler.
    pub fn notify_action(action: MediaAction) {
        dispatch(MediaEvent::Action(action));
    }

    /// Deliver a seek request (milliseconds) to the registered handler.
    pub fn notify_seek(position_ms: u64) {
        dispatch(MediaEvent::Seek(position_ms));
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::sync::{Arc, Mutex as StdMutex};

        #[test]
        fn action_strings_parse_and_unknown_is_none() {
            assert_eq!(parse_media_action("play"), Some(MediaAction::Play));
            assert_eq!(parse_media_action("pause"), Some(MediaAction::Pause));
            assert_eq!(parse_media_action("stop"), Some(MediaAction::Stop));
            assert_eq!(parse_media_action("next"), Some(MediaAction::Next));
            assert_eq!(parse_media_action("previous"), Some(MediaAction::Previous));
            assert_eq!(parse_media_action("rewind"), None);
            assert_eq!(parse_media_action(""), None);
            assert_eq!(parse_media_action("PLAY"), None);
        }

        #[test]
        fn handler_receives_actions_and_seeks() {
            let seen: Arc<StdMutex<Vec<MediaEvent>>> = Arc::new(StdMutex::new(Vec::new()));
            let seen_clone = Arc::clone(&seen);
            set_media_handler(Box::new(move |event| {
                seen_clone.lock().unwrap().push(event);
            }));
            assert!(has_media_handler());

            notify_action(MediaAction::Play);
            notify_action(MediaAction::Next);
            notify_seek(90_000);
            notify_seek(0);
            assert_eq!(
                *seen.lock().unwrap(),
                vec![
                    MediaEvent::Action(MediaAction::Play),
                    MediaEvent::Action(MediaAction::Next),
                    MediaEvent::Seek(90_000),
                    MediaEvent::Seek(0),
                ]
            );

            clear_media_handler();
            assert!(!has_media_handler());
            // No handler: must not panic, must not deliver.
            notify_action(MediaAction::Stop);
            notify_seek(1);
            assert_eq!(seen.lock().unwrap().len(), 4);
        }
    }
}
