#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Locale {
    #[default]
    #[serde(rename = "en")]
    English,
    #[serde(rename = "es")]
    Spanish,
    #[serde(rename = "pt-BR")]
    PortugueseBrazil,
}

impl Locale {
    pub const ALL: [Self; 3] = [Self::English, Self::Spanish, Self::PortugueseBrazil];

    pub const fn name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Spanish => "Español",
            Self::PortugueseBrazil => "Português (Brasil)",
        }
    }

    pub const fn text(self, text: Text) -> &'static str {
        text.get(self)
    }

    pub fn translate(self, message: &str) -> String {
        let translated = match message {
            "Direcional" => Some(Text::Dpad),
            "Direcional / analógico esquerdo" => Some(Text::DpadOrLeftStick),
            "R3 · pressionar analógico direito" => Some(Text::RightStickPress),
            "L3 · pressionar analógico esquerdo" => Some(Text::LeftStickPress),
            "Analógico esquerdo" => Some(Text::LeftStick),
            "Analógico direito ↕" => Some(Text::RightStickVertical),
            "Inferior · A / ✕ / B no Switch" => Some(Text::SouthButton),
            "Direito · B / ○ / A no Switch" => Some(Text::EastButton),
            "Esquerdo · X / □ / Y no Switch" => Some(Text::WestButton),
            "Superior · Y / △ / X no Switch" => Some(Text::NorthButton),
            "A / botão inferior" => Some(Text::SouthButtonShort),
            "B / botão direito" => Some(Text::EastButtonShort),
            "X / botão esquerdo" => Some(Text::WestButtonShort),
            "Y / botão superior" => Some(Text::NorthButtonShort),
            "Abrir / fechar teclado visual próprio" => Some(Text::ToggleKeyboard),
            "Lançador do sistema · Super / Windows" => Some(Text::SystemLauncher),
            "Mover ponteiro" => Some(Text::MovePointer),
            "Rolar página" => Some(Text::ScrollPage),
            "Clique esquerdo · segurar para arrastar" => Some(Text::LeftClickDrag),
            "Clique direito" => Some(Text::RightClick),
            "Enter · abrir item selecionado" => Some(Text::EnterSelected),
            "Escape · fechar menu / cancelar" => Some(Text::EscapeCancel),
            "Setas · segurar para repetir" => Some(Text::ArrowRepeat),
            "Tab · próximo campo" => Some(Text::NextField),
            "Alt+Tab · alternar janela" => Some(Text::SwitchWindow),
            "Start + Select / Menu + View" => Some(Text::PauseCombo),
            "Pausar / retomar · solte e centralize para retomar" => Some(Text::PauseResume),
            "Select · dois toques rápidos" => Some(Text::DoubleTapSelect),
            "Alternar entre Desktop e Jogo" => Some(Text::DoubleTapSelectAction),
            "Mover a seleção pelo teclado" => Some(Text::MoveKeyboardSelection),
            "Ativar a tecla selecionada" => Some(Text::TypeSelectedKey),
            "Apagar um caractere" => Some(Text::DeleteCharacter),
            "Inserir Enter" => Some(Text::InsertEnter),
            "Inserir Esc" => Some(Text::InsertEscape),
            "Inserir espaço" => Some(Text::InsertSpace),
            "Inserir Tab" => Some(Text::InsertTab),
            "Ativar ou desativar Shift" => Some(Text::ToggleShift),
            "Fechar o teclado visual" => Some(Text::CloseKeyboard),
            "Ajustes do teclado · direcional escolhe e altera" => Some(Text::KeyboardSettingsHelp),
            "Perfil aplicado e salvo para as próximas sessões." => Some(Text::ProfileSaved),
            "Este compositor não permite exibir o teclado próprio sem ativá-lo." => {
                Some(Text::KeyboardOverlayUnavailable)
            }
            "Teclado visual próprio · direcional move · A digita · B apaga · R3 fecha." => {
                Some(Text::KeyboardOpened)
            }
            "Teclado visual fechado." => Some(Text::KeyboardClosed),
            "Cole uma única linha de perfil SDL: GUID,nome,botões/eixos." => {
                Some(Text::PasteOneSdlMapping)
            }
            _ => None,
        };
        if let Some(text) = translated {
            return self.text(text).to_owned();
        }

        for (prefix, text) in [
            ("Erro ao carregar perfil SDL: ", Text::SdlLoadProfilePrefix),
            ("Erro ao ler os perfis: ", Text::SdlReadProfilesPrefix),
            (
                "Perfil aplicado nesta sessão, mas não foi salvo: ",
                Text::ProfileSaveFailedPrefix,
            ),
            (
                "Não foi possível salvar ajustes: ",
                Text::SettingsSaveFailedPrefix,
            ),
            (
                "Falha ao conectar entrada do Windows: ",
                Text::WindowsConnectFailedPrefix,
            ),
            (
                "Falha enviando entrada ao Windows: ",
                Text::WindowsSendFailedPrefix,
            ),
            (
                "Erro enviando entrada ao desktop: ",
                Text::DesktopSendFailedPrefix,
            ),
            ("SDL3: ", Text::SdlErrorPrefix),
        ] {
            if let Some(detail) = message.strip_prefix(prefix) {
                return format!("{}{detail}", self.text(text));
            }
        }
        message.to_owned()
    }

    pub fn detect() -> Self {
        #[cfg(target_os = "windows")]
        if let Some(identifier) = windows_locale() {
            return Self::from_identifier(&identifier);
        }

        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .into_iter()
            .filter_map(|name| std::env::var(name).ok())
            .find(|value| !value.is_empty())
            .map_or(Self::English, |identifier| {
                Self::from_identifier(&identifier)
            })
    }

    fn from_identifier(identifier: &str) -> Self {
        let language = identifier
            .split(['.', '@'])
            .next()
            .unwrap_or(identifier)
            .replace('_', "-")
            .to_ascii_lowercase();
        if language.starts_with("pt") {
            Self::PortugueseBrazil
        } else if language.starts_with("es") {
            Self::Spanish
        } else {
            Self::English
        }
    }
}

#[cfg(target_os = "windows")]
fn windows_locale() -> Option<String> {
    let mut name = [0_u16; 85];
    let length = unsafe { windows::Win32::Globalization::GetUserDefaultLocaleName(&mut name) };
    (length > 0).then(|| String::from_utf16_lossy(&name[..length as usize]))
}

macro_rules! catalog {
    ($($key:ident => ($en:literal, $es:literal, $pt:literal);)+) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Text {
            $($key,)+
        }

        impl Text {
            const fn get(self, locale: Locale) -> &'static str {
                match self {
                    $(Self::$key => match locale {
                        Locale::English => $en,
                        Locale::Spanish => $es,
                        Locale::PortugueseBrazil => $pt,
                    },)+
                }
            }
        }
    };
}

catalog! {
    TourWelcome => ("Welcome to Gamepad Desktop", "Bienvenido a Gamepad Desktop", "Boas-vindas ao Gamepad Desktop");
    TourWelcomeCopy => ("Your controller can also be your mouse and keyboard. Learn the basics in about two minutes, at your own pace.", "Tu mando también puede ser tu ratón y teclado. Aprende lo básico en unos dos minutos, a tu ritmo.", "Seu controle também pode ser mouse e teclado. Aprenda o básico em cerca de dois minutos, no seu ritmo.");
    TourModes => ("One controller, two modes", "Un mando, dos modos", "Um controle, dois modos");
    InterfaceSize => ("Interface size", "Tamaño de la interfaz", "Tamanho da interface");
    InterfaceSizeHelp => ("Use A− / A+ to read comfortably from a distance. Applies throughout the app and is saved automatically.", "Usa A− / A+ para leer a distancia. Se aplica a toda la app y se guarda automáticamente.", "Use A− / A+ para ler confortavelmente à distância. Vale para todo o app e é salvo automaticamente.");
    TourModesCopy => ("Desktop turns your controller into a mouse and keyboard. Game leaves control to your game or the native Windows focus navigation, where supported. Switch modes without disconnecting.", "Escritorio convierte el mando en ratón y teclado. Juego deja el control al juego o a la navegación de foco nativa de Windows, donde sea compatible. Cambia sin desconectar el mando.", "Desktop transforma o controle em mouse e teclado. Jogo deixa os comandos por conta do game ou da navegação de foco nativa do Windows, onde houver suporte. Alterne sem desconectar.");
    TourDesktopCard => ("Move the pointer, click, scroll and type with your controller.", "Mueve el puntero, haz clic, desplázate y escribe con el mando.", "Mova o ponteiro, clique, role e digite com o controle.");
    TourGameCard => ("Use the default controls of your game or native Windows focus navigation, where supported. Gamepad Desktop stops sending mouse and keyboard commands; it does not remap the game.", "Usa los controles predeterminados del juego o el foco nativo de Windows, donde sea compatible. Gamepad Desktop deja de enviar ratón y teclado; no reasigna los controles del juego.", "Use os controles padrão do game ou o foco nativo do Windows, onde houver suporte. Gamepad Desktop para de enviar mouse e teclado; ele não remapeia os botões do jogo.");
    TourPointer => ("Move the pointer", "Mueve el puntero", "Mova o ponteiro");
    TourPointerCopy => ("Move the left stick. Watch the pointer move inside this practice desktop.", "Mueve el stick izquierdo. Mira cómo se mueve el puntero dentro de este escritorio de práctica.", "Mova o analógico esquerdo. Veja o ponteiro se mover dentro deste desktop de prática.");
    TourClick => ("Left mouse button: select", "Botón izquierdo: seleccionar", "Botão esquerdo: selecionar");
    TourClickCopy => ("Press and release the bottom face button (A on Xbox / cross on PlayStation) over the target. This is a left mouse click: use it to select items and activate buttons.", "Pulsa y suelta el botón inferior (A en Xbox / cruz en PlayStation) sobre el objetivo. Es un clic izquierdo: selecciona elementos y activa botones.", "Pressione e solte o botão inferior (A no Xbox / cruz no PlayStation) sobre o alvo. Esse é o clique esquerdo do mouse: seleciona itens e aciona botões.");
    TourRightClick => ("Right mouse button: more options", "Botón derecho: más opciones", "Botão direito: mais opções");
    TourRightClickCopy => ("Press and release the right face button (B on Xbox / circle on PlayStation) over the target. This is a right mouse click: it opens the context menu when the app provides one.", "Pulsa y suelta el botón derecho (B en Xbox / círculo en PlayStation) sobre el objetivo. Es un clic derecho: abre el menú contextual cuando la app lo ofrece.", "Pressione e solte o botão da direita (B no Xbox / círculo no PlayStation) sobre o alvo. Esse é o clique direito do mouse: abre o menu de contexto quando o app oferece um.");
    TourDrag => ("Hold, move, release", "Mantén, mueve y suelta", "Segure, mova e solte");
    TourDragCopy => ("Hold the bottom face button over the card. Keep holding while moving the left stick toward the drop area. Release the button there to drop the card. Use this to drag files or move windows by their title bar.", "Mantén el botón inferior sobre la tarjeta. Sin soltarlo, mueve el stick izquierdo hasta el destino. Suelta el botón allí. Así puedes arrastrar archivos o mover ventanas desde la barra de título.", "Segure o botão inferior sobre o cartão. Sem soltar, mova o analógico esquerdo até o destino. Solte o botão ali para deixar o cartão. Use isso para arrastar arquivos ou mover janelas pela barra de título.");
    TourDropTarget => ("Drop here", "Suelta aquí", "Solte aqui");
    TourDragItem => ("Drag me", "Arrástrame", "Arraste");
    TourContextMenu => ("Context menu", "Menú contextual", "Menu de contexto");
    TourScroll => ("Scroll through a page", "Desplázate por una página", "Role uma página");
    TourScrollCopy => ("Move the right stick up or down, without pressing it, to scroll. Release it to stop. Pressing the stick (R3) opens the keyboard instead.", "Mueve el stick derecho arriba o abajo, sin pulsarlo, para desplazar el contenido. Suéltalo para parar. Pulsarlo (R3) abre el teclado.", "Mova o analógico direito para cima ou para baixo, sem apertá-lo, para rolar. Solte para parar. Apertar o analógico (R3) abre o teclado.");
    TourOpenKeyboard => ("Open the keyboard", "Abre el teclado", "Abra o teclado");
    TourOpenKeyboardCopy => ("Press the right stick like a button: this is R3. It opens the on-screen keyboard in Desktop mode.", "Pulsa el stick derecho como un botón: es R3. Abre el teclado en pantalla en modo Escritorio.", "Pressione o analógico direito como um botão: esse é o R3. Ele abre o teclado visual no modo Desktop.");
    TourType => ("Choose a letter and type", "Elige una letra y escribe", "Escolha uma letra e digite");
    TourTypeCopy => ("Use the D-pad or left stick to select a letter, then press the bottom face button to type it. The right face button deletes a character.", "Usa la cruceta o el stick izquierdo para elegir una letra y pulsa el botón inferior para escribirla. El botón derecho borra un carácter.", "Use o direcional ou o analógico esquerdo para escolher uma letra e o botão inferior para digitá-la. O botão direito apaga um caractere.");
    TourCloseKeyboard => ("Back to the mouse", "Vuelve al ratón", "Volte ao mouse");
    TourCloseKeyboardCopy => ("Press R3 again to close the keyboard. Your controller returns to mouse controls.", "Pulsa R3 otra vez para cerrar el teclado. El mando vuelve a controlar el ratón.", "Pressione R3 novamente para fechar o teclado. Seu controle volta aos comandos de mouse.");
    TourGame => ("Ready to play? Switch modes", "¿Listo para jugar? Cambia de modo", "Vai jogar? Troque de modo");
    TourGameCopy => ("Press Select twice quickly to enter Game mode. Mouse and keyboard commands stop. Your game uses its own controls; Windows apps with native controller support use their own focus navigation.", "Pulsa Select dos veces rápidamente para entrar en Juego. Se detienen el ratón y el teclado. El juego usa sus propios controles; las apps de Windows compatibles usan su navegación de foco nativa.", "Dê dois toques rápidos em Select para entrar em Jogo. Os comandos de mouse e teclado param. O game usa seus próprios controles; apps do Windows compatíveis usam a navegação de foco nativa.");
    TourDesktop => ("Return to Desktop", "Vuelve a Escritorio", "Retorne ao Desktop");
    TourDesktopCopy => ("Press Select twice again to return. Release the buttons and center the sticks before using the mouse.", "Pulsa Select dos veces otra vez para volver. Suelta los botones y centra los sticks antes de usar el ratón.", "Dê dois toques em Select novamente para voltar. Solte os botões e centralize os analógicos antes de usar o mouse.");
    TourPause => ("Pause whenever you need", "Pausa cuando lo necesites", "Pause quando precisar");
    TourPauseCopy => ("Press Start + Select together to pause Desktop controls. Release them and repeat to resume. This keeps you in the same mode.", "Pulsa Start + Select juntos para pausar el control del escritorio. Suéltalos y repite para reanudar. Permaneces en el mismo modo.", "Pressione Start + Select juntos para pausar o controle do desktop. Solte e repita para retomar. Você continua no mesmo modo.");
    TourReady => ("Your desktop is in your hands", "Tu escritorio está en tus manos", "Seu desktop está nas suas mãos");
    TourReadyCopy => ("Choose how to start. You can revisit this tour anytime from the panel.", "Elige cómo empezar. Puedes volver a este recorrido desde el panel cuando quieras.", "Escolha como começar. Você pode rever este roteiro pelo painel quando quiser.");
    TourStart => ("Learn the basics", "Aprender lo básico", "Aprender o básico");
    TourNext => ("Continue", "Continuar", "Continuar");
    TourBack => ("Back", "Atrás", "Voltar");
    TourSkip => ("Skip and open the app", "Omitir y abrir la aplicación", "Pular e abrir o aplicativo");
    TourDemo => ("Watch a demo", "Ver demostración", "Ver demonstração");
    TourPractice => ("Try with my controller", "Probar con mi mando", "Experimentar com meu controle");
    TourDemoBadge => ("DEMONSTRATION", "DEMOSTRACIÓN", "DEMONSTRAÇÃO");
    TourSandbox => ("Practice area · commands stay inside this window", "Zona de práctica · los comandos se quedan en esta ventana", "Área de prática · os comandos ficam dentro desta janela");
    TourSuccess => ("Nice! You learned this command. Continue when you are ready.", "¡Muy bien! Ya conoces este comando. Continúa cuando quieras.", "Isso! Você aprendeu este comando. Continue quando quiser.");
    TourDemoSuccess => ("That's how it works. Try it yourself or continue to the next lesson.", "Así funciona. Pruébalo o continúa con la siguiente lección.", "É assim que funciona. Experimente ou continue para a próxima etapa.");
    TourOptional => ("Practice is optional. You can continue at any time.", "La práctica es opcional. Puedes continuar cuando quieras.", "A prática é opcional. Você pode continuar a qualquer momento.");
    TourTarget => ("Click here", "Haz clic aquí", "Clique aqui");
    TourMoveTarget => ("Target", "Objetivo", "Alvo");
    TourKeyboardUnavailable => ("This session cannot display the real on-screen keyboard. You can still learn its controls here.", "Esta sesión no puede mostrar el teclado en pantalla real. Puedes aprender sus controles aquí.", "Esta sessão não permite exibir o teclado visual real. Você pode aprender seus comandos por aqui.");
    TourPracticeDesktop => ("Practice desktop", "Escritorio de práctica", "Desktop de prática");
    TourTypingField => ("Your letters appear here…", "Tus letras aparecen aquí…", "Suas letras aparecem aqui…");
    TourNavigation => ("LB / L1: back · RB / R1: continue · Tab / Enter also work", "LB / L1: atrás · RB / R1: continuar · También puedes usar Tab / Enter", "LB / L1: voltar · RB / R1: continuar · Tab / Enter também funcionam");
    TourStartDesktop => ("Start in Desktop", "Empezar en Escritorio", "Começar no Desktop");
    TourStartGame => ("Start in Game", "Empezar en Juego", "Começar no modo Jogo");
    TourReplay => ("Review welcome tour", "Repetir bienvenida", "Rever boas-vindas");
    TourConnect => ("Connect a controller, or follow the demonstrations without one.", "Conecta un mando o sigue las demostraciones sin uno.", "Conecte um controle ou acompanhe as demonstrações sem ele.");
    TourProfile => ("This controller needs a profile. You can watch the demos and configure it in the panel afterward.", "Este mando necesita un perfil. Puedes ver las demostraciones y configurarlo después en el panel.", "Este controle precisa de um perfil. Você pode ver as demonstrações e configurá-lo depois no painel.");
    TourShortWelcome => ("Welcome", "Bienvenida", "Boas-vindas");
    TourShortModes => ("Two modes", "Dos modos", "Dois modos");
    TourShortKeyboard => ("Keyboard", "Teclado", "Teclado");
    TourShortSwitch => ("Switch & pause", "Cambiar y pausar", "Trocar e pausar");
    TourShortReady => ("Ready", "Listo", "Pronto");
    Donate => ("Donate", "Donar", "Apoiar");
    Projects => ("My Projects", "Mis proyectos", "Meus projetos");
    TrayOpen => ("Open panel", "Abrir panel", "Abrir painel");
    TrayHide => ("Hide panel", "Ocultar panel", "Recolher painel");
    DesktopControl => ("Desktop control", "Control del escritorio", "Controle do desktop");
    Connecting => ("Connecting to the system…", "Conectando al sistema…", "Conectando ao sistema…");
    Retry => ("Try again", "Intentar de nuevo", "Tentar novamente");
    ControllerPrefix => ("Controller: ", "Control: ", "Controle: ");
    KeyboardActive => ("On-screen keyboard · D-pad / left stick moves · A types · R3 closes", "Teclado en pantalla · cruceta / stick izquierdo mueve · A escribe · R3 cierra", "Teclado visual · direcional / analógico esquerdo move · A digita · R3 fecha");
    MouseMode => ("Mouse mode · release buttons and center sticks when connecting or resuming", "Modo ratón · suelta los botones y centra los sticks al conectar o reanudar", "Modo mouse · solte os botões e centralize os analógicos ao conectar ou retomar");
    Paused => ("Paused · press Start + Select to resume", "En pausa · pulsa Start + Select para reanudar", "Pausado · pressione Start + Select para retomar");
    ConnectGamepad => ("Connect a gamepad. Detection is automatic.", "Conecta un mando. La detección es automática.", "Conecte um gamepad. A detecção é automática.");
    SelectController => ("Select a controller with a profile or import one below.", "Selecciona un mando con perfil o importa uno abajo.", "Selecione um controle com perfil ou importe um abaixo.");
    DeviceSupport => ("USB/Bluetooth · Xbox, PlayStation, Switch and SDL profiles for generic devices.", "USB/Bluetooth · Xbox, PlayStation, Switch y perfiles SDL para mandos genéricos.", "USB/Bluetooth · Xbox, PlayStation, Switch e perfis SDL para genéricos.");
    KeyboardVisual => ("ON-SCREEN KEYBOARD", "TECLADO EN PANTALLA", "TECLADO VISUAL");
    PausedLabel => ("PAUSED", "EN PAUSA", "PAUSADO");
    MouseLabel => ("MOUSE", "RATÓN", "MOUSE");
    PanelIntro => ("Use your gamepad to control the desktop, type with the on-screen keyboard, or let a game receive the controller directly. Choose a mode to get started.", "Usa el mando para controlar el escritorio, escribir con el teclado en pantalla o dejar el control directo al juego. Elige un modo para empezar.", "Use seu controle para navegar no desktop, digitar pelo teclado visual ou jogar com a entrada direta do controle. Escolha um modo para começar.");
    ControllerSymbols => ("Controller symbols", "Símbolos del mando", "Símbolos do controle");
    ControllerSymbolsHelp => ("Choose symbols for this guide: on Xbox, A is on the bottom and X is on the left; PlayStation Cross (✕) is on the bottom. This does not change your controller mapping.", "Elige los símbolos de esta guía: en Xbox, A está abajo y X a la izquierda; la cruz de PlayStation (✕) está abajo. Esto no cambia la asignación del mando.", "Escolha os símbolos deste guia: no Xbox, A fica embaixo e X à esquerda; a cruz do PlayStation (✕) fica embaixo. Isso não altera o mapeamento do controle.");
    ChooseMode => ("How do you want to use your controller?", "¿Cómo quieres usar el mando?", "Como você quer usar o controle?");
    ResumeGuide => ("Start + Select pauses only Desktop mode. After resuming, release all buttons and center the sticks.", "Start + Select pausa solo el modo escritorio. Al reanudar, suelta los botones y centra los sticks.", "Start + Select pausa apenas o modo Desktop. Ao retomar, solte os botões e centralize os analógicos.");
    CommandGuide => ("Controller guide", "Guía del mando", "Guia de comandos");
    PointerCommands => ("Mouse and scrolling", "Ratón y desplazamiento", "Mouse e rolagem");
    NavigationCommands => ("Selection and windows", "Selección y ventanas", "Seleção e janelas");
    DesktopCommandsHelp => ("These controls apply in Desktop mode while the on-screen keyboard is closed.", "Estos controles se aplican en modo escritorio con el teclado en pantalla cerrado.", "Estes comandos funcionam no modo Desktop, com o teclado visual fechado.");
    KeyboardCommandsHelp => ("In Desktop mode, press R3 to open the keyboard. While it is open, the buttons below control typing, including F1–F12.", "En modo escritorio, pulsa R3 para abrir el teclado. Mientras esté abierto, estos botones controlan la escritura, incluyendo F1–F12.", "No modo Desktop, pressione R3 para abrir o teclado. Enquanto estiver aberto, os botões abaixo controlam a digitação, incluindo F1–F12.");
    GameCommandsHelp => ("Game mode is active: your game defines the controls. Double tap Select to return to Desktop and use the mouse or on-screen keyboard.", "Modo juego activo: el juego define los controles. Pulsa Select dos veces para volver al escritorio y usar el ratón o el teclado en pantalla.", "Modo Jogo ativo: os comandos são definidos pelo próprio jogo. Dê dois toques em Select para voltar ao Desktop e usar o mouse ou o teclado visual.");
    DesktopCommands => ("Desktop controls", "Controles del escritorio", "Comandos do desktop");
    KeyboardCommands => ("On-screen keyboard controls", "Controles del teclado en pantalla", "Comandos do teclado visual");
    ButtonPositionNote => ("South, east, west, and north refer to the physical positions of the four face buttons. Escape cancels menus; it is not the browser Back command.", "Sur, este, oeste y norte indican la posición física de los cuatro botones. Escape cancela menús; no es el comando Atrás del navegador.", "Inferior, direito, esquerdo e superior indicam a posição física dos quatro botões. Escape cancela menus; não é o comando Voltar do navegador.");
    KeyboardFocusNote => ("The keyboard does not focus its own window; the text field and cursor stay in the previous app.", "El teclado no enfoca su propia ventana; el campo y el cursor permanecen en la aplicación anterior.", "O teclado não ativa a própria janela; o campo e o cursor de texto permanecem no aplicativo anterior.");
    Settings => ("Settings", "Configuración", "Configurações");
    StartWithOS => ("Run when I sign in (starts minimized)", "Iniciar al iniciar sesión (minimizado)", "Iniciar com o sistema (minimizado)");
    Language => ("Language", "Idioma", "Idioma");
    PreferencesSaveFailedPrefix => ("Could not save preferences: ", "No se pudo guardar la configuración: ", "Não foi possível salvar as configurações: ");
    ControlsAndAdjustments => ("Controller and adjustments", "Mando y ajustes", "Controle e ajustes");
    InputMode => ("Input mode", "Modo de entrada", "Modo de entrada");
    DesktopMode => ("Desktop", "Escritorio", "Desktop");
    GameMode => ("Game", "Juego", "Jogo");
    DesktopModeHelp => ("Desktop mode lets the controller send mouse and keyboard actions. Start + Select pauses or resumes control.", "El modo escritorio permite controlar el ratón y el teclado. Start + Select pausa o reanuda el control.", "O modo Desktop permite controlar mouse e teclado. Start + Select pausa ou retoma o controle.");
    GameModeHelp => ("Gamepad Desktop stops sending mouse and keyboard actions. Your physical controller remains available to games.", "Gamepad Desktop deja de enviar acciones de ratón y teclado. El mando físico sigue disponible para los juegos.", "O Gamepad Desktop deixa de enviar comandos de mouse e teclado. O controle físico continua disponível para os jogos.");
    DoubleTapSelect => ("Select · double tap", "Select · pulsa dos veces", "Select · dois toques rápidos");
    DoubleTapSelectAction => ("Switch between Desktop and Game", "Alternar entre escritorio y juego", "Alternar entre Desktop e Jogo");
    DoubleTapSelectHelp => ("Double tap Select to switch modes.", "Pulsa Select dos veces para cambiar de modo.", "Dê dois toques rápidos em Select para alternar o modo.");
    GameModeBadge => ("GAME MODE", "MODO JUEGO", "MODO JOGO");
    DesktopModeNotice => ("Desktop mode selected. Start + Select pauses or resumes desktop control.", "Modo escritorio seleccionado. Start + Select pausa o reanuda el control del escritorio.", "Modo Desktop selecionado. Start + Select pausa ou retoma o controle do desktop.");
    GameModeNotice => ("Game mode enabled. Gamepad Desktop stopped sending mouse and keyboard actions.", "Modo juego activado. Gamepad Desktop dejó de enviar acciones de ratón y teclado.", "Modo Jogo ativado. O Gamepad Desktop parou de enviar comandos de mouse e teclado.");
    Automatic => ("Automatic", "Automático", "Automático");
    DisconnectedController => ("Disconnected controller", "Mando desconectado", "Controle desconectado");
    NeedsProfile => (" · profile required", " · necesita un perfil", " · precisa de perfil");
    ControlDesktop => ("Control desktop (Start + Select pauses/resumes)", "Controlar el escritorio (Start + Select pausa/reanuda)", "Controlar desktop (Start + Select pausa/retoma)");
    NotificationTitle => ("Gamepad Desktop", "Gamepad Desktop", "Gamepad Desktop");
    PauseCombo => ("Start + Select / Menu + View", "Start + Select / Menú + Vista", "Start + Select / Menu + View");
    ControlPausedNotice => ("Control paused. Press START + SELECT to resume.", "Control pausado. Pulsa START + SELECT para reanudar.", "Controle pausado. Pressione START + SELECT para retomar.");
    ControlResumedNotice => ("Control resumed. Release all buttons and center both sticks to continue.", "Control reanudado. Suelta los botones y centra ambos sticks para continuar.", "Controle retomado. Solte os botões e centralize os dois analógicos para continuar.");
    ControllerConnectedPrefix => ("Controller connected: ", "Control conectado: ", "Controle conectado: ");
    ControllerDisconnectedNotice => ("Controller disconnected. Reconnect it to resume control.", "Control desconectado. Reconéctalo para volver a controlar el escritorio.", "Controle desconectado. Reconecte-o para voltar a controlar o desktop.");
    NoUsableProfileNotice => ("No usable controller profile was found. Import an SDL mapping in the desktop panel.", "No se encontró un perfil de control compatible. Importa un mapeo SDL en el panel del escritorio.", "Nenhum perfil de controle compatível foi encontrado. Importe um mapeamento SDL no painel do desktop.");
    KeyboardUnavailableNotice => ("The on-screen keyboard could not be opened.", "No se pudo abrir el teclado en pantalla.", "Não foi possível abrir o teclado visual.");
    RuntimeStoppedPrefix => ("Desktop control stopped: ", "El control del escritorio se detuvo: ", "O controle do desktop parou: ");
    Speed => ("Speed", "Velocidad", "Velocidade");
    Deadzone => ("Deadzone", "Zona muerta", "Zona morta");
    CustomizeKeyboard => ("Customize QWERTY keyboard", "Personalizar teclado QWERTY", "Personalizar teclado QWERTY");
    WidthPx => ("Width (px)", "Ancho (px)", "Largura (px)");
    Top => ("Top", "Arriba", "Em cima");
    Bottom => ("Bottom", "Abajo", "Embaixo");
    Opacity => ("Opacity", "Opacidad", "Opacidade");
    RepeatDirection => ("Repeat D-pad while held", "Repetir la cruceta al mantenerla", "Repetir direcional ao segurar");
    InitialWait => ("Initial delay (ms)", "Espera inicial (ms)", "Espera inicial (ms)");
    RepeatInterval => ("Repeat interval (ms)", "Intervalo de repetición (ms)", "Intervalo (ms)");
    RestoreKeyboard => ("Restore keyboard defaults", "Restaurar ajustes del teclado", "Restaurar ajustes do teclado");
    KeyboardFollowsField => ("The keyboard follows the field when an app provides its position. Top/Bottom sets the fallback position when that information is unavailable.", "El teclado sigue el campo cuando la aplicación informa su posición. Arriba/Abajo define la posición alternativa cuando no está disponible.", "O teclado acompanha o campo quando o aplicativo fornece sua posição. Em cima/Embaixo define a alternativa quando essa informação não está disponível.");
    KeyboardSettingsHelp => ("With the keyboard open, Start opens these settings from the gamepad. Preferences are saved when you return.", "Con el teclado abierto, Start abre estos ajustes desde el mando. Las preferencias se guardan al volver.", "Com o teclado aberto, Start também abre estes ajustes pelo gamepad. As preferências são salvas ao voltar.");
    KeyboardBehaviorHelp => ("ANSI QWERTY · Shift applies to the next letter · Caps locks uppercase · Ctrl, Alt, and Super combine with the next key.", "QWERTY ANSI · Shift se aplica a la siguiente letra · Caps fija mayúsculas · Ctrl, Alt y Super se combinan con la siguiente tecla.", "QWERTY ANSI · Shift vale para a próxima letra · Caps fixa maiúsculas · Ctrl, Alt e Super combinam com a próxima tecla.");
    TrayUnavailablePrefix => ("Tray unavailable: ", "Bandeja no disponible: ", "Bandeja indisponível: ");
    ClosePanelKeepsRunning => ("Closing the panel keeps the controller active in the tray near the clock.", "Al cerrar el panel, el mando sigue activo en la bandeja junto al reloj.", "Fechar o painel mantém o controle ativo na bandeja, perto do relógio.");
    QuitApplication => ("Quit application", "Cerrar la aplicación", "Encerrar aplicativo");
    ControllerProfiles => ("Controller profiles", "Perfiles de mandos", "Perfis de controles");
    KnownProfiles => ("Known controllers use SDL profiles automatically. For an unknown model, import an SDL mapping line.", "Los mandos conocidos usan perfiles SDL automáticamente. Para un modelo desconocido, importa una línea de mapeo SDL.", "Controles conhecidos usam o perfil SDL automaticamente. Para um modelo desconhecido, importe uma linha de mapeamento SDL.");
    ApplySaveProfile => ("Apply and save profile", "Aplicar y guardar perfil", "Aplicar e salvar perfil");
    ProfileSaved => ("Profile applied and saved for future sessions.", "Perfil aplicado y guardado para próximas sesiones.", "Perfil aplicado e salvo para as próximas sessões.");
    KeyboardSettingsTitle => ("KEYBOARD SETTINGS", "AJUSTES DEL TECLADO", "AJUSTES DO TECLADO");
    KeyboardWindowTitle => ("Gamepad Desktop · Keyboard", "Gamepad Desktop · Teclado", "Gamepad Desktop · Teclado");
    KeyboardMoveAdjust => ("D-pad / left stick: up/down selects · left/right adjusts", "Cruceta / stick izquierdo: arriba/abajo selecciona · lados ajusta", "Direcional / analógico esquerdo: cima/baixo escolhe · lados altera");
    KeyboardWidth => ("Width", "Ancho", "Tamanho");
    KeyboardPosition => ("Position", "Posición", "Posição");
    KeyboardOpacity => ("Opacity", "Opacidad", "Opacidade");
    KeyboardRepeat => ("Repeat while held", "Repetir al mantener", "Repetir ao segurar");
    KeyboardRepeatDelay => ("Delay before repeat", "Espera antes de repetir", "Espera antes de repetir");
    KeyboardRepeatInterval => ("Repeat interval", "Intervalo de repetición", "Intervalo de repetição");
    Yes => ("yes", "sí", "sim");
    No => ("no", "no", "não");
    KeyboardCustomizeHelp => ("Start returns and saves · R3 closes", "Start vuelve y guarda · R3 cierra", "Start voltar e salvar · R3 fechar");
    KeyboardTypingHelp => ("A selects · F1–F12 · B delete · L3 space · R1 Shift · Start settings", "A activa · F1–F12 · B borrar · L3 espacio · R1 Shift · Start ajustes", "A ativa · F1–F12 · B apaga · L3 espaço · R1 Shift · Start ajustes");
    KeyboardNavigationHelp => ("D-pad / left stick moves · X Enter · Y Esc · L1 Tab · R3 close", "Cruceta / stick izquierdo mueve · X Enter · Y Esc · L1 Tab · R3 cerrar", "Direcional / analógico move · X Enter · Y Esc · L1 Tab · R3 fechar");
    Space => ("Space", "Espacio", "Espaço");
    KeyboardOverlayUnavailable => ("This compositor cannot show the custom keyboard without activating it.", "Este compositor no puede mostrar el teclado propio sin activarlo.", "Este compositor não permite exibir o teclado próprio sem ativá-lo.");
    KeyboardOpened => ("On-screen keyboard · D-pad / left stick moves · A types · B deletes · R3 closes.", "Teclado en pantalla · cruceta / stick izquierdo mueve · A escribe · B borra · R3 cierra.", "Teclado visual · direcional / analógico esquerdo move · A digita · B apaga · R3 fecha.");
    KeyboardClosed => ("On-screen keyboard closed.", "Teclado en pantalla cerrado.", "Teclado visual fechado.");
    PasteOneSdlMapping => ("Paste one SDL profile line: GUID,name,buttons/axes.", "Pega una línea de perfil SDL: GUID,nombre,botones/ejes.", "Cole uma única linha de perfil SDL: GUID,nome,botões/eixos.");
    SdlLoadProfilePrefix => ("Could not load SDL profile: ", "No se pudo cargar el perfil SDL: ", "Erro ao carregar perfil SDL: ");
    SdlReadProfilesPrefix => ("Could not read profiles: ", "No se pudieron leer los perfiles: ", "Erro ao ler os perfis: ");
    ProfileSaveFailedPrefix => ("Profile applied for this session, but could not be saved: ", "Perfil aplicado para esta sesión, pero no se pudo guardar: ", "Perfil aplicado nesta sessão, mas não foi salvo: ");
    SettingsSaveFailedPrefix => ("Could not save settings: ", "No se pudieron guardar los ajustes: ", "Não foi possível salvar ajustes: ");
    WindowsConnectFailedPrefix => ("Could not connect Windows input: ", "No se pudo conectar la entrada de Windows: ", "Falha ao conectar entrada do Windows: ");
    WindowsSendFailedPrefix => ("Could not send input to Windows: ", "No se pudo enviar la entrada a Windows: ", "Falha enviando entrada ao Windows: ");
    DesktopSendFailedPrefix => ("Could not send input to the desktop: ", "No se pudo enviar la entrada al escritorio: ", "Erro enviando entrada ao desktop: ");
    SdlErrorPrefix => ("SDL3: ", "SDL3: ", "SDL3: ");
    Dpad => ("D-pad", "Cruceta", "Direcional");
    DpadOrLeftStick => ("D-pad / left stick", "Cruceta / stick izquierdo", "Direcional / analógico esquerdo");
    RightStickPress => ("R3 · press right stick", "R3 · pulsar stick derecho", "R3 · pressionar analógico direito");
    LeftStickPress => ("L3 · press left stick", "L3 · pulsar stick izquierdo", "L3 · pressionar analógico esquerdo");
    LeftStick => ("Left stick", "Stick izquierdo", "Analógico esquerdo");
    RightStickVertical => ("Right stick ↕", "Stick derecho ↕", "Analógico direito ↕");
    SouthButton => ("South · A / ✕ / B on Switch", "Sur · A / ✕ / B en Switch", "Inferior · A / ✕ / B no Switch");
    EastButton => ("East · B / ○ / A on Switch", "Este · B / ○ / A en Switch", "Direito · B / ○ / A no Switch");
    WestButton => ("West · X / □ / Y on Switch", "Oeste · X / □ / Y en Switch", "Esquerdo · X / □ / Y no Switch");
    NorthButton => ("North · Y / △ / X on Switch", "Norte · Y / △ / X en Switch", "Superior · Y / △ / X no Switch");
    SouthButtonShort => ("A / south button", "A / botón sur", "A / botão inferior");
    EastButtonShort => ("B / east button", "B / botón este", "B / botão direito");
    WestButtonShort => ("X / west button", "X / botón oeste", "X / botão esquerdo");
    NorthButtonShort => ("Y / north button", "Y / botón norte", "Y / botão superior");
    ToggleKeyboard => ("Open / close custom on-screen keyboard", "Abrir / cerrar teclado en pantalla", "Abrir / fechar teclado visual próprio");
    SystemLauncher => ("System launcher · Super / Windows", "Lanzador del sistema · Super / Windows", "Lançador do sistema · Super / Windows");
    MovePointer => ("Move pointer", "Mover el puntero", "Mover ponteiro");
    ScrollPage => ("Scroll page", "Desplazar página", "Rolar página");
    LeftClickDrag => ("Left click · hold to drag", "Clic izquierdo · mantener para arrastrar", "Clique esquerdo · segurar para arrastar");
    RightClick => ("Right click", "Clic derecho", "Clique direito");
    EnterSelected => ("Enter · open selected item", "Enter · abrir elemento seleccionado", "Enter · abrir item selecionado");
    EscapeCancel => ("Escape · close menu / cancel", "Escape · cerrar menú / cancelar", "Escape · fechar menu / cancelar");
    ArrowRepeat => ("Arrow keys · hold to repeat", "Flechas · mantener para repetir", "Setas · segurar para repetir");
    NextField => ("Tab · next field", "Tab · campo siguiente", "Tab · próximo campo");
    SwitchWindow => ("Alt+Tab · switch window", "Alt+Tab · cambiar ventana", "Alt+Tab · alternar janela");
    PauseResume => ("Pause / resume · release and center to resume", "Pausar / reanudar · suelta y centra para reanudar", "Pausar / retomar · solte e centralize para retomar");
    MoveKeyboardSelection => ("Move selection across the keyboard", "Mover la selección por el teclado", "Mover a seleção pelo teclado");
    TypeSelectedKey => ("Activate the selected key", "Activar la tecla seleccionada", "Ativar a tecla selecionada");
    DeleteCharacter => ("Delete one character", "Borrar un carácter", "Apagar um caractere");
    InsertEnter => ("Insert Enter", "Insertar Enter", "Inserir Enter");
    InsertEscape => ("Insert Escape", "Insertar Esc", "Inserir Esc");
    InsertSpace => ("Insert space", "Insertar espacio", "Inserir espaço");
    InsertTab => ("Insert Tab", "Insertar Tab", "Inserir Tab");
    ToggleShift => ("Toggle Shift", "Activar o desactivar Shift", "Ativar ou desativar Shift");
    CloseKeyboard => ("Close on-screen keyboard", "Cerrar teclado en pantalla", "Fechar o teclado visual");
}
