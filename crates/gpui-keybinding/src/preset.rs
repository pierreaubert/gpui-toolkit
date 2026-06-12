use serde::{Deserialize, Serialize};

/// Available keymap presets.
///
/// Each preset defines a different editing/navigation style.
/// Applications implement [`KeybindingProvider`](crate::KeybindingProvider) to map
/// their actions to keys for each preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum KeymapPreset {
    #[default]
    Default,
    Vim,
    Emacs,
    VSCode,
}

impl KeymapPreset {
    /// Human-readable preset name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Vim => "Vim",
            Self::Emacs => "Emacs",
            Self::VSCode => "VSCode",
        }
    }

    /// Short description of the preset style.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Default => "Standard keybindings with arrow keys and platform shortcuts",
            Self::Vim => "Vim-style navigation with hjkl keys and modal commands",
            Self::Emacs => "Emacs-style navigation with Ctrl/Alt key combinations",
            Self::VSCode => "VSCode-style shortcuts familiar to many developers",
        }
    }

    /// Cycle to the next preset in order.
    pub fn next(&self) -> Self {
        match self {
            Self::Default => Self::Vim,
            Self::Vim => Self::Emacs,
            Self::Emacs => Self::VSCode,
            Self::VSCode => Self::Default,
        }
    }

    /// All available presets.
    pub fn all() -> &'static [KeymapPreset] {
        &[Self::Default, Self::Vim, Self::Emacs, Self::VSCode]
    }
}

impl std::fmt::Display for KeymapPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}
