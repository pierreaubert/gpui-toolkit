use crate :: { MacKeyboardMapper , pasteboard :: Pasteboard , renderer } ;
use cocoa :: { base :: { id } } ;
use gpui :: { Action , BackgroundExecutor , ForegroundExecutor , OwnedMenu , PlatformTextSystem } ;
use std :: { rc :: Rc , sync :: { Arc } } ;

pub(crate) struct MacPlatformState {
    pub(super) background_executor: BackgroundExecutor,
    pub(super) foreground_executor: ForegroundExecutor,
    pub(super) text_system: Arc<dyn PlatformTextSystem>,
    pub(super) renderer_context: renderer::Context,
    pub(super) headless: bool,
    pub(super) general_pasteboard: Pasteboard,
    pub(super) find_pasteboard: Pasteboard,
    pub(super) reopen: Option<Box<dyn FnMut()>>,
    pub(super) on_keyboard_layout_change: Option<Box<dyn FnMut()>>,
    pub(super) on_thermal_state_change: Option<Box<dyn FnMut()>>,
    pub(super) quit: Option<Box<dyn FnMut()>>,
    pub(super) menu_command: Option<Box<dyn FnMut(&dyn Action)>>,
    pub(super) validate_menu_command: Option<Box<dyn FnMut(&dyn Action) -> bool>>,
    pub(super) will_open_menu: Option<Box<dyn FnMut()>>,
    pub(super) menu_actions: Vec<Box<dyn Action>>,
    pub(super) open_urls: Option<Box<dyn FnMut(Vec<String>)>>,
    pub(super) finish_launching: Option<Box<dyn FnOnce()>>,
    pub(super) dock_menu: Option<id>,
    pub(super) menus: Option<Vec<OwnedMenu>>,
    pub(super) keyboard_mapper: Rc<MacKeyboardMapper>,
}

