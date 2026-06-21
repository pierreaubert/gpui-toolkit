use collections::HashMap;
use gpui :: { KeybindingKeystroke , Keystroke , PlatformKeyboardMapper } ;
use super::misc::get_key_equivalents;

pub(crate) struct MacKeyboardMapper {
    pub(super) key_equivalents: Option<HashMap<char, char>>,
}

impl PlatformKeyboardMapper for MacKeyboardMapper {
    fn map_key_equivalent(
        &self,
        mut keystroke: Keystroke,
        use_key_equivalents: bool,
    ) -> KeybindingKeystroke {
        if use_key_equivalents && let Some(key_equivalents) = &self.key_equivalents {
            if keystroke.key.chars().count() == 1
                && let Some(key) = key_equivalents.get(&keystroke.key.chars().next().unwrap())
            {
                keystroke.key = key.to_string();
            }
        }
        KeybindingKeystroke::from_keystroke(keystroke)
    }

    fn get_key_equivalents(&self) -> Option<&HashMap<char, char>> {
        self.key_equivalents.as_ref()
    }
}

impl MacKeyboardMapper {
    pub(crate) fn new(layout_id: &str) -> Self {
        let key_equivalents = get_key_equivalents(layout_id);

        Self { key_equivalents }
    }
}

