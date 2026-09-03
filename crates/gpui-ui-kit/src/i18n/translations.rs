use super::language::Language;
use super::types::TranslationKey;
use std::collections::HashMap;

/// Translations storage
pub struct Translations {
    pub(super) translations: HashMap<(Language, TranslationKey), &'static str>,
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

    pub(super) fn add_english(t: &mut HashMap<(Language, TranslationKey), &'static str>) {
        use Language::English as L;

        t.insert((L, TranslationKey::AppTitle), "GPUI UI Kit Showcase");
        t.insert(
            (L, TranslationKey::AppSubtitle),
            "A comprehensive library of reusable UI components for GPUI applications.",
        );

        // Menu
        t.insert((L, TranslationKey::MenuFile), "File");
        t.insert((L, TranslationKey::MenuEdit), "Edit");
        t.insert((L, TranslationKey::MenuView), "View");
        t.insert((L, TranslationKey::MenuHelp), "Help");
        t.insert((L, TranslationKey::MenuQuit), "Quit");
        t.insert((L, TranslationKey::MenuTheme), "Theme");
        t.insert((L, TranslationKey::MenuLanguage), "Language");
        t.insert((L, TranslationKey::MenuSettings), "Settings");

        // Theme
        t.insert((L, TranslationKey::ThemeDark), "Dark");
        t.insert((L, TranslationKey::ThemeLight), "Light");

        // Sections
        t.insert((L, TranslationKey::SectionButtons), "Buttons");
        t.insert((L, TranslationKey::SectionTypography), "Typography");
        t.insert((L, TranslationKey::SectionBadges), "Badges");
        t.insert((L, TranslationKey::SectionAvatars), "Avatars");
        t.insert((L, TranslationKey::SectionFormControls), "Form Controls");
        t.insert((L, TranslationKey::SectionProgress), "Progress Indicators");
        t.insert((L, TranslationKey::SectionAlerts), "Alerts");
        t.insert((L, TranslationKey::SectionTabs), "Tabs");
        t.insert((L, TranslationKey::SectionCards), "Cards");
        t.insert((L, TranslationKey::SectionBreadcrumbs), "Breadcrumbs");
        t.insert((L, TranslationKey::SectionSpinners), "Loading Indicators");
        t.insert((L, TranslationKey::SectionLayout), "Layout Components");
        t.insert((L, TranslationKey::SectionIconButtons), "Icon Buttons");
        t.insert((L, TranslationKey::SectionToasts), "Toasts");
        t.insert((L, TranslationKey::SectionDialogs), "Dialogs");
        t.insert((L, TranslationKey::SectionMenus), "Menus");
        t.insert((L, TranslationKey::SectionTooltips), "Tooltips");
        t.insert((L, TranslationKey::SectionPotentiometers), "Potentiometers");
        t.insert((L, TranslationKey::SectionAccordion), "Accordion");
        t.insert((L, TranslationKey::SectionQrCode), "QR Code");
        t.insert((L, TranslationKey::SectionContextMenu), "Context Menu");
        t.insert((L, TranslationKey::SectionPopover), "Popover");
        t.insert((L, TranslationKey::SectionSidebar), "Sidebar");
        t.insert((L, TranslationKey::SectionStatusBar), "Status Bar");
        t.insert((L, TranslationKey::SectionSearchBar), "Search Bar");
        t.insert(
            (L, TranslationKey::SectionKeyboardShortcut),
            "Keyboard Shortcuts",
        );
        t.insert((L, TranslationKey::SectionEmptyState), "Empty State");
        t.insert((L, TranslationKey::SectionConfirmDialog), "Confirm Dialog");
        t.insert((L, TranslationKey::SectionSplitPane), "Split Pane");
        t.insert((L, TranslationKey::SectionImageView), "Image View");
        t.insert((L, TranslationKey::SectionSettingsForm), "Settings Form");
        t.insert((L, TranslationKey::SectionStepIndicator), "Step Indicator");
        t.insert(
            (L, TranslationKey::SectionLoadingOverlay),
            "Loading Overlay",
        );
        t.insert((L, TranslationKey::SectionTag), "Tag");
        t.insert((L, TranslationKey::SectionToolbar), "Toolbar");
        t.insert((L, TranslationKey::SectionNotification), "Notification");
        t.insert((L, TranslationKey::SectionTreeView), "Tree View");
        t.insert((L, TranslationKey::SectionDragList), "Drag List");
        t.insert(
            (L, TranslationKey::SectionCommandPalette),
            "Command Palette",
        );
        t.insert((L, TranslationKey::SectionAccessibility), "Accessibility");
        t.insert((L, TranslationKey::SectionThinkingOrbs), "Thinking Orbs");

        // Labels
        t.insert((L, TranslationKey::LabelVariants), "Variants");
        t.insert((L, TranslationKey::LabelSizes), "Sizes");
        t.insert((L, TranslationKey::LabelStates), "States");
        t.insert((L, TranslationKey::LabelToggles), "Toggles");
        t.insert((L, TranslationKey::LabelCheckboxes), "Checkboxes");
        t.insert((L, TranslationKey::LabelRadioGroup), "Radio group");
        t.insert((L, TranslationKey::LabelCombobox), "Combobox");
        t.insert((L, TranslationKey::LabelDatePicker), "Date picker");
        t.insert((L, TranslationKey::LabelSlider), "Slider");
        t.insert((L, TranslationKey::LabelInput), "Input");
        t.insert((L, TranslationKey::LabelSmall), "Small");
        t.insert((L, TranslationKey::LabelMedium), "Medium");
        t.insert((L, TranslationKey::LabelLarge), "Large");
        t.insert((L, TranslationKey::LabelDisabled), "Disabled");
        t.insert((L, TranslationKey::LabelSelected), "Selected");

        // Buttons
        t.insert((L, TranslationKey::ButtonPrimary), "Primary");
        t.insert((L, TranslationKey::ButtonSecondary), "Secondary");
        t.insert((L, TranslationKey::ButtonDestructive), "Destructive");
        t.insert((L, TranslationKey::ButtonGhost), "Ghost");
        t.insert((L, TranslationKey::ButtonOutline), "Outline");
        t.insert((L, TranslationKey::ButtonCancel), "Cancel");
        t.insert((L, TranslationKey::ButtonSave), "Save");
        t.insert((L, TranslationKey::ButtonConfirm), "Confirm");

        // Dialog
        t.insert((L, TranslationKey::DialogConfirmTitle), "Confirm Action");
        t.insert(
            (L, TranslationKey::DialogConfirmMessage),
            "Are you sure you want to continue? This action cannot be undone.",
        );

        // Alerts
        t.insert((L, TranslationKey::AlertInfo), "Information");
        t.insert((L, TranslationKey::AlertSuccess), "Success");
        t.insert((L, TranslationKey::AlertWarning), "Warning");
        t.insert((L, TranslationKey::AlertError), "Error");
        t.insert(
            (L, TranslationKey::AlertInfoMessage),
            "This is an informational message.",
        );
        t.insert(
            (L, TranslationKey::AlertSuccessMessage),
            "Your changes have been saved successfully.",
        );
        t.insert(
            (L, TranslationKey::AlertWarningMessage),
            "Please review your settings before continuing.",
        );
        t.insert(
            (L, TranslationKey::AlertErrorMessage),
            "An error occurred while processing your request.",
        );

        // Accordion
        t.insert(
            (L, TranslationKey::AccordionGettingStarted),
            "Getting Started",
        );
        t.insert((L, TranslationKey::AccordionFeatures), "Features");
        t.insert((L, TranslationKey::AccordionConfiguration), "Configuration");
    }

    pub(super) fn add_french(t: &mut HashMap<(Language, TranslationKey), &'static str>) {
        use Language::French as L;

        t.insert((L, TranslationKey::AppTitle), "Vitrine du UI Kit GPUI");
        t.insert(
            (L, TranslationKey::AppSubtitle),
            "Une bibliotheque complete de composants UI reutilisables pour les applications GPUI.",
        );

        // Menu
        t.insert((L, TranslationKey::MenuFile), "Fichier");
        t.insert((L, TranslationKey::MenuEdit), "Edition");
        t.insert((L, TranslationKey::MenuView), "Affichage");
        t.insert((L, TranslationKey::MenuHelp), "Aide");
        t.insert((L, TranslationKey::MenuQuit), "Quitter");
        t.insert((L, TranslationKey::MenuTheme), "Theme");
        t.insert((L, TranslationKey::MenuLanguage), "Langue");
        t.insert((L, TranslationKey::MenuSettings), "Parametres");

        // Theme
        t.insert((L, TranslationKey::ThemeDark), "Sombre");
        t.insert((L, TranslationKey::ThemeLight), "Clair");

        // Sections
        t.insert((L, TranslationKey::SectionButtons), "Boutons");
        t.insert((L, TranslationKey::SectionTypography), "Typographie");
        t.insert((L, TranslationKey::SectionBadges), "Badges");
        t.insert((L, TranslationKey::SectionAvatars), "Avatars");
        t.insert(
            (L, TranslationKey::SectionFormControls),
            "Controles de formulaire",
        );
        t.insert(
            (L, TranslationKey::SectionProgress),
            "Indicateurs de progression",
        );
        t.insert((L, TranslationKey::SectionAlerts), "Alertes");
        t.insert((L, TranslationKey::SectionTabs), "Onglets");
        t.insert((L, TranslationKey::SectionCards), "Cartes");
        t.insert((L, TranslationKey::SectionBreadcrumbs), "Fil d'Ariane");
        t.insert(
            (L, TranslationKey::SectionSpinners),
            "Indicateurs de chargement",
        );
        t.insert(
            (L, TranslationKey::SectionLayout),
            "Composants de mise en page",
        );
        t.insert((L, TranslationKey::SectionIconButtons), "Boutons icones");
        t.insert((L, TranslationKey::SectionToasts), "Notifications");
        t.insert((L, TranslationKey::SectionDialogs), "Dialogues");
        t.insert((L, TranslationKey::SectionMenus), "Menus");
        t.insert((L, TranslationKey::SectionTooltips), "Infobulles");
        t.insert((L, TranslationKey::SectionPotentiometers), "Potentiometres");
        t.insert((L, TranslationKey::SectionAccordion), "Accordeon");
        t.insert((L, TranslationKey::SectionQrCode), "Code QR");
        t.insert((L, TranslationKey::SectionContextMenu), "Menu contextuel");
        t.insert((L, TranslationKey::SectionPopover), "Popover");
        t.insert((L, TranslationKey::SectionSidebar), "Barre laterale");
        t.insert((L, TranslationKey::SectionStatusBar), "Barre d'etat");
        t.insert((L, TranslationKey::SectionSearchBar), "Barre de recherche");
        t.insert(
            (L, TranslationKey::SectionKeyboardShortcut),
            "Raccourcis clavier",
        );
        t.insert((L, TranslationKey::SectionEmptyState), "Etat vide");
        t.insert(
            (L, TranslationKey::SectionConfirmDialog),
            "Dialogue de confirmation",
        );
        t.insert((L, TranslationKey::SectionSplitPane), "Panneau divise");
        t.insert((L, TranslationKey::SectionImageView), "Vue image");
        t.insert(
            (L, TranslationKey::SectionSettingsForm),
            "Formulaire de parametres",
        );
        t.insert(
            (L, TranslationKey::SectionStepIndicator),
            "Indicateur d'etapes",
        );
        t.insert(
            (L, TranslationKey::SectionLoadingOverlay),
            "Ecran de chargement",
        );
        t.insert((L, TranslationKey::SectionTag), "Etiquette");
        t.insert((L, TranslationKey::SectionToolbar), "Barre d'outils");
        t.insert((L, TranslationKey::SectionNotification), "Notification");
        t.insert((L, TranslationKey::SectionTreeView), "Vue arborescente");
        t.insert((L, TranslationKey::SectionDragList), "Liste glissable");
        t.insert(
            (L, TranslationKey::SectionCommandPalette),
            "Palette de commandes",
        );
        t.insert((L, TranslationKey::SectionAccessibility), "Accessibilite");
        t.insert(
            (L, TranslationKey::SectionThinkingOrbs),
            "Orbes de réflexion",
        );

        // Labels
        t.insert((L, TranslationKey::LabelVariants), "Variantes");
        t.insert((L, TranslationKey::LabelSizes), "Tailles");
        t.insert((L, TranslationKey::LabelStates), "Etats");
        t.insert((L, TranslationKey::LabelToggles), "Interrupteurs");
        t.insert((L, TranslationKey::LabelCheckboxes), "Cases a cocher");
        t.insert((L, TranslationKey::LabelRadioGroup), "Groupe radio");
        t.insert((L, TranslationKey::LabelCombobox), "Zone de liste");
        t.insert((L, TranslationKey::LabelDatePicker), "Sélecteur de date");
        t.insert((L, TranslationKey::LabelSlider), "Curseur");
        t.insert((L, TranslationKey::LabelInput), "Champ de saisie");
        t.insert((L, TranslationKey::LabelSmall), "Petit");
        t.insert((L, TranslationKey::LabelMedium), "Moyen");
        t.insert((L, TranslationKey::LabelLarge), "Grand");
        t.insert((L, TranslationKey::LabelDisabled), "Desactive");
        t.insert((L, TranslationKey::LabelSelected), "Selectionne");

        // Buttons
        t.insert((L, TranslationKey::ButtonPrimary), "Primaire");
        t.insert((L, TranslationKey::ButtonSecondary), "Secondaire");
        t.insert((L, TranslationKey::ButtonDestructive), "Destructif");
        t.insert((L, TranslationKey::ButtonGhost), "Fantome");
        t.insert((L, TranslationKey::ButtonOutline), "Contour");
        t.insert((L, TranslationKey::ButtonCancel), "Annuler");
        t.insert((L, TranslationKey::ButtonSave), "Enregistrer");
        t.insert((L, TranslationKey::ButtonConfirm), "Confirmer");

        // Dialog
        t.insert(
            (L, TranslationKey::DialogConfirmTitle),
            "Confirmer l'action",
        );
        t.insert(
            (L, TranslationKey::DialogConfirmMessage),
            "Etes-vous sur de vouloir continuer ? Cette action est irreversible.",
        );

        // Alerts
        t.insert((L, TranslationKey::AlertInfo), "Information");
        t.insert((L, TranslationKey::AlertSuccess), "Succes");
        t.insert((L, TranslationKey::AlertWarning), "Avertissement");
        t.insert((L, TranslationKey::AlertError), "Erreur");
        t.insert(
            (L, TranslationKey::AlertInfoMessage),
            "Ceci est un message d'information.",
        );
        t.insert(
            (L, TranslationKey::AlertSuccessMessage),
            "Vos modifications ont ete enregistrees avec succes.",
        );
        t.insert(
            (L, TranslationKey::AlertWarningMessage),
            "Veuillez verifier vos parametres avant de continuer.",
        );
        t.insert(
            (L, TranslationKey::AlertErrorMessage),
            "Une erreur s'est produite lors du traitement de votre demande.",
        );

        // Accordion
        t.insert((L, TranslationKey::AccordionGettingStarted), "Premiers pas");
        t.insert((L, TranslationKey::AccordionFeatures), "Fonctionnalites");
        t.insert((L, TranslationKey::AccordionConfiguration), "Configuration");
    }

    pub(super) fn add_german(t: &mut HashMap<(Language, TranslationKey), &'static str>) {
        use Language::German as L;

        t.insert((L, TranslationKey::AppTitle), "GPUI UI Kit Showcase");
        t.insert(
            (L, TranslationKey::AppSubtitle),
            "Eine umfassende Bibliothek wiederverwendbarer UI-Komponenten fur GPUI-Anwendungen.",
        );

        // Menu
        t.insert((L, TranslationKey::MenuFile), "Datei");
        t.insert((L, TranslationKey::MenuEdit), "Bearbeiten");
        t.insert((L, TranslationKey::MenuView), "Ansicht");
        t.insert((L, TranslationKey::MenuHelp), "Hilfe");
        t.insert((L, TranslationKey::MenuQuit), "Beenden");
        t.insert((L, TranslationKey::MenuTheme), "Thema");
        t.insert((L, TranslationKey::MenuLanguage), "Sprache");
        t.insert((L, TranslationKey::MenuSettings), "Einstellungen");

        // Theme
        t.insert((L, TranslationKey::ThemeDark), "Dunkel");
        t.insert((L, TranslationKey::ThemeLight), "Hell");

        // Sections
        t.insert((L, TranslationKey::SectionButtons), "Schaltflachen");
        t.insert((L, TranslationKey::SectionTypography), "Typografie");
        t.insert((L, TranslationKey::SectionBadges), "Abzeichen");
        t.insert((L, TranslationKey::SectionAvatars), "Avatare");
        t.insert(
            (L, TranslationKey::SectionFormControls),
            "Formularsteuerung",
        );
        t.insert((L, TranslationKey::SectionProgress), "Fortschrittsanzeigen");
        t.insert((L, TranslationKey::SectionAlerts), "Warnungen");
        t.insert((L, TranslationKey::SectionTabs), "Registerkarten");
        t.insert((L, TranslationKey::SectionCards), "Karten");
        t.insert((L, TranslationKey::SectionBreadcrumbs), "Brotkrumen");
        t.insert((L, TranslationKey::SectionSpinners), "Ladeanzeigen");
        t.insert((L, TranslationKey::SectionLayout), "Layout-Komponenten");
        t.insert(
            (L, TranslationKey::SectionIconButtons),
            "Symbol-Schaltflachen",
        );
        t.insert((L, TranslationKey::SectionToasts), "Benachrichtigungen");
        t.insert((L, TranslationKey::SectionDialogs), "Dialoge");
        t.insert((L, TranslationKey::SectionMenus), "Menus");
        t.insert((L, TranslationKey::SectionTooltips), "Tooltips");
        t.insert((L, TranslationKey::SectionPotentiometers), "Potentiometer");
        t.insert((L, TranslationKey::SectionAccordion), "Akkordeon");
        t.insert((L, TranslationKey::SectionQrCode), "QR-Code");
        t.insert((L, TranslationKey::SectionContextMenu), "Kontextmenu");
        t.insert((L, TranslationKey::SectionPopover), "Popover");
        t.insert((L, TranslationKey::SectionSidebar), "Seitenleiste");
        t.insert((L, TranslationKey::SectionStatusBar), "Statusleiste");
        t.insert((L, TranslationKey::SectionSearchBar), "Suchleiste");
        t.insert((L, TranslationKey::SectionKeyboardShortcut), "Tastenkurzel");
        t.insert((L, TranslationKey::SectionEmptyState), "Leerer Zustand");
        t.insert(
            (L, TranslationKey::SectionConfirmDialog),
            "Bestatigungsdialog",
        );
        t.insert((L, TranslationKey::SectionSplitPane), "Geteiltes Fenster");
        t.insert((L, TranslationKey::SectionImageView), "Bildansicht");
        t.insert(
            (L, TranslationKey::SectionSettingsForm),
            "Einstellungsformular",
        );
        t.insert((L, TranslationKey::SectionStepIndicator), "Schrittanzeige");
        t.insert((L, TranslationKey::SectionLoadingOverlay), "Ladebildschirm");
        t.insert((L, TranslationKey::SectionTag), "Schlagwort");
        t.insert((L, TranslationKey::SectionToolbar), "Symbolleiste");
        t.insert((L, TranslationKey::SectionNotification), "Benachrichtigung");
        t.insert((L, TranslationKey::SectionTreeView), "Baumansicht");
        t.insert((L, TranslationKey::SectionDragList), "Zieh-Liste");
        t.insert((L, TranslationKey::SectionCommandPalette), "Befehlspalette");
        t.insert(
            (L, TranslationKey::SectionAccessibility),
            "Barrierefreiheit",
        );
        t.insert((L, TranslationKey::SectionThinkingOrbs), "Denk-Orbs");

        // Labels
        t.insert((L, TranslationKey::LabelVariants), "Varianten");
        t.insert((L, TranslationKey::LabelSizes), "Grossen");
        t.insert((L, TranslationKey::LabelStates), "Zustande");
        t.insert((L, TranslationKey::LabelToggles), "Schalter");
        t.insert((L, TranslationKey::LabelCheckboxes), "Kontrollkastchen");
        t.insert((L, TranslationKey::LabelRadioGroup), "Optionsfeldgruppe");
        t.insert((L, TranslationKey::LabelCombobox), "Kombinationsfeld");
        t.insert((L, TranslationKey::LabelDatePicker), "Datumsauswahl");
        t.insert((L, TranslationKey::LabelSlider), "Schieberegler");
        t.insert((L, TranslationKey::LabelInput), "Eingabe");
        t.insert((L, TranslationKey::LabelSmall), "Klein");
        t.insert((L, TranslationKey::LabelMedium), "Mittel");
        t.insert((L, TranslationKey::LabelLarge), "Gross");
        t.insert((L, TranslationKey::LabelDisabled), "Deaktiviert");
        t.insert((L, TranslationKey::LabelSelected), "Ausgewahlt");

        // Buttons
        t.insert((L, TranslationKey::ButtonPrimary), "Primar");
        t.insert((L, TranslationKey::ButtonSecondary), "Sekundar");
        t.insert((L, TranslationKey::ButtonDestructive), "Destruktiv");
        t.insert((L, TranslationKey::ButtonGhost), "Geist");
        t.insert((L, TranslationKey::ButtonOutline), "Umriss");
        t.insert((L, TranslationKey::ButtonCancel), "Abbrechen");
        t.insert((L, TranslationKey::ButtonSave), "Speichern");
        t.insert((L, TranslationKey::ButtonConfirm), "Bestatigen");

        // Dialog
        t.insert((L, TranslationKey::DialogConfirmTitle), "Aktion bestatigen");
        t.insert(
            (L, TranslationKey::DialogConfirmMessage),
            "Sind Sie sicher, dass Sie fortfahren mochten? Diese Aktion kann nicht ruckgangig gemacht werden.",
        );

        // Alerts
        t.insert((L, TranslationKey::AlertInfo), "Information");
        t.insert((L, TranslationKey::AlertSuccess), "Erfolg");
        t.insert((L, TranslationKey::AlertWarning), "Warnung");
        t.insert((L, TranslationKey::AlertError), "Fehler");
        t.insert(
            (L, TranslationKey::AlertInfoMessage),
            "Dies ist eine Informationsmeldung.",
        );
        t.insert(
            (L, TranslationKey::AlertSuccessMessage),
            "Ihre Anderungen wurden erfolgreich gespeichert.",
        );
        t.insert(
            (L, TranslationKey::AlertWarningMessage),
            "Bitte uberprufen Sie Ihre Einstellungen, bevor Sie fortfahren.",
        );
        t.insert(
            (L, TranslationKey::AlertErrorMessage),
            "Bei der Verarbeitung Ihrer Anfrage ist ein Fehler aufgetreten.",
        );

        // Accordion
        t.insert(
            (L, TranslationKey::AccordionGettingStarted),
            "Erste Schritte",
        );
        t.insert((L, TranslationKey::AccordionFeatures), "Funktionen");
        t.insert((L, TranslationKey::AccordionConfiguration), "Konfiguration");
    }

    pub(super) fn add_spanish(t: &mut HashMap<(Language, TranslationKey), &'static str>) {
        use Language::Spanish as L;

        t.insert((L, TranslationKey::AppTitle), "Galeria del UI Kit GPUI");
        t.insert(
            (L, TranslationKey::AppSubtitle),
            "Una biblioteca completa de componentes UI reutilizables para aplicaciones GPUI.",
        );

        // Menu
        t.insert((L, TranslationKey::MenuFile), "Archivo");
        t.insert((L, TranslationKey::MenuEdit), "Editar");
        t.insert((L, TranslationKey::MenuView), "Ver");
        t.insert((L, TranslationKey::MenuHelp), "Ayuda");
        t.insert((L, TranslationKey::MenuQuit), "Salir");
        t.insert((L, TranslationKey::MenuTheme), "Tema");
        t.insert((L, TranslationKey::MenuLanguage), "Idioma");
        t.insert((L, TranslationKey::MenuSettings), "Configuracion");

        // Theme
        t.insert((L, TranslationKey::ThemeDark), "Oscuro");
        t.insert((L, TranslationKey::ThemeLight), "Claro");

        // Sections
        t.insert((L, TranslationKey::SectionButtons), "Botones");
        t.insert((L, TranslationKey::SectionTypography), "Tipografia");
        t.insert((L, TranslationKey::SectionBadges), "Insignias");
        t.insert((L, TranslationKey::SectionAvatars), "Avatares");
        t.insert(
            (L, TranslationKey::SectionFormControls),
            "Controles de formulario",
        );
        t.insert(
            (L, TranslationKey::SectionProgress),
            "Indicadores de progreso",
        );
        t.insert((L, TranslationKey::SectionAlerts), "Alertas");
        t.insert((L, TranslationKey::SectionTabs), "Pestanas");
        t.insert((L, TranslationKey::SectionCards), "Tarjetas");
        t.insert((L, TranslationKey::SectionBreadcrumbs), "Migas de pan");
        t.insert((L, TranslationKey::SectionSpinners), "Indicadores de carga");
        t.insert((L, TranslationKey::SectionLayout), "Componentes de diseno");
        t.insert((L, TranslationKey::SectionIconButtons), "Botones de icono");
        t.insert((L, TranslationKey::SectionToasts), "Notificaciones");
        t.insert((L, TranslationKey::SectionDialogs), "Dialogos");
        t.insert((L, TranslationKey::SectionMenus), "Menus");
        t.insert((L, TranslationKey::SectionTooltips), "Sugerencias");
        t.insert((L, TranslationKey::SectionPotentiometers), "Potenciometros");
        t.insert((L, TranslationKey::SectionAccordion), "Acordeon");
        t.insert((L, TranslationKey::SectionQrCode), "Codigo QR");
        t.insert((L, TranslationKey::SectionContextMenu), "Menu contextual");
        t.insert((L, TranslationKey::SectionPopover), "Popover");
        t.insert((L, TranslationKey::SectionSidebar), "Barra lateral");
        t.insert((L, TranslationKey::SectionStatusBar), "Barra de estado");
        t.insert((L, TranslationKey::SectionSearchBar), "Barra de busqueda");
        t.insert(
            (L, TranslationKey::SectionKeyboardShortcut),
            "Atajos de teclado",
        );
        t.insert((L, TranslationKey::SectionEmptyState), "Estado vacio");
        t.insert(
            (L, TranslationKey::SectionConfirmDialog),
            "Dialogo de confirmacion",
        );
        t.insert((L, TranslationKey::SectionSplitPane), "Panel dividido");
        t.insert((L, TranslationKey::SectionImageView), "Vista de imagen");
        t.insert(
            (L, TranslationKey::SectionSettingsForm),
            "Formulario de ajustes",
        );
        t.insert(
            (L, TranslationKey::SectionStepIndicator),
            "Indicador de pasos",
        );
        t.insert(
            (L, TranslationKey::SectionLoadingOverlay),
            "Pantalla de carga",
        );
        t.insert((L, TranslationKey::SectionTag), "Etiqueta");
        t.insert((L, TranslationKey::SectionToolbar), "Barra de herramientas");
        t.insert((L, TranslationKey::SectionNotification), "Notificacion");
        t.insert((L, TranslationKey::SectionTreeView), "Vista de arbol");
        t.insert((L, TranslationKey::SectionDragList), "Lista arrastrable");
        t.insert(
            (L, TranslationKey::SectionCommandPalette),
            "Paleta de comandos",
        );
        t.insert((L, TranslationKey::SectionAccessibility), "Accesibilidad");
        t.insert(
            (L, TranslationKey::SectionThinkingOrbs),
            "Orbes de pensamiento",
        );

        // Labels
        t.insert((L, TranslationKey::LabelVariants), "Variantes");
        t.insert((L, TranslationKey::LabelSizes), "Tamanos");
        t.insert((L, TranslationKey::LabelStates), "Estados");
        t.insert((L, TranslationKey::LabelToggles), "Interruptores");
        t.insert((L, TranslationKey::LabelCheckboxes), "Casillas");
        t.insert((L, TranslationKey::LabelRadioGroup), "Grupo de opciones");
        t.insert((L, TranslationKey::LabelCombobox), "Cuadro combinado");
        t.insert((L, TranslationKey::LabelDatePicker), "Selector de fecha");
        t.insert((L, TranslationKey::LabelSlider), "Deslizador");
        t.insert((L, TranslationKey::LabelInput), "Entrada");
        t.insert((L, TranslationKey::LabelSmall), "Pequeno");
        t.insert((L, TranslationKey::LabelMedium), "Mediano");
        t.insert((L, TranslationKey::LabelLarge), "Grande");
        t.insert((L, TranslationKey::LabelDisabled), "Desactivado");
        t.insert((L, TranslationKey::LabelSelected), "Seleccionado");

        // Buttons
        t.insert((L, TranslationKey::ButtonPrimary), "Primario");
        t.insert((L, TranslationKey::ButtonSecondary), "Secundario");
        t.insert((L, TranslationKey::ButtonDestructive), "Destructivo");
        t.insert((L, TranslationKey::ButtonGhost), "Fantasma");
        t.insert((L, TranslationKey::ButtonOutline), "Contorno");
        t.insert((L, TranslationKey::ButtonCancel), "Cancelar");
        t.insert((L, TranslationKey::ButtonSave), "Guardar");
        t.insert((L, TranslationKey::ButtonConfirm), "Confirmar");

        // Dialog
        t.insert((L, TranslationKey::DialogConfirmTitle), "Confirmar accion");
        t.insert(
            (L, TranslationKey::DialogConfirmMessage),
            "Esta seguro de que desea continuar? Esta accion no se puede deshacer.",
        );

        // Alerts
        t.insert((L, TranslationKey::AlertInfo), "Informacion");
        t.insert((L, TranslationKey::AlertSuccess), "Exito");
        t.insert((L, TranslationKey::AlertWarning), "Advertencia");
        t.insert((L, TranslationKey::AlertError), "Error");
        t.insert(
            (L, TranslationKey::AlertInfoMessage),
            "Este es un mensaje informativo.",
        );
        t.insert(
            (L, TranslationKey::AlertSuccessMessage),
            "Sus cambios se han guardado correctamente.",
        );
        t.insert(
            (L, TranslationKey::AlertWarningMessage),
            "Por favor revise su configuracion antes de continuar.",
        );
        t.insert(
            (L, TranslationKey::AlertErrorMessage),
            "Se produjo un error al procesar su solicitud.",
        );

        // Accordion
        t.insert(
            (L, TranslationKey::AccordionGettingStarted),
            "Primeros pasos",
        );
        t.insert((L, TranslationKey::AccordionFeatures), "Caracteristicas");
        t.insert((L, TranslationKey::AccordionConfiguration), "Configuracion");
    }

    pub(super) fn add_japanese(t: &mut HashMap<(Language, TranslationKey), &'static str>) {
        use Language::Japanese as L;

        t.insert((L, TranslationKey::AppTitle), "GPUI UI Kit Showcase");
        t.insert(
            (L, TranslationKey::AppSubtitle),
            "GPUIアプリケーション用の再利用可能なUIコンポーネントの包括的なライブラリ。",
        );

        // Menu
        t.insert((L, TranslationKey::MenuFile), "ファイル");
        t.insert((L, TranslationKey::MenuEdit), "編集");
        t.insert((L, TranslationKey::MenuView), "表示");
        t.insert((L, TranslationKey::MenuHelp), "ヘルプ");
        t.insert((L, TranslationKey::MenuQuit), "終了");
        t.insert((L, TranslationKey::MenuTheme), "テーマ");
        t.insert((L, TranslationKey::MenuLanguage), "言語");
        t.insert((L, TranslationKey::MenuSettings), "設定");

        // Theme
        t.insert((L, TranslationKey::ThemeDark), "ダーク");
        t.insert((L, TranslationKey::ThemeLight), "ライト");

        // Sections
        t.insert((L, TranslationKey::SectionButtons), "ボタン");
        t.insert((L, TranslationKey::SectionTypography), "タイポグラフィ");
        t.insert((L, TranslationKey::SectionBadges), "バッジ");
        t.insert((L, TranslationKey::SectionAvatars), "アバター");
        t.insert(
            (L, TranslationKey::SectionFormControls),
            "フォームコントロール",
        );
        t.insert((L, TranslationKey::SectionProgress), "進捗インジケーター");
        t.insert((L, TranslationKey::SectionAlerts), "アラート");
        t.insert((L, TranslationKey::SectionTabs), "タブ");
        t.insert((L, TranslationKey::SectionCards), "カード");
        t.insert((L, TranslationKey::SectionBreadcrumbs), "パンくずリスト");
        t.insert(
            (L, TranslationKey::SectionSpinners),
            "読み込みインジケーター",
        );
        t.insert(
            (L, TranslationKey::SectionLayout),
            "レイアウトコンポーネント",
        );
        t.insert((L, TranslationKey::SectionIconButtons), "アイコンボタン");
        t.insert((L, TranslationKey::SectionToasts), "トースト");
        t.insert((L, TranslationKey::SectionDialogs), "ダイアログ");
        t.insert((L, TranslationKey::SectionMenus), "メニュー");
        t.insert((L, TranslationKey::SectionTooltips), "ツールチップ");
        t.insert(
            (L, TranslationKey::SectionPotentiometers),
            "ポテンショメーター",
        );
        t.insert((L, TranslationKey::SectionAccordion), "アコーディオン");
        t.insert((L, TranslationKey::SectionQrCode), "QRコード");
        t.insert(
            (L, TranslationKey::SectionContextMenu),
            "コンテキストメニュー",
        );
        t.insert((L, TranslationKey::SectionPopover), "ポップオーバー");
        t.insert((L, TranslationKey::SectionSidebar), "サイドバー");
        t.insert((L, TranslationKey::SectionStatusBar), "ステータスバー");
        t.insert((L, TranslationKey::SectionSearchBar), "検索バー");
        t.insert(
            (L, TranslationKey::SectionKeyboardShortcut),
            "キーボードショートカット",
        );
        t.insert((L, TranslationKey::SectionEmptyState), "空の状態");
        t.insert((L, TranslationKey::SectionConfirmDialog), "確認ダイアログ");
        t.insert((L, TranslationKey::SectionSplitPane), "分割パネル");
        t.insert((L, TranslationKey::SectionImageView), "画像ビュー");
        t.insert((L, TranslationKey::SectionSettingsForm), "設定フォーム");
        t.insert(
            (L, TranslationKey::SectionStepIndicator),
            "ステップインジケーター",
        );
        t.insert(
            (L, TranslationKey::SectionLoadingOverlay),
            "ローディングオーバーレイ",
        );
        t.insert((L, TranslationKey::SectionTag), "タグ");
        t.insert((L, TranslationKey::SectionToolbar), "ツールバー");
        t.insert((L, TranslationKey::SectionNotification), "通知");
        t.insert((L, TranslationKey::SectionTreeView), "ツリービュー");
        t.insert((L, TranslationKey::SectionDragList), "ドラッグリスト");
        t.insert(
            (L, TranslationKey::SectionCommandPalette),
            "コマンドパレット",
        );
        t.insert(
            (L, TranslationKey::SectionAccessibility),
            "アクセシビリティ",
        );
        t.insert((L, TranslationKey::SectionThinkingOrbs), "思考オーブ");

        // Labels
        t.insert((L, TranslationKey::LabelVariants), "バリエーション");
        t.insert((L, TranslationKey::LabelSizes), "サイズ");
        t.insert((L, TranslationKey::LabelStates), "状態");
        t.insert((L, TranslationKey::LabelToggles), "トグル");
        t.insert((L, TranslationKey::LabelCheckboxes), "チェックボックス");
        t.insert((L, TranslationKey::LabelRadioGroup), "ラジオグループ");
        t.insert((L, TranslationKey::LabelCombobox), "コンボボックス");
        t.insert((L, TranslationKey::LabelDatePicker), "日付ピッカー");
        t.insert((L, TranslationKey::LabelSlider), "スライダー");
        t.insert((L, TranslationKey::LabelInput), "入力");
        t.insert((L, TranslationKey::LabelSmall), "小");
        t.insert((L, TranslationKey::LabelMedium), "中");
        t.insert((L, TranslationKey::LabelLarge), "大");
        t.insert((L, TranslationKey::LabelDisabled), "無効");
        t.insert((L, TranslationKey::LabelSelected), "選択済み");

        // Buttons
        t.insert((L, TranslationKey::ButtonPrimary), "プライマリ");
        t.insert((L, TranslationKey::ButtonSecondary), "セカンダリ");
        t.insert((L, TranslationKey::ButtonDestructive), "破壊的");
        t.insert((L, TranslationKey::ButtonGhost), "ゴースト");
        t.insert((L, TranslationKey::ButtonOutline), "アウトライン");
        t.insert((L, TranslationKey::ButtonCancel), "キャンセル");
        t.insert((L, TranslationKey::ButtonSave), "保存");
        t.insert((L, TranslationKey::ButtonConfirm), "確認");

        // Dialog
        t.insert((L, TranslationKey::DialogConfirmTitle), "アクションの確認");
        t.insert(
            (L, TranslationKey::DialogConfirmMessage),
            "続行してもよろしいですか？この操作は元に戻せません。",
        );

        // Alerts
        t.insert((L, TranslationKey::AlertInfo), "情報");
        t.insert((L, TranslationKey::AlertSuccess), "成功");
        t.insert((L, TranslationKey::AlertWarning), "警告");
        t.insert((L, TranslationKey::AlertError), "エラー");
        t.insert(
            (L, TranslationKey::AlertInfoMessage),
            "これは情報メッセージです。",
        );
        t.insert(
            (L, TranslationKey::AlertSuccessMessage),
            "変更が正常に保存されました。",
        );
        t.insert(
            (L, TranslationKey::AlertWarningMessage),
            "続行する前に設定を確認してください。",
        );
        t.insert(
            (L, TranslationKey::AlertErrorMessage),
            "リクエストの処理中にエラーが発生しました。",
        );

        // Accordion
        t.insert((L, TranslationKey::AccordionGettingStarted), "はじめに");
        t.insert((L, TranslationKey::AccordionFeatures), "機能");
        t.insert((L, TranslationKey::AccordionConfiguration), "設定");
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
