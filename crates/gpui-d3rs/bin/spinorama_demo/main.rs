use gpui::*;

mod app;
mod render;
mod types;
mod utils;

use app::SpinoramaApp;

// Define actions
actions!(spinorama_demo, [Quit]);

fn main() {
    Application::new().run(|cx| {
        // Activate app and register quit action
        cx.activate(true);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);

        // Set up application menu
        cx.set_menus(vec![Menu {
            name: "Spinorama Viewer".into(),
            items: vec![
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Quit Spinorama Viewer", Quit),
            ],
        }]);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: point(px(100.0), px(100.0)),
                    size: size(px(1200.0), px(800.0)),
                })),
                titlebar: Some(TitlebarOptions {
                    title: Some("Spinorama Viewer".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_window, cx| cx.new(SpinoramaApp::new),
        )
        .expect("Failed to open window");
    });
}
