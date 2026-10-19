//! Real-browser lifecycle fixture; no GPU provider or product test bypasses.

// Rust guideline compliant 2026-02-21

#[cfg(target_family = "wasm")]
mod browser {
    use gpui::PlatformInput;
    use gpui_web_foreign::{
        host::{self, ForeignRenderer},
        input::CompositionOp,
    };
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    use wasm_bindgen::prelude::*;

    thread_local! {
        static EVENTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }
    fn record(value: &str) {
        EVENTS.with(|events| events.borrow_mut().push(value.to_owned()));
    }

    struct Renderer {
        id: Rc<Cell<u32>>,
        case: u32,
        generation: u32,
        fired: bool,
        active: Rc<Cell<bool>>,
    }
    impl Renderer {
        fn run(&mut self, action: &str) {
            assert!(!self.active.replace(true), "renderer called recursively");
            record(&format!("enter:{}:{action}", self.generation));
            let first = !self.fired;
            if action == "down" {
                self.fired = true;
            }
            if (self.case == 1 && action == "frame")
                || (self.case == 2 && action == "down")
                || (self.case == 3 && action == "resize")
            {
                host::destroy_editor(self.id.get());
                assert!(self.active.get());
                record("closed-in-callback");
            }
            if self.case == 4 && action == "down" {
                host::set_renderer(
                    self.id.get(),
                    Renderer {
                        id: self.id.clone(),
                        case: 0,
                        generation: 2,
                        fired: false,
                        active: self.active.clone(),
                    },
                );
                js_sys::eval("window.__foreignNestedKey('keyup')").unwrap();
                record("replacement-queued");
            }
            if (self.case == 5 && action == "down" && first) || (self.case == 6 && action == "down")
            {
                js_sys::eval("window.__foreignNestedKey('keydown')").unwrap();
                record("nested-queued");
            }
            if self.case == 7 && action == "down" && first {
                js_sys::eval("window.__foreignNestedComposition()").unwrap();
                record("focus-composition-queued");
            }
            record(&format!("exit:{}:{action}", self.generation));
            self.active.set(false);
        }
    }
    impl Drop for Renderer {
        fn drop(&mut self) {
            assert!(!self.active.get(), "provider dropped during callback");
            record(&format!("drop:{}", self.generation));
        }
    }
    impl ForeignRenderer for Renderer {
        fn request_frame(&mut self) {
            self.run("frame");
        }
        fn resize(&mut self, _: (u32, u32)) {
            self.run("resize");
        }
        fn input(&mut self, input: PlatformInput) {
            match input {
                PlatformInput::KeyDown(_) => self.run("down"),
                PlatformInput::KeyUp(_) => self.run("up"),
                PlatformInput::ModifiersChanged(_) => {
                    record(&format!("modifiers:{}", self.generation));
                }
                _ => self.run("pointer"),
            }
        }
        fn composition(&mut self, op: CompositionOp) {
            match op {
                CompositionOp::SetMarked(text) => record(&format!("marked:{text}")),
                CompositionOp::Commit(text) => record(&format!("commit:{text}")),
            }
            self.run("composition");
        }
        fn set_active(&mut self, _: bool) {
            self.run("active");
        }
        fn set_hovered(&mut self, _: bool) {
            self.run("hovered");
        }
    }

    #[wasm_bindgen]
    pub fn start_case(case: u32) -> u32 {
        EVENTS.with(|events| events.borrow_mut().clear());
        let id = Rc::new(Cell::new(0));
        let result = host::create_editor_with_renderer(
            "foreign-canvas",
            "lifecycle",
            Renderer {
                id: id.clone(),
                case,
                generation: 1,
                fired: false,
                active: Rc::new(Cell::new(false)),
            },
        );
        assert_ne!(result, 0);
        id.set(result);
        result
    }
    #[wasm_bindgen]
    pub fn close_editor(id: u32) {
        host::destroy_editor(id);
    }
    #[wasm_bindgen]
    pub fn events() -> String {
        EVENTS.with(|events| events.borrow().join("|"))
    }
    #[wasm_bindgen]
    pub fn headless_frames(id: u32) -> usize {
        host::dropped_frames(id)
    }
}
