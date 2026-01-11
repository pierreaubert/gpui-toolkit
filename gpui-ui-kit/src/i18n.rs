//! Internationalization (i18n) system for gpui-ui-kit
//!
//! Provides translation support with multiple languages.

use gpui::*;
use std::collections::HashMap;

/// Available languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Language {
    /// English (default)
    #[default]
    English,
    /// French
    French,
    /// German
    German,
    /// Spanish
    Spanish,
    /// Japanese
    Japanese,
}

impl Language {
    /// Get all available languages
    pub fn all() -> &'static [Language] {
        &[
            Language::English,
            Language::French,
            Language::German,
            Language::Spanish,
            Language::Japanese,
        ]
    }

    /// Get display name in the language itself
    pub fn native_name(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::French => "Francais",
            Language::German => "Deutsch",
            Language::Spanish => "Espanol",
            Language::Japanese => "Nihongo",
        }
    }

    /// Get language code (ISO 639-1)
    pub fn code(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::French => "fr",
            Language::German => "de",
            Language::Spanish => "es",
            Language::Japanese => "ja",
        }
    }

    /// Get flag emoji
    pub fn flag(&self) -> &'static str {
        match self {
            Language::English => "GB",
            Language::French => "FR",
            Language::German => "DE",
            Language::Spanish => "ES",
            Language::Japanese => "JP",
        }
    }
}

/// Translation keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TranslationKey {
    // App
    AppTitle,
    AppSubtitle,

    // Menu
    MenuFile,
    MenuEdit,
    MenuView,
    MenuHelp,
    MenuQuit,
    MenuTheme,
    MenuLanguage,
    MenuSettings,

    // Theme
    ThemeDark,
    ThemeLight,

    // Section titles
    SectionButtons,
    SectionTypography,
    SectionBadges,
    SectionAvatars,
    SectionFormControls,
    SectionProgress,
    SectionAlerts,
    SectionTabs,
    SectionCards,
    SectionBreadcrumbs,
    SectionSpinners,
    SectionLayout,
    SectionIconButtons,
    SectionToasts,
    SectionDialogs,
    SectionMenus,
    SectionTooltips,
    SectionPotentiometers,
    SectionAccordion,

    // Component labels
    LabelVariants,
    LabelSizes,
    LabelStates,
    LabelToggles,
    LabelCheckboxes,
    LabelSlider,
    LabelInput,
    LabelSmall,
    LabelMedium,
    LabelLarge,
    LabelDisabled,
    LabelSelected,

    // Button labels
    ButtonPrimary,
    ButtonSecondary,
    ButtonDestructive,
    ButtonGhost,
    ButtonOutline,
    ButtonCancel,
    ButtonSave,
    ButtonConfirm,

    // Dialog
    DialogConfirmTitle,
    DialogConfirmMessage,

    // Alerts
    AlertInfo,
    AlertSuccess,
    AlertWarning,
    AlertError,
    AlertInfoMessage,
    AlertSuccessMessage,
    AlertWarningMessage,
    AlertErrorMessage,

    // Accordion
    AccordionGettingStarted,
    AccordionFeatures,
    AccordionConfiguration,
}

/// Translations storage
pub struct Translations {
    translations: HashMap<(Language, TranslationKey), &'static str>,
}

impl Translations {
    /// Create new translations with all built-in strings
    pub fn new() -> Self {
        let mut translations = HashMap::new();

        // English translations
        Self::add_english(&mut translations);
        // French translations
        Self::add_french(&mut translations);
        // German translations
        Self::add_german(&mut translations);
        // Spanish translations
        Self::add_spanish(&mut translations);
        // Japanese translations
        Self::add_japanese(&mut translations);

        Self { translations }
    }

    fn add_english(t: &mut HashMap<(Language, TranslationKey), &'static str>) {
        use Language::English as L;
        use TranslationKey::*;

        t.insert((L, AppTitle), "GPUI UI Kit Showcase");
        t.insert(
            (L, AppSubtitle),
            "A comprehensive library of reusable UI components for GPUI applications.",
        );

        // Menu
        t.insert((L, MenuFile), "File");
        t.insert((L, MenuEdit), "Edit");
        t.insert((L, MenuView), "View");
        t.insert((L, MenuHelp), "Help");
        t.insert((L, MenuQuit), "Quit");
        t.insert((L, MenuTheme), "Theme");
        t.insert((L, MenuLanguage), "Language");
        t.insert((L, MenuSettings), "Settings");

        // Theme
        t.insert((L, ThemeDark), "Dark");
        t.insert((L, ThemeLight), "Light");

        // Sections
        t.insert((L, SectionButtons), "Buttons");
        t.insert((L, SectionTypography), "Typography");
        t.insert((L, SectionBadges), "Badges");
        t.insert((L, SectionAvatars), "Avatars");
        t.insert((L, SectionFormControls), "Form Controls");
        t.insert((L, SectionProgress), "Progress Indicators");
        t.insert((L, SectionAlerts), "Alerts");
        t.insert((L, SectionTabs), "Tabs");
        t.insert((L, SectionCards), "Cards");
        t.insert((L, SectionBreadcrumbs), "Breadcrumbs");
        t.insert((L, SectionSpinners), "Loading Indicators");
        t.insert((L, SectionLayout), "Layout Components");
        t.insert((L, SectionIconButtons), "Icon Buttons");
        t.insert((L, SectionToasts), "Toasts");
        t.insert((L, SectionDialogs), "Dialogs");
        t.insert((L, SectionMenus), "Menus");
        t.insert((L, SectionTooltips), "Tooltips");
        t.insert((L, SectionPotentiometers), "Potentiometers");
        t.insert((L, SectionAccordion), "Accordion");

        // Labels
        t.insert((L, LabelVariants), "Variants");
        t.insert((L, LabelSizes), "Sizes");
        t.insert((L, LabelStates), "States");
        t.insert((L, LabelToggles), "Toggles");
        t.insert((L, LabelCheckboxes), "Checkboxes");
        t.insert((L, LabelSlider), "Slider");
        t.insert((L, LabelInput), "Input");
        t.insert((L, LabelSmall), "Small");
        t.insert((L, LabelMedium), "Medium");
        t.insert((L, LabelLarge), "Large");
        t.insert((L, LabelDisabled), "Disabled");
        t.insert((L, LabelSelected), "Selected");

        // Buttons
        t.insert((L, ButtonPrimary), "Primary");
        t.insert((L, ButtonSecondary), "Secondary");
        t.insert((L, ButtonDestructive), "Destructive");
        t.insert((L, ButtonGhost), "Ghost");
        t.insert((L, ButtonOutline), "Outline");
        t.insert((L, ButtonCancel), "Cancel");
        t.insert((L, ButtonSave), "Save");
        t.insert((L, ButtonConfirm), "Confirm");

        // Dialog
        t.insert((L, DialogConfirmTitle), "Confirm Action");
        t.insert(
            (L, DialogConfirmMessage),
            "Are you sure you want to continue? This action cannot be undone.",
        );

        // Alerts
        t.insert((L, AlertInfo), "Information");
        t.insert((L, AlertSuccess), "Success");
        t.insert((L, AlertWarning), "Warning");
        t.insert((L, AlertError), "Error");
        t.insert((L, AlertInfoMessage), "This is an informational message.");
        t.insert(
            (L, AlertSuccessMessage),
            "Your changes have been saved successfully.",
        );
        t.insert(
            (L, AlertWarningMessage),
            "Please review your settings before continuing.",
        );
        t.insert(
            (L, AlertErrorMessage),
            "An error occurred while processing your request.",
        );

        // Accordion
        t.insert((L, AccordionGettingStarted), "Getting Started");
        t.insert((L, AccordionFeatures), "Features");
        t.insert((L, AccordionConfiguration), "Configuration");
    }

    fn add_french(t: &mut HashMap<(Language, TranslationKey), &'static str>) {
        use Language::French as L;
        use TranslationKey::*;

        t.insert((L, AppTitle), "Vitrine du UI Kit GPUI");
        t.insert(
            (L, AppSubtitle),
            "Une bibliotheque complete de composants UI reutilisables pour les applications GPUI.",
        );

        // Menu
        t.insert((L, MenuFile), "Fichier");
        t.insert((L, MenuEdit), "Edition");
        t.insert((L, MenuView), "Affichage");
        t.insert((L, MenuHelp), "Aide");
        t.insert((L, MenuQuit), "Quitter");
        t.insert((L, MenuTheme), "Theme");
        t.insert((L, MenuLanguage), "Langue");
        t.insert((L, MenuSettings), "Parametres");

        // Theme
        t.insert((L, ThemeDark), "Sombre");
        t.insert((L, ThemeLight), "Clair");

        // Sections
        t.insert((L, SectionButtons), "Boutons");
        t.insert((L, SectionTypography), "Typographie");
        t.insert((L, SectionBadges), "Badges");
        t.insert((L, SectionAvatars), "Avatars");
        t.insert((L, SectionFormControls), "Controles de formulaire");
        t.insert((L, SectionProgress), "Indicateurs de progression");
        t.insert((L, SectionAlerts), "Alertes");
        t.insert((L, SectionTabs), "Onglets");
        t.insert((L, SectionCards), "Cartes");
        t.insert((L, SectionBreadcrumbs), "Fil d'Ariane");
        t.insert((L, SectionSpinners), "Indicateurs de chargement");
        t.insert((L, SectionLayout), "Composants de mise en page");
        t.insert((L, SectionIconButtons), "Boutons icones");
        t.insert((L, SectionToasts), "Notifications");
        t.insert((L, SectionDialogs), "Dialogues");
        t.insert((L, SectionMenus), "Menus");
        t.insert((L, SectionTooltips), "Infobulles");
        t.insert((L, SectionPotentiometers), "Potentiometres");
        t.insert((L, SectionAccordion), "Accordeon");

        // Labels
        t.insert((L, LabelVariants), "Variantes");
        t.insert((L, LabelSizes), "Tailles");
        t.insert((L, LabelStates), "Etats");
        t.insert((L, LabelToggles), "Interrupteurs");
        t.insert((L, LabelCheckboxes), "Cases a cocher");
        t.insert((L, LabelSlider), "Curseur");
        t.insert((L, LabelInput), "Champ de saisie");
        t.insert((L, LabelSmall), "Petit");
        t.insert((L, LabelMedium), "Moyen");
        t.insert((L, LabelLarge), "Grand");
        t.insert((L, LabelDisabled), "Desactive");
        t.insert((L, LabelSelected), "Selectionne");

        // Buttons
        t.insert((L, ButtonPrimary), "Primaire");
        t.insert((L, ButtonSecondary), "Secondaire");
        t.insert((L, ButtonDestructive), "Destructif");
        t.insert((L, ButtonGhost), "Fantome");
        t.insert((L, ButtonOutline), "Contour");
        t.insert((L, ButtonCancel), "Annuler");
        t.insert((L, ButtonSave), "Enregistrer");
        t.insert((L, ButtonConfirm), "Confirmer");

        // Dialog
        t.insert((L, DialogConfirmTitle), "Confirmer l'action");
        t.insert(
            (L, DialogConfirmMessage),
            "Etes-vous sur de vouloir continuer ? Cette action est irreversible.",
        );

        // Alerts
        t.insert((L, AlertInfo), "Information");
        t.insert((L, AlertSuccess), "Succes");
        t.insert((L, AlertWarning), "Avertissement");
        t.insert((L, AlertError), "Erreur");
        t.insert((L, AlertInfoMessage), "Ceci est un message d'information.");
        t.insert(
            (L, AlertSuccessMessage),
            "Vos modifications ont ete enregistrees avec succes.",
        );
        t.insert(
            (L, AlertWarningMessage),
            "Veuillez verifier vos parametres avant de continuer.",
        );
        t.insert(
            (L, AlertErrorMessage),
            "Une erreur s'est produite lors du traitement de votre demande.",
        );

        // Accordion
        t.insert((L, AccordionGettingStarted), "Premiers pas");
        t.insert((L, AccordionFeatures), "Fonctionnalites");
        t.insert((L, AccordionConfiguration), "Configuration");
    }

    fn add_german(t: &mut HashMap<(Language, TranslationKey), &'static str>) {
        use Language::German as L;
        use TranslationKey::*;

        t.insert((L, AppTitle), "GPUI UI Kit Showcase");
        t.insert(
            (L, AppSubtitle),
            "Eine umfassende Bibliothek wiederverwendbarer UI-Komponenten fur GPUI-Anwendungen.",
        );

        // Menu
        t.insert((L, MenuFile), "Datei");
        t.insert((L, MenuEdit), "Bearbeiten");
        t.insert((L, MenuView), "Ansicht");
        t.insert((L, MenuHelp), "Hilfe");
        t.insert((L, MenuQuit), "Beenden");
        t.insert((L, MenuTheme), "Thema");
        t.insert((L, MenuLanguage), "Sprache");
        t.insert((L, MenuSettings), "Einstellungen");

        // Theme
        t.insert((L, ThemeDark), "Dunkel");
        t.insert((L, ThemeLight), "Hell");

        // Sections
        t.insert((L, SectionButtons), "Schaltflachen");
        t.insert((L, SectionTypography), "Typografie");
        t.insert((L, SectionBadges), "Abzeichen");
        t.insert((L, SectionAvatars), "Avatare");
        t.insert((L, SectionFormControls), "Formularsteuerung");
        t.insert((L, SectionProgress), "Fortschrittsanzeigen");
        t.insert((L, SectionAlerts), "Warnungen");
        t.insert((L, SectionTabs), "Registerkarten");
        t.insert((L, SectionCards), "Karten");
        t.insert((L, SectionBreadcrumbs), "Brotkrumen");
        t.insert((L, SectionSpinners), "Ladeanzeigen");
        t.insert((L, SectionLayout), "Layout-Komponenten");
        t.insert((L, SectionIconButtons), "Symbol-Schaltflachen");
        t.insert((L, SectionToasts), "Benachrichtigungen");
        t.insert((L, SectionDialogs), "Dialoge");
        t.insert((L, SectionMenus), "Menus");
        t.insert((L, SectionTooltips), "Tooltips");
        t.insert((L, SectionPotentiometers), "Potentiometer");
        t.insert((L, SectionAccordion), "Akkordeon");

        // Labels
        t.insert((L, LabelVariants), "Varianten");
        t.insert((L, LabelSizes), "Grossen");
        t.insert((L, LabelStates), "Zustande");
        t.insert((L, LabelToggles), "Schalter");
        t.insert((L, LabelCheckboxes), "Kontrollkastchen");
        t.insert((L, LabelSlider), "Schieberegler");
        t.insert((L, LabelInput), "Eingabe");
        t.insert((L, LabelSmall), "Klein");
        t.insert((L, LabelMedium), "Mittel");
        t.insert((L, LabelLarge), "Gross");
        t.insert((L, LabelDisabled), "Deaktiviert");
        t.insert((L, LabelSelected), "Ausgewahlt");

        // Buttons
        t.insert((L, ButtonPrimary), "Primar");
        t.insert((L, ButtonSecondary), "Sekundar");
        t.insert((L, ButtonDestructive), "Destruktiv");
        t.insert((L, ButtonGhost), "Geist");
        t.insert((L, ButtonOutline), "Umriss");
        t.insert((L, ButtonCancel), "Abbrechen");
        t.insert((L, ButtonSave), "Speichern");
        t.insert((L, ButtonConfirm), "Bestatigen");

        // Dialog
        t.insert((L, DialogConfirmTitle), "Aktion bestatigen");
        t.insert(
            (L, DialogConfirmMessage),
            "Sind Sie sicher, dass Sie fortfahren mochten? Diese Aktion kann nicht ruckgangig gemacht werden.",
        );

        // Alerts
        t.insert((L, AlertInfo), "Information");
        t.insert((L, AlertSuccess), "Erfolg");
        t.insert((L, AlertWarning), "Warnung");
        t.insert((L, AlertError), "Fehler");
        t.insert((L, AlertInfoMessage), "Dies ist eine Informationsmeldung.");
        t.insert(
            (L, AlertSuccessMessage),
            "Ihre Anderungen wurden erfolgreich gespeichert.",
        );
        t.insert(
            (L, AlertWarningMessage),
            "Bitte uberprufen Sie Ihre Einstellungen, bevor Sie fortfahren.",
        );
        t.insert(
            (L, AlertErrorMessage),
            "Bei der Verarbeitung Ihrer Anfrage ist ein Fehler aufgetreten.",
        );

        // Accordion
        t.insert((L, AccordionGettingStarted), "Erste Schritte");
        t.insert((L, AccordionFeatures), "Funktionen");
        t.insert((L, AccordionConfiguration), "Konfiguration");
    }

    fn add_spanish(t: &mut HashMap<(Language, TranslationKey), &'static str>) {
        use Language::Spanish as L;
        use TranslationKey::*;

        t.insert((L, AppTitle), "Galeria del UI Kit GPUI");
        t.insert(
            (L, AppSubtitle),
            "Una biblioteca completa de componentes UI reutilizables para aplicaciones GPUI.",
        );

        // Menu
        t.insert((L, MenuFile), "Archivo");
        t.insert((L, MenuEdit), "Editar");
        t.insert((L, MenuView), "Ver");
        t.insert((L, MenuHelp), "Ayuda");
        t.insert((L, MenuQuit), "Salir");
        t.insert((L, MenuTheme), "Tema");
        t.insert((L, MenuLanguage), "Idioma");
        t.insert((L, MenuSettings), "Configuracion");

        // Theme
        t.insert((L, ThemeDark), "Oscuro");
        t.insert((L, ThemeLight), "Claro");

        // Sections
        t.insert((L, SectionButtons), "Botones");
        t.insert((L, SectionTypography), "Tipografia");
        t.insert((L, SectionBadges), "Insignias");
        t.insert((L, SectionAvatars), "Avatares");
        t.insert((L, SectionFormControls), "Controles de formulario");
        t.insert((L, SectionProgress), "Indicadores de progreso");
        t.insert((L, SectionAlerts), "Alertas");
        t.insert((L, SectionTabs), "Pestanas");
        t.insert((L, SectionCards), "Tarjetas");
        t.insert((L, SectionBreadcrumbs), "Migas de pan");
        t.insert((L, SectionSpinners), "Indicadores de carga");
        t.insert((L, SectionLayout), "Componentes de diseno");
        t.insert((L, SectionIconButtons), "Botones de icono");
        t.insert((L, SectionToasts), "Notificaciones");
        t.insert((L, SectionDialogs), "Dialogos");
        t.insert((L, SectionMenus), "Menus");
        t.insert((L, SectionTooltips), "Sugerencias");
        t.insert((L, SectionPotentiometers), "Potenciometros");
        t.insert((L, SectionAccordion), "Acordeon");

        // Labels
        t.insert((L, LabelVariants), "Variantes");
        t.insert((L, LabelSizes), "Tamanos");
        t.insert((L, LabelStates), "Estados");
        t.insert((L, LabelToggles), "Interruptores");
        t.insert((L, LabelCheckboxes), "Casillas");
        t.insert((L, LabelSlider), "Deslizador");
        t.insert((L, LabelInput), "Entrada");
        t.insert((L, LabelSmall), "Pequeno");
        t.insert((L, LabelMedium), "Mediano");
        t.insert((L, LabelLarge), "Grande");
        t.insert((L, LabelDisabled), "Desactivado");
        t.insert((L, LabelSelected), "Seleccionado");

        // Buttons
        t.insert((L, ButtonPrimary), "Primario");
        t.insert((L, ButtonSecondary), "Secundario");
        t.insert((L, ButtonDestructive), "Destructivo");
        t.insert((L, ButtonGhost), "Fantasma");
        t.insert((L, ButtonOutline), "Contorno");
        t.insert((L, ButtonCancel), "Cancelar");
        t.insert((L, ButtonSave), "Guardar");
        t.insert((L, ButtonConfirm), "Confirmar");

        // Dialog
        t.insert((L, DialogConfirmTitle), "Confirmar accion");
        t.insert(
            (L, DialogConfirmMessage),
            "Esta seguro de que desea continuar? Esta accion no se puede deshacer.",
        );

        // Alerts
        t.insert((L, AlertInfo), "Informacion");
        t.insert((L, AlertSuccess), "Exito");
        t.insert((L, AlertWarning), "Advertencia");
        t.insert((L, AlertError), "Error");
        t.insert((L, AlertInfoMessage), "Este es un mensaje informativo.");
        t.insert(
            (L, AlertSuccessMessage),
            "Sus cambios se han guardado correctamente.",
        );
        t.insert(
            (L, AlertWarningMessage),
            "Por favor revise su configuracion antes de continuar.",
        );
        t.insert(
            (L, AlertErrorMessage),
            "Se produjo un error al procesar su solicitud.",
        );

        // Accordion
        t.insert((L, AccordionGettingStarted), "Primeros pasos");
        t.insert((L, AccordionFeatures), "Caracteristicas");
        t.insert((L, AccordionConfiguration), "Configuracion");
    }

    fn add_japanese(t: &mut HashMap<(Language, TranslationKey), &'static str>) {
        use Language::Japanese as L;
        use TranslationKey::*;

        t.insert((L, AppTitle), "GPUI UI Kit Showcase");
        t.insert(
            (L, AppSubtitle),
            "GPUIアプリケーション用の再利用可能なUIコンポーネントの包括的なライブラリ。",
        );

        // Menu
        t.insert((L, MenuFile), "ファイル");
        t.insert((L, MenuEdit), "編集");
        t.insert((L, MenuView), "表示");
        t.insert((L, MenuHelp), "ヘルプ");
        t.insert((L, MenuQuit), "終了");
        t.insert((L, MenuTheme), "テーマ");
        t.insert((L, MenuLanguage), "言語");
        t.insert((L, MenuSettings), "設定");

        // Theme
        t.insert((L, ThemeDark), "ダーク");
        t.insert((L, ThemeLight), "ライト");

        // Sections
        t.insert((L, SectionButtons), "ボタン");
        t.insert((L, SectionTypography), "タイポグラフィ");
        t.insert((L, SectionBadges), "バッジ");
        t.insert((L, SectionAvatars), "アバター");
        t.insert((L, SectionFormControls), "フォームコントロール");
        t.insert((L, SectionProgress), "進捗インジケーター");
        t.insert((L, SectionAlerts), "アラート");
        t.insert((L, SectionTabs), "タブ");
        t.insert((L, SectionCards), "カード");
        t.insert((L, SectionBreadcrumbs), "パンくずリスト");
        t.insert((L, SectionSpinners), "読み込みインジケーター");
        t.insert((L, SectionLayout), "レイアウトコンポーネント");
        t.insert((L, SectionIconButtons), "アイコンボタン");
        t.insert((L, SectionToasts), "トースト");
        t.insert((L, SectionDialogs), "ダイアログ");
        t.insert((L, SectionMenus), "メニュー");
        t.insert((L, SectionTooltips), "ツールチップ");
        t.insert((L, SectionPotentiometers), "ポテンショメーター");
        t.insert((L, SectionAccordion), "アコーディオン");

        // Labels
        t.insert((L, LabelVariants), "バリエーション");
        t.insert((L, LabelSizes), "サイズ");
        t.insert((L, LabelStates), "状態");
        t.insert((L, LabelToggles), "トグル");
        t.insert((L, LabelCheckboxes), "チェックボックス");
        t.insert((L, LabelSlider), "スライダー");
        t.insert((L, LabelInput), "入力");
        t.insert((L, LabelSmall), "小");
        t.insert((L, LabelMedium), "中");
        t.insert((L, LabelLarge), "大");
        t.insert((L, LabelDisabled), "無効");
        t.insert((L, LabelSelected), "選択済み");

        // Buttons
        t.insert((L, ButtonPrimary), "プライマリ");
        t.insert((L, ButtonSecondary), "セカンダリ");
        t.insert((L, ButtonDestructive), "破壊的");
        t.insert((L, ButtonGhost), "ゴースト");
        t.insert((L, ButtonOutline), "アウトライン");
        t.insert((L, ButtonCancel), "キャンセル");
        t.insert((L, ButtonSave), "保存");
        t.insert((L, ButtonConfirm), "確認");

        // Dialog
        t.insert((L, DialogConfirmTitle), "アクションの確認");
        t.insert(
            (L, DialogConfirmMessage),
            "続行してもよろしいですか？この操作は元に戻せません。",
        );

        // Alerts
        t.insert((L, AlertInfo), "情報");
        t.insert((L, AlertSuccess), "成功");
        t.insert((L, AlertWarning), "警告");
        t.insert((L, AlertError), "エラー");
        t.insert((L, AlertInfoMessage), "これは情報メッセージです。");
        t.insert((L, AlertSuccessMessage), "変更が正常に保存されました。");
        t.insert(
            (L, AlertWarningMessage),
            "続行する前に設定を確認してください。",
        );
        t.insert(
            (L, AlertErrorMessage),
            "リクエストの処理中にエラーが発生しました。",
        );

        // Accordion
        t.insert((L, AccordionGettingStarted), "はじめに");
        t.insert((L, AccordionFeatures), "機能");
        t.insert((L, AccordionConfiguration), "設定");
    }

    /// Get translation for a key
    pub fn get(&self, lang: Language, key: TranslationKey) -> &'static str {
        self.translations
            .get(&(lang, key))
            .copied()
            .or_else(|| self.translations.get(&(Language::English, key)).copied())
            .unwrap_or("???")
    }
}

impl Default for Translations {
    fn default() -> Self {
        Self::new()
    }
}

/// Global state for i18n management
pub struct I18nState {
    pub language: Language,
    pub translations: Translations,
}

impl Global for I18nState {}

impl I18nState {
    /// Create new i18n state with default (English) language
    pub fn new() -> Self {
        Self {
            language: Language::default(),
            translations: Translations::new(),
        }
    }

    /// Set language
    pub fn set_language(&mut self, language: Language) {
        self.language = language;
    }

    /// Get translation for current language
    pub fn t(&self, key: TranslationKey) -> &'static str {
        self.translations.get(self.language, key)
    }
}

impl Default for I18nState {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for easy i18n access
pub trait I18nExt {
    /// Get translation for current language
    fn t(&self, key: TranslationKey) -> &'static str;

    /// Get current language
    fn language(&self) -> Language;
}

impl I18nExt for App {
    fn t(&self, key: TranslationKey) -> &'static str {
        self.try_global::<I18nState>()
            .map(|s| s.t(key))
            .unwrap_or("???")
    }

    fn language(&self) -> Language {
        self.try_global::<I18nState>()
            .map(|s| s.language)
            .unwrap_or_default()
    }
}
