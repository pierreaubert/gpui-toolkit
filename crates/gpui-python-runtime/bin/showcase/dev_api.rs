//! Opt-in file mailbox for native input smoke tests and local debugging.
//!
//! Clients atomically publish request.json and read response.json in the directory
//! named by GPUI_TOOLKIT_DEV_API. Only one request may be outstanding per host.

// Rust guideline compliant 2026-02-21

use super::*;
use gpui_ui_kit::{
    Scene2DButton, Scene2DKeyPhase, Scene2DPointerDevice, Scene2DPointerEvent, Scene2DPointerPhase,
    ScenePoint,
};

// Poll only while explicitly enabled. This keeps the API responsive without
// adding a listener thread or any work to ordinary application sessions.
const POLL_INTERVAL: Duration = Duration::from_millis(25);
// Requests contain input coordinates, never application documents.
const MAX_REQUEST_BYTES: u64 = 64 * 1024;

#[derive(Debug, Deserialize)]
struct Request {
    id: String,
    #[serde(flatten)]
    command: Command,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum Command {
    Status,
    Select {
        section: String,
    },
    Click {
        surface: String,
        x: f32,
        y: f32,
    },
    Pointer {
        surface: String,
        phase: Scene2DPointerPhase,
        #[serde(default)]
        device: Scene2DPointerDevice,
        contact_id: u64,
        x: f32,
        y: f32,
    },
    Key {
        surface: String,
        phase: Scene2DKeyPhase,
        key: String,
    },
    WindowClick {
        x: f32,
        y: f32,
    },
}

impl PythonIrShowcase {
    pub(super) fn dev_bounds_element(&self, id: String) -> AnyElement {
        let store = self.dev_api_bounds.clone();
        canvas(
            move |bounds, _, _| {
                store.borrow_mut().insert(id.clone(), bounds);
            },
            |_, (), _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .into_any_element()
    }

    pub(super) fn start_dev_api(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.dev_api_task.is_some() {
            return;
        }
        let Some(directory) = env::var_os("GPUI_TOOLKIT_DEV_API").map(PathBuf::from) else {
            return;
        };
        if let Err(error) = fs::create_dir_all(&directory) {
            eprintln!("Cannot start native development API: {error}");
            return;
        }
        self.dev_api_task = Some(cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(POLL_INTERVAL).await;
                let request_path = directory.join("request.json");
                let bytes = match fs::metadata(&request_path) {
                    Ok(metadata) if metadata.len() <= MAX_REQUEST_BYTES => fs::read(&request_path),
                    Ok(_) => Err(std::io::Error::other(
                        "development API request exceeds 64 KiB",
                    )),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(error) => Err(error),
                };
                let request = bytes.map_err(|error| error.to_string()).and_then(|bytes| {
                    serde_json::from_slice::<Request>(&bytes).map_err(|error| error.to_string())
                });
                let response = match request {
                    Ok(request) => {
                        let id = request.id;
                        let outcome = match request.command {
                            Command::WindowClick { x, y } => {
                                cx.update(|window, app| dispatch_window_click(x, y, window, app))
                            }
                            command => this.update_in(cx, |this, window, cx| {
                                this.drain_session(cx);
                                let result = this.execute_dev_command(command, cx);
                                cx.notify();
                                window.refresh();
                                result
                            }),
                        };
                        // A developer command must work even when the platform
                        // compositor is not requesting frames (for example,
                        // an obscured window during a terminal smoke test).
                        if cx
                            .update(|window, app| {
                                window.refresh();
                                let arena = window.draw(app);
                                arena.clear();
                            })
                            .is_err()
                        {
                            break;
                        }
                        match outcome {
                            Ok(Ok(result)) => {
                                serde_json::json!({"id": id, "ok": true, "result": result})
                            }
                            Ok(Err(error)) => {
                                serde_json::json!({"id": id, "ok": false, "error": error})
                            }
                            Err(_) => break,
                        }
                    }
                    Err(error) => serde_json::json!({"ok": false, "error": error}),
                };
                // Remove the consumed request before publishing its response,
                // so a client can safely submit its next request immediately.
                let write_response = || -> std::io::Result<()> {
                    fs::remove_file(&request_path)?;
                    let temporary = directory.join("response.tmp");
                    fs::write(&temporary, serde_json::to_vec(&response)?)?;
                    fs::rename(temporary, directory.join("response.json"))
                };
                if let Err(error) = write_response() {
                    eprintln!("Cannot publish native development API response: {error}");
                }
            }
        }));
    }

    fn execute_dev_command(
        &mut self,
        command: Command,
        cx: &mut Context<Self>,
    ) -> Result<Value, String> {
        match command {
            Command::Status => Ok(serde_json::json!({
                "section": self.current_section,
                "window_active": self.window_active,
                "revision": self.session_state.revision(),
                "error": self.load_error,
                "surface_errors": self.scene2d_errors,
                "surfaces": self.scene2d_states.keys().collect::<Vec<_>>(),
                "bounds": self.dev_api_bounds.borrow().iter().map(|(id, bounds)| {
                    (id.clone(), serde_json::json!({
                        "x": bounds.origin.x.as_f32(), "y": bounds.origin.y.as_f32(),
                        "width": bounds.size.width.as_f32(), "height": bounds.size.height.as_f32(),
                    }))
                }).collect::<serde_json::Map<String, Value>>(),
                "app": self.app_value,
                "stderr": self.session.as_ref().map(|session| session.stderr_diagnostics()),
            })),
            Command::Select { section } => {
                if !self
                    .app
                    .as_ref()
                    .is_some_and(|app| app.sections.iter().any(|item| item.id == section))
                {
                    return Err(format!("Unknown section {section:?}"));
                }
                self.select_section(section, cx);
                Ok(Value::Null)
            }
            Command::WindowClick { .. } => Err("Window clicks require an unborrowed view".into()),
            Command::Click { surface, x, y } => {
                let position = dev_position(x, y)?;
                let state = self.dev_surface(&surface)?;
                let mut down = Scene2DPointerEvent::new(
                    Scene2DPointerPhase::Down,
                    Scene2DPointerDevice::Mouse,
                    1,
                    0,
                    position,
                );
                down.buttons.push(Scene2DButton::Left);
                let mut inputs = state.route_pointer(down);
                inputs.extend(state.route_pointer(Scene2DPointerEvent::new(
                    Scene2DPointerPhase::Up,
                    Scene2DPointerDevice::Mouse,
                    1,
                    0,
                    position,
                )));
                self.dev_dispatch(&surface, inputs)
            }
            Command::Pointer {
                surface,
                phase,
                device,
                contact_id,
                x,
                y,
            } => {
                let position = dev_position(x, y)?;
                let state = self.dev_surface(&surface)?;
                let mut event = Scene2DPointerEvent::new(phase, device, contact_id, 0, position);
                if device == Scene2DPointerDevice::Mouse
                    && phase != Scene2DPointerPhase::Up
                    && phase != Scene2DPointerPhase::Cancel
                {
                    event.buttons.push(Scene2DButton::Left);
                }
                self.dev_dispatch(&surface, state.route_pointer(event))
            }
            Command::Key {
                surface,
                phase,
                key,
            } => {
                let state = self.dev_surface(&surface)?;
                self.dev_dispatch(
                    &surface,
                    state
                        .route_key(phase, &key, false, Vec::new())
                        .into_iter()
                        .collect(),
                )
            }
        }
    }

    fn dev_surface(&self, id: &str) -> Result<Scene2DState, String> {
        if !self.app_value.as_ref().is_some_and(|app| {
            scene2d_surface_ids_in_section(app, &self.current_section)
                .iter()
                .any(|surface| surface == id)
        }) {
            return Err(format!("Surface {id:?} is not in the current section"));
        }
        self.scene2d_states
            .get(id)
            .cloned()
            .ok_or_else(|| format!("Surface {id:?} has not rendered yet"))
    }

    fn dev_dispatch(
        &self,
        surface: &str,
        inputs: Vec<gpui_ui_kit::Scene2DInput>,
    ) -> Result<Value, String> {
        if inputs.is_empty() {
            return Err(format!("Surface {surface:?} did not accept the input"));
        }
        let sink = self.scene2d_sink().ok_or("Python session is unavailable")?;
        let mut requests = Vec::new();
        for input in inputs {
            let event = serde_json::to_value(input).map_err(|error| error.to_string())?;
            let id = sink
                .dispatch_with_id(
                    surface,
                    "scene2d.event",
                    Some("scene2d_event".into()),
                    serde_json::json!({"type": "scene2d.event", "event": event}),
                )
                .map_err(|error| error.to_string())?;
            requests.push(serde_json::json!({"id": id, "event": event}));
        }
        Ok(serde_json::json!({"requests": requests}))
    }
}

fn dev_position(x: f32, y: f32) -> Result<ScenePoint, String> {
    if x.is_finite() && y.is_finite() {
        Ok(ScenePoint { x, y })
    } else {
        Err("Input coordinates must be finite".into())
    }
}

fn dispatch_window_click(
    x: f32,
    y: f32,
    window: &mut Window,
    app: &mut App,
) -> Result<Value, String> {
    let position = dev_position(x, y)?;
    window.dispatch_event(
        MouseDownEvent {
            position: point(px(position.x), px(position.y)),
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: 1,
            first_mouse: false,
        }
        .to_platform_input(),
        app,
    );
    window.dispatch_event(
        MouseUpEvent {
            position: point(px(position.x), px(position.y)),
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: 1,
        }
        .to_platform_input(),
        app,
    );
    Ok(Value::Null)
}
