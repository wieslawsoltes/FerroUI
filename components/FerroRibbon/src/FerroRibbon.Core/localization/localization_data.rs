pub(super) const CULTURES: &[(&str, &[(&str, &str)])] = &[
    (
        "en",
        &[
            ("File", "File"),
            ("CollapseRibbon", "Collapse the Ribbon"),
            ("PinRibbon", "Pin the Ribbon"),
            ("RibbonDisplayOptions", "Ribbon display options"),
            ("FullScreenMode", "Full-screen mode"),
            ("ShowTabsOnly", "Show tabs only"),
            ("AlwaysShowRibbon", "Always show Ribbon"),
            ("ShowPanelButtons", "Show panel buttons"),
            ("ShowPanelTitles", "Show panel titles"),
            ("MinimizeToTabs", "Minimize to Tabs"),
            ("MinimizeToPanelTitles", "Minimize to Panel Titles"),
            ("MinimizeToPanelButtons", "Minimize to Panel Buttons"),
            ("CycleThroughAll", "Cycle through All"),
            ("MinimizeRibbon", "Minimize the Ribbon"),
            ("ShowFullRibbon", "Show Full Ribbon"),
            ("ShowTabs", "Show Tabs"),
            ("ShowPanels", "Show Panels"),
            ("ShowGroupTitles", "Show Panel Titles"),
            ("FloatPanel", "Float Panel"),
            ("ReturnPanelToRibbon", "Return Panel to Ribbon"),
            ("ReturnPanelsToRibbon", "Return Panels to Ribbon"),
            ("PinPanel", "Keep panel open"),
            ("UnpinPanel", "Unpin panel"),
            ("ExpandPanel", "More {0} commands"),
            ("RecentDocuments", "Recent Documents"),
            ("SearchCommands", "Search commands"),
            ("UseSimplifiedRibbon", "Simplified Ribbon"),
            ("UseClassicRibbon", "Classic Ribbon"),
            ("ShowQuickAccessToolbar", "Show Quick Access Toolbar"),
            ("HideQuickAccessToolbar", "Hide Quick Access Toolbar"),
            ("ShowAboveRibbon", "Show Above the Ribbon"),
            ("ShowBelowRibbon", "Show Below the Ribbon"),
            ("ShowCommandLabels", "Show Command Labels"),
            (
                "CustomizeQuickAccessToolbar",
                "Customize Quick Access Toolbar",
            ),
            ("MoreCommands", "More Commands..."),
            ("AddToQuickAccessToolbar", "Add to Quick Access Toolbar"),
            (
                "RemoveFromQuickAccessToolbar",
                "Remove from Quick Access Toolbar",
            ),
            ("CustomizeRibbon", "Customize the Ribbon..."),
            ("MoreOptions", "More options"),
            ("SplitButtonOptions", "{0} options"),
            ("ZoomIn", "Zoom in"),
            ("ZoomOut", "Zoom out"),
            ("Zoom", "Zoom"),
            (
                "InvalidImport",
                "The text is not a valid ribbon customization (JSON). Nothing was imported.",
            ),
            ("Search", "Search"),
            ("SearchPlaceholder", "Search (Alt+Q)"),
            ("SearchNoResults", "No results"),
            ("SearchRecent", "Recently used"),
            ("SearchActions", "Actions"),
            ("Back", "Back"),
            ("Automatic", "Automatic"),
            ("NoColor", "No Color"),
            ("MoreColors", "More Colors..."),
            ("ThemeColors", "Theme Colors"),
            ("StandardColors", "Standard Colors"),
            ("RecentColors", "Recent Colors"),
            ("ScrollLeft", "Scroll left"),
            ("ScrollRight", "Scroll right"),
            ("GalleryUp", "Row up"),
            ("GalleryDown", "Row down"),
            ("GalleryMore", "More"),
            ("GalleryFilter", "Filter"),
            ("DialogLauncher", "{0} Settings"),
            ("TablePickerFormat", "{0}x{1} Table"),
            ("InsertTable", "Insert Table"),
            ("Ok", "OK"),
            ("Cancel", "Cancel"),
            ("Reset", "Reset"),
            ("NewTab", "New Tab"),
            ("NewGroup", "New Group"),
            ("Rename", "Rename..."),
            ("Add", "Add >>"),
            ("Remove", "<< Remove"),
            ("MoveUp", "Move Up"),
            ("MoveDown", "Move Down"),
            ("ImportExport", "Import/Export"),
            ("ChooseCommands", "Choose commands"),
            ("MainTabs", "Main Tabs"),
            ("QuickAccessToolbar", "Quick Access Toolbar"),
            ("CustomGroupSuffix", "(Custom)"),
            (
                "KeyTipsHint",
                "Press a key to choose a command. Press Esc to go back.",
            ),
            ("Close", "Close"),
            ("Account", "Account"),
            ("Options", "Options"),
            ("CommandPalette", "Type a command"),
        ],
    ),
    (
        "de",
        &[
            ("File", "Datei"),
            ("CollapseRibbon", "Menüband reduzieren"),
            ("PinRibbon", "Menüband fixieren"),
            ("RibbonDisplayOptions", "Menüband-Anzeigeoptionen"),
            ("FullScreenMode", "Vollbildmodus"),
            ("ShowTabsOnly", "Nur Registerkarten anzeigen"),
            ("AlwaysShowRibbon", "Menüband immer anzeigen"),
            ("ShowPanelButtons", "Gruppenschaltflächen anzeigen"),
            ("ShowPanelTitles", "Gruppentitel anzeigen"),
            ("MinimizeToTabs", "Auf Registerkarten minimieren"),
            ("MinimizeToPanelTitles", "Auf Gruppentitel minimieren"),
            (
                "MinimizeToPanelButtons",
                "Auf Gruppenschaltflächen minimieren",
            ),
            ("CycleThroughAll", "Alle durchlaufen"),
            ("MinimizeRibbon", "Menüband minimieren"),
            ("ShowFullRibbon", "Vollständiges Menüband anzeigen"),
            ("ShowTabs", "Registerkarten anzeigen"),
            ("ShowPanels", "Gruppen anzeigen"),
            ("ShowGroupTitles", "Gruppentitel anzeigen"),
            ("FloatPanel", "Gruppe lösen"),
            ("ReturnPanelToRibbon", "Gruppe an Menüband zurückgeben"),
            ("ReturnPanelsToRibbon", "Gruppen an Menüband zurückgeben"),
            ("PinPanel", "Gruppe geöffnet lassen"),
            ("UnpinPanel", "Gruppe lösen"),
            ("ExpandPanel", "Weitere Befehle: {0}"),
            ("RecentDocuments", "Zuletzt verwendete Dokumente"),
            ("SearchCommands", "Befehle suchen"),
            ("UseSimplifiedRibbon", "Vereinfachtes Menüband"),
            ("UseClassicRibbon", "Klassisches Menüband"),
            (
                "ShowQuickAccessToolbar",
                "Symbolleiste für den Schnellzugriff anzeigen",
            ),
            (
                "HideQuickAccessToolbar",
                "Symbolleiste für den Schnellzugriff ausblenden",
            ),
            ("ShowAboveRibbon", "Über dem Menüband anzeigen"),
            ("ShowBelowRibbon", "Unter dem Menüband anzeigen"),
            ("ShowCommandLabels", "Befehlsbeschriftungen anzeigen"),
            (
                "CustomizeQuickAccessToolbar",
                "Symbolleiste für den Schnellzugriff anpassen",
            ),
            ("MoreCommands", "Weitere Befehle..."),
            (
                "AddToQuickAccessToolbar",
                "Zu Symbolleiste für den Schnellzugriff hinzufügen",
            ),
            (
                "RemoveFromQuickAccessToolbar",
                "Aus Symbolleiste für den Schnellzugriff entfernen",
            ),
            ("CustomizeRibbon", "Menüband anpassen..."),
            ("MoreOptions", "Weitere Optionen"),
            ("SplitButtonOptions", "{0} – Optionen"),
            ("ZoomIn", "Vergrößern"),
            ("ZoomOut", "Verkleinern"),
            ("Zoom", "Zoom"),
            (
                "InvalidImport",
                "Der Text ist keine gültige Menüband-Anpassung (JSON). Es wurde nichts importiert.",
            ),
            ("Search", "Suchen"),
            ("SearchPlaceholder", "Suchen (Alt+Q)"),
            ("SearchNoResults", "Keine Ergebnisse"),
            ("SearchRecent", "Zuletzt verwendet"),
            ("SearchActions", "Aktionen"),
            ("Back", "Zurück"),
            ("Automatic", "Automatisch"),
            ("NoColor", "Keine Farbe"),
            ("MoreColors", "Weitere Farben..."),
            ("ThemeColors", "Designfarben"),
            ("StandardColors", "Standardfarben"),
            ("RecentColors", "Zuletzt verwendete Farben"),
            ("ScrollLeft", "Nach links scrollen"),
            ("ScrollRight", "Nach rechts scrollen"),
            ("GalleryUp", "Zeile nach oben"),
            ("GalleryDown", "Zeile nach unten"),
            ("GalleryMore", "Mehr"),
            ("GalleryFilter", "Filtern"),
            ("DialogLauncher", "{0}-Einstellungen"),
            ("TablePickerFormat", "{0}x{1}-Tabelle"),
            ("InsertTable", "Tabelle einfügen"),
            ("Ok", "OK"),
            ("Cancel", "Abbrechen"),
            ("Reset", "Zurücksetzen"),
            ("NewTab", "Neue Registerkarte"),
            ("NewGroup", "Neue Gruppe"),
            ("Rename", "Umbenennen..."),
            ("Add", "Hinzufügen >>"),
            ("Remove", "<< Entfernen"),
            ("MoveUp", "Nach oben"),
            ("MoveDown", "Nach unten"),
            ("ImportExport", "Importieren/Exportieren"),
            ("ChooseCommands", "Befehle auswählen"),
            ("MainTabs", "Hauptregisterkarten"),
            ("QuickAccessToolbar", "Symbolleiste für den Schnellzugriff"),
            ("CustomGroupSuffix", "(Benutzerdefiniert)"),
            (
                "KeyTipsHint",
                "Drücken Sie eine Taste, um einen Befehl auszuwählen. Esc für zurück.",
            ),
            ("Close", "Schließen"),
            ("Account", "Konto"),
            ("Options", "Optionen"),
            ("CommandPalette", "Befehl eingeben"),
        ],
    ),
    (
        "fr",
        &[
            ("File", "Fichier"),
            ("CollapseRibbon", "Réduire le ruban"),
            ("PinRibbon", "Épingler le ruban"),
            ("RibbonDisplayOptions", "Options d'affichage du ruban"),
            ("FullScreenMode", "Mode plein écran"),
            ("ShowTabsOnly", "Afficher uniquement les onglets"),
            ("AlwaysShowRibbon", "Toujours afficher le ruban"),
            ("ShowPanelButtons", "Afficher les boutons des groupes"),
            ("ShowPanelTitles", "Afficher les titres des groupes"),
            ("MinimizeToTabs", "Réduire aux onglets"),
            ("MinimizeToPanelTitles", "Réduire aux titres des groupes"),
            ("MinimizeToPanelButtons", "Réduire aux boutons des groupes"),
            ("CycleThroughAll", "Parcourir tous les états"),
            ("MinimizeRibbon", "Réduire le ruban"),
            ("ShowFullRibbon", "Afficher le ruban complet"),
            ("ShowTabs", "Afficher les onglets"),
            ("ShowPanels", "Afficher les groupes"),
            ("ShowGroupTitles", "Afficher les titres des groupes"),
            ("FloatPanel", "Détacher le groupe"),
            ("ReturnPanelToRibbon", "Remettre le groupe dans le ruban"),
            ("ReturnPanelsToRibbon", "Remettre les groupes dans le ruban"),
            ("PinPanel", "Garder le groupe ouvert"),
            ("UnpinPanel", "Libérer le groupe"),
            ("ExpandPanel", "Autres commandes : {0}"),
            ("RecentDocuments", "Documents récents"),
            ("SearchCommands", "Rechercher des commandes"),
            ("UseSimplifiedRibbon", "Ruban simplifié"),
            ("UseClassicRibbon", "Ruban classique"),
            (
                "ShowQuickAccessToolbar",
                "Afficher la barre d'outils Accès rapide",
            ),
            (
                "HideQuickAccessToolbar",
                "Masquer la barre d'outils Accès rapide",
            ),
            ("ShowAboveRibbon", "Afficher au-dessus du ruban"),
            ("ShowBelowRibbon", "Afficher sous le ruban"),
            ("ShowCommandLabels", "Afficher les étiquettes de commande"),
            (
                "CustomizeQuickAccessToolbar",
                "Personnaliser la barre d'outils Accès rapide",
            ),
            ("MoreCommands", "Autres commandes..."),
            (
                "AddToQuickAccessToolbar",
                "Ajouter à la barre d'outils Accès rapide",
            ),
            (
                "RemoveFromQuickAccessToolbar",
                "Supprimer de la barre d'outils Accès rapide",
            ),
            ("CustomizeRibbon", "Personnaliser le ruban..."),
            ("MoreOptions", "Plus d'options"),
            ("SplitButtonOptions", "Options de {0}"),
            ("ZoomIn", "Zoom avant"),
            ("ZoomOut", "Zoom arrière"),
            ("Zoom", "Zoom"),
            (
                "InvalidImport",
                "Le texte n'est pas une personnalisation du ruban valide (JSON). Rien n'a été importé.",
            ),
            ("Search", "Rechercher"),
            ("SearchPlaceholder", "Rechercher (Alt+Q)"),
            ("SearchNoResults", "Aucun résultat"),
            ("SearchRecent", "Récemment utilisées"),
            ("SearchActions", "Actions"),
            ("Back", "Retour"),
            ("Automatic", "Automatique"),
            ("NoColor", "Aucune couleur"),
            ("MoreColors", "Autres couleurs..."),
            ("ThemeColors", "Couleurs du thème"),
            ("StandardColors", "Couleurs standard"),
            ("RecentColors", "Couleurs récentes"),
            ("ScrollLeft", "Défiler vers la gauche"),
            ("ScrollRight", "Défiler vers la droite"),
            ("GalleryUp", "Ligne précédente"),
            ("GalleryDown", "Ligne suivante"),
            ("GalleryMore", "Plus"),
            ("GalleryFilter", "Filtrer"),
            ("DialogLauncher", "Paramètres {0}"),
            ("TablePickerFormat", "Tableau {0}x{1}"),
            ("InsertTable", "Insérer un tableau"),
            ("Ok", "OK"),
            ("Cancel", "Annuler"),
            ("Reset", "Réinitialiser"),
            ("NewTab", "Nouvel onglet"),
            ("NewGroup", "Nouveau groupe"),
            ("Rename", "Renommer..."),
            ("Add", "Ajouter >>"),
            ("Remove", "<< Supprimer"),
            ("MoveUp", "Monter"),
            ("MoveDown", "Descendre"),
            ("ImportExport", "Importer/Exporter"),
            ("ChooseCommands", "Choisir les commandes"),
            ("MainTabs", "Onglets principaux"),
            ("QuickAccessToolbar", "Barre d'outils Accès rapide"),
            ("CustomGroupSuffix", "(Personnalisé)"),
            (
                "KeyTipsHint",
                "Appuyez sur une touche pour choisir une commande. Échap pour revenir.",
            ),
            ("Close", "Fermer"),
            ("Account", "Compte"),
            ("Options", "Options"),
            ("CommandPalette", "Tapez une commande"),
        ],
    ),
    (
        "es",
        &[
            ("File", "Archivo"),
            ("CollapseRibbon", "Contraer la cinta de opciones"),
            ("PinRibbon", "Anclar la cinta de opciones"),
            (
                "RibbonDisplayOptions",
                "Opciones de presentación de la cinta",
            ),
            ("FullScreenMode", "Modo de pantalla completa"),
            ("ShowTabsOnly", "Mostrar solo pestañas"),
            ("AlwaysShowRibbon", "Mostrar siempre la cinta"),
            ("ShowPanelButtons", "Mostrar botones de grupos"),
            ("ShowPanelTitles", "Mostrar títulos de grupos"),
            ("MinimizeToTabs", "Minimizar a pestañas"),
            ("MinimizeToPanelTitles", "Minimizar a títulos de grupos"),
            ("MinimizeToPanelButtons", "Minimizar a botones de grupos"),
            ("CycleThroughAll", "Recorrer todos"),
            ("MinimizeRibbon", "Minimizar la cinta"),
            ("ShowFullRibbon", "Mostrar la cinta completa"),
            ("ShowTabs", "Mostrar pestañas"),
            ("ShowPanels", "Mostrar grupos"),
            ("ShowGroupTitles", "Mostrar títulos de grupos"),
            ("FloatPanel", "Desacoplar grupo"),
            ("ReturnPanelToRibbon", "Devolver grupo a la cinta"),
            ("ReturnPanelsToRibbon", "Devolver grupos a la cinta"),
            ("PinPanel", "Mantener grupo abierto"),
            ("UnpinPanel", "Desanclar grupo"),
            ("ExpandPanel", "Más comandos de {0}"),
            ("RecentDocuments", "Documentos recientes"),
            ("SearchCommands", "Buscar comandos"),
            ("UseSimplifiedRibbon", "Cinta de opciones simplificada"),
            ("UseClassicRibbon", "Cinta de opciones clásica"),
            (
                "ShowQuickAccessToolbar",
                "Mostrar barra de herramientas de acceso rápido",
            ),
            (
                "HideQuickAccessToolbar",
                "Ocultar barra de herramientas de acceso rápido",
            ),
            ("ShowAboveRibbon", "Mostrar encima de la cinta"),
            ("ShowBelowRibbon", "Mostrar debajo de la cinta"),
            ("ShowCommandLabels", "Mostrar etiquetas de comandos"),
            (
                "CustomizeQuickAccessToolbar",
                "Personalizar barra de herramientas de acceso rápido",
            ),
            ("MoreCommands", "Más comandos..."),
            (
                "AddToQuickAccessToolbar",
                "Agregar a la barra de herramientas de acceso rápido",
            ),
            (
                "RemoveFromQuickAccessToolbar",
                "Quitar de la barra de herramientas de acceso rápido",
            ),
            ("CustomizeRibbon", "Personalizar la cinta de opciones..."),
            ("MoreOptions", "Más opciones"),
            ("SplitButtonOptions", "Opciones de {0}"),
            ("ZoomIn", "Acercar"),
            ("ZoomOut", "Alejar"),
            ("Zoom", "Zoom"),
            (
                "InvalidImport",
                "El texto no es una personalización de la cinta válida (JSON). No se importó nada.",
            ),
            ("Search", "Buscar"),
            ("SearchPlaceholder", "Buscar (Alt+Q)"),
            ("SearchNoResults", "No hay resultados"),
            ("SearchRecent", "Usados recientemente"),
            ("SearchActions", "Acciones"),
            ("Back", "Atrás"),
            ("Automatic", "Automático"),
            ("NoColor", "Sin color"),
            ("MoreColors", "Más colores..."),
            ("ThemeColors", "Colores del tema"),
            ("StandardColors", "Colores estándar"),
            ("RecentColors", "Colores recientes"),
            ("ScrollLeft", "Desplazar a la izquierda"),
            ("ScrollRight", "Desplazar a la derecha"),
            ("GalleryUp", "Fila anterior"),
            ("GalleryDown", "Fila siguiente"),
            ("GalleryMore", "Más"),
            ("GalleryFilter", "Filtrar"),
            ("DialogLauncher", "Configuración de {0}"),
            ("TablePickerFormat", "Tabla de {0}x{1}"),
            ("InsertTable", "Insertar tabla"),
            ("Ok", "Aceptar"),
            ("Cancel", "Cancelar"),
            ("Reset", "Restablecer"),
            ("NewTab", "Nueva pestaña"),
            ("NewGroup", "Nuevo grupo"),
            ("Rename", "Cambiar nombre..."),
            ("Add", "Agregar >>"),
            ("Remove", "<< Quitar"),
            ("MoveUp", "Subir"),
            ("MoveDown", "Bajar"),
            ("ImportExport", "Importar/Exportar"),
            ("ChooseCommands", "Comandos disponibles"),
            ("MainTabs", "Pestañas principales"),
            (
                "QuickAccessToolbar",
                "Barra de herramientas de acceso rápido",
            ),
            ("CustomGroupSuffix", "(Personalizado)"),
            (
                "KeyTipsHint",
                "Presione una tecla para elegir un comando. Esc para volver.",
            ),
            ("Close", "Cerrar"),
            ("Account", "Cuenta"),
            ("Options", "Opciones"),
            ("CommandPalette", "Escriba un comando"),
        ],
    ),
    (
        "pl",
        &[
            ("File", "Plik"),
            ("CollapseRibbon", "Zwiń wstążkę"),
            ("PinRibbon", "Przypnij wstążkę"),
            ("RibbonDisplayOptions", "Opcje wyświetlania wstążki"),
            ("FullScreenMode", "Tryb pełnoekranowy"),
            ("ShowTabsOnly", "Pokaż tylko karty"),
            ("AlwaysShowRibbon", "Zawsze pokazuj wstążkę"),
            ("ShowPanelButtons", "Pokaż przyciski grup"),
            ("ShowPanelTitles", "Pokaż tytuły grup"),
            ("MinimizeToTabs", "Minimalizuj do kart"),
            ("MinimizeToPanelTitles", "Minimalizuj do tytułów grup"),
            ("MinimizeToPanelButtons", "Minimalizuj do przycisków grup"),
            ("CycleThroughAll", "Przełączaj wszystkie"),
            ("MinimizeRibbon", "Minimalizuj wstążkę"),
            ("ShowFullRibbon", "Pokaż pełną wstążkę"),
            ("ShowTabs", "Pokaż karty"),
            ("ShowPanels", "Pokaż grupy"),
            ("ShowGroupTitles", "Pokaż tytuły grup"),
            ("FloatPanel", "Odepnij grupę"),
            ("ReturnPanelToRibbon", "Przywróć grupę do wstążki"),
            ("ReturnPanelsToRibbon", "Przywróć grupy do wstążki"),
            ("PinPanel", "Zostaw grupę otwartą"),
            ("UnpinPanel", "Odepnij grupę"),
            ("ExpandPanel", "Więcej poleceń: {0}"),
            ("RecentDocuments", "Ostatnie dokumenty"),
            ("SearchCommands", "Szukaj poleceń"),
            ("UseSimplifiedRibbon", "Uproszczona wstążka"),
            ("UseClassicRibbon", "Klasyczna wstążka"),
            (
                "ShowQuickAccessToolbar",
                "Pokaż pasek narzędzi Szybki dostęp",
            ),
            (
                "HideQuickAccessToolbar",
                "Ukryj pasek narzędzi Szybki dostęp",
            ),
            ("ShowAboveRibbon", "Pokaż nad wstążką"),
            ("ShowBelowRibbon", "Pokaż pod wstążką"),
            ("ShowCommandLabels", "Pokaż etykiety poleceń"),
            (
                "CustomizeQuickAccessToolbar",
                "Dostosuj pasek narzędzi Szybki dostęp",
            ),
            ("MoreCommands", "Więcej poleceń..."),
            (
                "AddToQuickAccessToolbar",
                "Dodaj do paska narzędzi Szybki dostęp",
            ),
            (
                "RemoveFromQuickAccessToolbar",
                "Usuń z paska narzędzi Szybki dostęp",
            ),
            ("CustomizeRibbon", "Dostosuj wstążkę..."),
            ("MoreOptions", "Więcej opcji"),
            ("SplitButtonOptions", "{0} – opcje"),
            ("ZoomIn", "Powiększ"),
            ("ZoomOut", "Pomniejsz"),
            ("Zoom", "Powiększenie"),
            (
                "InvalidImport",
                "Tekst nie jest prawidłowym dostosowaniem wstążki (JSON). Nic nie zaimportowano.",
            ),
            ("Search", "Wyszukaj"),
            ("SearchPlaceholder", "Wyszukaj (Alt+Q)"),
            ("SearchNoResults", "Brak wyników"),
            ("SearchRecent", "Ostatnio używane"),
            ("SearchActions", "Akcje"),
            ("Back", "Wstecz"),
            ("Automatic", "Automatyczny"),
            ("NoColor", "Brak koloru"),
            ("MoreColors", "Więcej kolorów..."),
            ("ThemeColors", "Kolory motywu"),
            ("StandardColors", "Kolory standardowe"),
            ("RecentColors", "Ostatnie kolory"),
            ("ScrollLeft", "Przewiń w lewo"),
            ("ScrollRight", "Przewiń w prawo"),
            ("GalleryUp", "Wiersz w górę"),
            ("GalleryDown", "Wiersz w dół"),
            ("GalleryMore", "Więcej"),
            ("GalleryFilter", "Filtruj"),
            ("DialogLauncher", "Ustawienia: {0}"),
            ("TablePickerFormat", "Tabela {0}x{1}"),
            ("InsertTable", "Wstaw tabelę"),
            ("Ok", "OK"),
            ("Cancel", "Anuluj"),
            ("Reset", "Resetuj"),
            ("NewTab", "Nowa karta"),
            ("NewGroup", "Nowa grupa"),
            ("Rename", "Zmień nazwę..."),
            ("Add", "Dodaj >>"),
            ("Remove", "<< Usuń"),
            ("MoveUp", "Przenieś w górę"),
            ("MoveDown", "Przenieś w dół"),
            ("ImportExport", "Importuj/Eksportuj"),
            ("ChooseCommands", "Wybierz polecenia"),
            ("MainTabs", "Karty główne"),
            ("QuickAccessToolbar", "Pasek narzędzi Szybki dostęp"),
            ("CustomGroupSuffix", "(Niestandardowa)"),
            (
                "KeyTipsHint",
                "Naciśnij klawisz, aby wybrać polecenie. Esc – wstecz.",
            ),
            ("Close", "Zamknij"),
            ("Account", "Konto"),
            ("Options", "Opcje"),
            ("CommandPalette", "Wpisz polecenie"),
        ],
    ),
];
impl super::RibbonStrings {
    pub fn file(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("File")))
            .expect("constant nonnull key")
    }
    pub fn collapse_ribbon(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("CollapseRibbon")))
            .expect("constant nonnull key")
    }
    pub fn pin_ribbon(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("PinRibbon")))
            .expect("constant nonnull key")
    }
    pub fn ribbon_display_options(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("RibbonDisplayOptions")))
            .expect("constant nonnull key")
    }
    pub fn full_screen_mode(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("FullScreenMode")))
            .expect("constant nonnull key")
    }
    pub fn show_tabs_only(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ShowTabsOnly")))
            .expect("constant nonnull key")
    }
    pub fn always_show_ribbon(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("AlwaysShowRibbon")))
            .expect("constant nonnull key")
    }
    pub fn show_panel_buttons(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ShowPanelButtons")))
            .expect("constant nonnull key")
    }
    pub fn show_panel_titles(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ShowPanelTitles")))
            .expect("constant nonnull key")
    }
    pub fn minimize_to_tabs(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("MinimizeToTabs")))
            .expect("constant nonnull key")
    }
    pub fn minimize_to_panel_titles(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("MinimizeToPanelTitles")))
            .expect("constant nonnull key")
    }
    pub fn minimize_to_panel_buttons(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("MinimizeToPanelButtons")))
            .expect("constant nonnull key")
    }
    pub fn cycle_through_all(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("CycleThroughAll")))
            .expect("constant nonnull key")
    }
    pub fn minimize_ribbon(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("MinimizeRibbon")))
            .expect("constant nonnull key")
    }
    pub fn show_full_ribbon(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ShowFullRibbon")))
            .expect("constant nonnull key")
    }
    pub fn show_tabs(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ShowTabs")))
            .expect("constant nonnull key")
    }
    pub fn show_panels(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ShowPanels")))
            .expect("constant nonnull key")
    }
    pub fn show_group_titles(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ShowGroupTitles")))
            .expect("constant nonnull key")
    }
    pub fn float_panel(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("FloatPanel")))
            .expect("constant nonnull key")
    }
    pub fn return_panel_to_ribbon(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ReturnPanelToRibbon")))
            .expect("constant nonnull key")
    }
    pub fn return_panels_to_ribbon(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ReturnPanelsToRibbon")))
            .expect("constant nonnull key")
    }
    pub fn pin_panel(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("PinPanel")))
            .expect("constant nonnull key")
    }
    pub fn unpin_panel(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("UnpinPanel")))
            .expect("constant nonnull key")
    }
    pub fn expand_panel(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ExpandPanel")))
            .expect("constant nonnull key")
    }
    pub fn recent_documents(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("RecentDocuments")))
            .expect("constant nonnull key")
    }
    pub fn search_commands(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("SearchCommands")))
            .expect("constant nonnull key")
    }
    pub fn use_simplified_ribbon(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("UseSimplifiedRibbon")))
            .expect("constant nonnull key")
    }
    pub fn use_classic_ribbon(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("UseClassicRibbon")))
            .expect("constant nonnull key")
    }
    pub fn show_quick_access_toolbar(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ShowQuickAccessToolbar")))
            .expect("constant nonnull key")
    }
    pub fn hide_quick_access_toolbar(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("HideQuickAccessToolbar")))
            .expect("constant nonnull key")
    }
    pub fn show_above_ribbon(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ShowAboveRibbon")))
            .expect("constant nonnull key")
    }
    pub fn show_below_ribbon(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ShowBelowRibbon")))
            .expect("constant nonnull key")
    }
    pub fn show_command_labels(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ShowCommandLabels")))
            .expect("constant nonnull key")
    }
    pub fn customize_quick_access_toolbar(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("CustomizeQuickAccessToolbar")))
            .expect("constant nonnull key")
    }
    pub fn more_commands(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("MoreCommands")))
            .expect("constant nonnull key")
    }
    pub fn add_to_quick_access_toolbar(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("AddToQuickAccessToolbar")))
            .expect("constant nonnull key")
    }
    pub fn remove_from_quick_access_toolbar(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("RemoveFromQuickAccessToolbar")))
            .expect("constant nonnull key")
    }
    pub fn customize_ribbon(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("CustomizeRibbon")))
            .expect("constant nonnull key")
    }
    pub fn more_options(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("MoreOptions")))
            .expect("constant nonnull key")
    }
    pub fn split_button_options(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("SplitButtonOptions")))
            .expect("constant nonnull key")
    }
    pub fn zoom_in(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ZoomIn")))
            .expect("constant nonnull key")
    }
    pub fn zoom_out(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ZoomOut")))
            .expect("constant nonnull key")
    }
    pub fn zoom(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("Zoom")))
            .expect("constant nonnull key")
    }
    pub fn invalid_import(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("InvalidImport")))
            .expect("constant nonnull key")
    }
    pub fn search(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("Search")))
            .expect("constant nonnull key")
    }
    pub fn search_placeholder(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("SearchPlaceholder")))
            .expect("constant nonnull key")
    }
    pub fn search_no_results(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("SearchNoResults")))
            .expect("constant nonnull key")
    }
    pub fn search_recent(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("SearchRecent")))
            .expect("constant nonnull key")
    }
    pub fn search_actions(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("SearchActions")))
            .expect("constant nonnull key")
    }
    pub fn back(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("Back")))
            .expect("constant nonnull key")
    }
    pub fn automatic(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("Automatic")))
            .expect("constant nonnull key")
    }
    pub fn no_color(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("NoColor")))
            .expect("constant nonnull key")
    }
    pub fn more_colors(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("MoreColors")))
            .expect("constant nonnull key")
    }
    pub fn theme_colors(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ThemeColors")))
            .expect("constant nonnull key")
    }
    pub fn standard_colors(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("StandardColors")))
            .expect("constant nonnull key")
    }
    pub fn recent_colors(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("RecentColors")))
            .expect("constant nonnull key")
    }
    pub fn scroll_left(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ScrollLeft")))
            .expect("constant nonnull key")
    }
    pub fn scroll_right(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ScrollRight")))
            .expect("constant nonnull key")
    }
    pub fn gallery_up(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("GalleryUp")))
            .expect("constant nonnull key")
    }
    pub fn gallery_down(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("GalleryDown")))
            .expect("constant nonnull key")
    }
    pub fn gallery_more(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("GalleryMore")))
            .expect("constant nonnull key")
    }
    pub fn gallery_filter(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("GalleryFilter")))
            .expect("constant nonnull key")
    }
    pub fn dialog_launcher(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("DialogLauncher")))
            .expect("constant nonnull key")
    }
    pub fn table_picker_format(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("TablePickerFormat")))
            .expect("constant nonnull key")
    }
    pub fn insert_table(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("InsertTable")))
            .expect("constant nonnull key")
    }
    pub fn ok(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("Ok")))
            .expect("constant nonnull key")
    }
    pub fn cancel(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("Cancel")))
            .expect("constant nonnull key")
    }
    pub fn reset(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("Reset")))
            .expect("constant nonnull key")
    }
    pub fn new_tab(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("NewTab")))
            .expect("constant nonnull key")
    }
    pub fn new_group(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("NewGroup")))
            .expect("constant nonnull key")
    }
    pub fn rename(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("Rename")))
            .expect("constant nonnull key")
    }
    pub fn add(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("Add")))
            .expect("constant nonnull key")
    }
    pub fn remove(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("Remove")))
            .expect("constant nonnull key")
    }
    pub fn move_up(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("MoveUp")))
            .expect("constant nonnull key")
    }
    pub fn move_down(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("MoveDown")))
            .expect("constant nonnull key")
    }
    pub fn import_export(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ImportExport")))
            .expect("constant nonnull key")
    }
    pub fn choose_commands(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("ChooseCommands")))
            .expect("constant nonnull key")
    }
    pub fn main_tabs(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("MainTabs")))
            .expect("constant nonnull key")
    }
    pub fn quick_access_toolbar(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("QuickAccessToolbar")))
            .expect("constant nonnull key")
    }
    pub fn custom_group_suffix(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("CustomGroupSuffix")))
            .expect("constant nonnull key")
    }
    pub fn key_tips_hint(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("KeyTipsHint")))
            .expect("constant nonnull key")
    }
    pub fn close(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("Close")))
            .expect("constant nonnull key")
    }
    pub fn account(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("Account")))
            .expect("constant nonnull key")
    }
    pub fn options(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("Options")))
            .expect("constant nonnull key")
    }
    pub fn command_palette(&self) -> Option<crate::RibbonString> {
        self.get(Some(&super::literal("CommandPalette")))
            .expect("constant nonnull key")
    }
}
#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;
    #[test]
    fn all_source_properties_and_resource_strings_match_unchanged_csharp() {
        let data: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/localization.json"))
                .unwrap();
        for row in data["tables"].as_array().unwrap() {
            let name = row["culture"].as_str().unwrap();
            let culture = RibbonCulture::new(name.into(), name.into());
            let strings = RibbonStrings::for_culture(Some(culture)).unwrap();
            assert_eq!(
                serde_json::json!(strings.file().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("File".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.collapse_ribbon().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("CollapseRibbon".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.pin_ribbon().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("PinRibbon".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.ribbon_display_options().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!(
                            "RibbonDisplayOptions".encode_utf16().collect::<Vec<_>>()
                        ))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.full_screen_mode().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("FullScreenMode".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.show_tabs_only().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("ShowTabsOnly".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.always_show_ribbon().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("AlwaysShowRibbon".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.show_panel_buttons().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("ShowPanelButtons".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.show_panel_titles().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("ShowPanelTitles".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.minimize_to_tabs().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("MinimizeToTabs".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.minimize_to_panel_titles().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!(
                            "MinimizeToPanelTitles".encode_utf16().collect::<Vec<_>>()
                        ))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.minimize_to_panel_buttons().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!(
                            "MinimizeToPanelButtons".encode_utf16().collect::<Vec<_>>()
                        ))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.cycle_through_all().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("CycleThroughAll".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.minimize_ribbon().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("MinimizeRibbon".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.show_full_ribbon().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("ShowFullRibbon".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.show_tabs().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("ShowTabs".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.show_panels().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("ShowPanels".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.show_group_titles().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("ShowGroupTitles".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.float_panel().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("FloatPanel".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.return_panel_to_ribbon().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!(
                            "ReturnPanelToRibbon".encode_utf16().collect::<Vec<_>>()
                        ))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.return_panels_to_ribbon().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!(
                            "ReturnPanelsToRibbon".encode_utf16().collect::<Vec<_>>()
                        ))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.pin_panel().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("PinPanel".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.unpin_panel().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("UnpinPanel".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.expand_panel().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("ExpandPanel".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.recent_documents().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("RecentDocuments".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.search_commands().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("SearchCommands".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.use_simplified_ribbon().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!(
                            "UseSimplifiedRibbon".encode_utf16().collect::<Vec<_>>()
                        ))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.use_classic_ribbon().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("UseClassicRibbon".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.show_quick_access_toolbar().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!(
                            "ShowQuickAccessToolbar".encode_utf16().collect::<Vec<_>>()
                        ))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.hide_quick_access_toolbar().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!(
                            "HideQuickAccessToolbar".encode_utf16().collect::<Vec<_>>()
                        ))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.show_above_ribbon().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("ShowAboveRibbon".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.show_below_ribbon().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("ShowBelowRibbon".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.show_command_labels().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!(
                            "ShowCommandLabels".encode_utf16().collect::<Vec<_>>()
                        ))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.customize_quick_access_toolbar().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!(
                            "CustomizeQuickAccessToolbar"
                                .encode_utf16()
                                .collect::<Vec<_>>()
                        ))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.more_commands().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("MoreCommands".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.add_to_quick_access_toolbar().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!(
                            "AddToQuickAccessToolbar".encode_utf16().collect::<Vec<_>>()
                        ))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(
                    strings
                        .remove_from_quick_access_toolbar()
                        .unwrap()
                        .as_utf16()
                ),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!(
                            "RemoveFromQuickAccessToolbar"
                                .encode_utf16()
                                .collect::<Vec<_>>()
                        ))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.customize_ribbon().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("CustomizeRibbon".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.more_options().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("MoreOptions".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.split_button_options().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!(
                            "SplitButtonOptions".encode_utf16().collect::<Vec<_>>()
                        ))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.zoom_in().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("ZoomIn".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.zoom_out().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("ZoomOut".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.zoom().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("Zoom".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.invalid_import().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("InvalidImport".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.search().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("Search".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.search_placeholder().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!(
                            "SearchPlaceholder".encode_utf16().collect::<Vec<_>>()
                        ))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.search_no_results().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("SearchNoResults".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.search_recent().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("SearchRecent".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.search_actions().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("SearchActions".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.back().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("Back".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.automatic().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("Automatic".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.no_color().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("NoColor".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.more_colors().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("MoreColors".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.theme_colors().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("ThemeColors".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.standard_colors().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("StandardColors".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.recent_colors().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("RecentColors".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.scroll_left().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("ScrollLeft".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.scroll_right().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("ScrollRight".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.gallery_up().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("GalleryUp".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.gallery_down().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("GalleryDown".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.gallery_more().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("GalleryMore".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.gallery_filter().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("GalleryFilter".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.dialog_launcher().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("DialogLauncher".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.table_picker_format().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!(
                            "TablePickerFormat".encode_utf16().collect::<Vec<_>>()
                        ))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.insert_table().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("InsertTable".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(serde_json::json!(strings.ok().unwrap().as_utf16()), row["values"].as_array().unwrap().iter().find(|r| r["key"]==serde_json::json!("Ok".encode_utf16().collect::<Vec<_>>())).unwrap()["value"]);
            assert_eq!(
                serde_json::json!(strings.cancel().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("Cancel".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.reset().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("Reset".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.new_tab().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("NewTab".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.new_group().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("NewGroup".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.rename().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("Rename".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.add().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(
                        |r| r["key"] == serde_json::json!("Add".encode_utf16().collect::<Vec<_>>())
                    )
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.remove().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("Remove".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.move_up().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("MoveUp".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.move_down().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("MoveDown".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.import_export().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("ImportExport".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.choose_commands().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("ChooseCommands".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.main_tabs().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("MainTabs".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.quick_access_toolbar().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!(
                            "QuickAccessToolbar".encode_utf16().collect::<Vec<_>>()
                        ))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.custom_group_suffix().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!(
                            "CustomGroupSuffix".encode_utf16().collect::<Vec<_>>()
                        ))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.key_tips_hint().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("KeyTipsHint".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.close().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("Close".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.account().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("Account".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.options().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("Options".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
            assert_eq!(
                serde_json::json!(strings.command_palette().unwrap().as_utf16()),
                row["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["key"]
                        == serde_json::json!("CommandPalette".encode_utf16().collect::<Vec<_>>()))
                    .unwrap()["value"]
            );
        }
        assert_eq!(CULTURES.len(), 5);
    }
}
