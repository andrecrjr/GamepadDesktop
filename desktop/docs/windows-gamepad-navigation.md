# Navegação do gamepad no Windows

## Problema e solução

O Windows pode converter entradas do controle em teclas virtuais de navegação. Assim, mover o analógico para controlar o ponteiro também pode mover a seleção de um painel do sistema, e pressionar A pode ativar um item além do clique enviado pelo Gamepad Desktop.

O aplicativo instala um hook `WH_KEYBOARD_LL` em uma thread própria. Durante o modo Desktop ativo, com um controle conectado e fora do onboarding, ele bloqueia somente `VK_GAMEPAD_A` até `VK_GAMEPAD_RIGHT_THUMBSTICK_LEFT` (`0xC3`–`0xDA`), exceto na sessão de teclado aberta com R3. Essa faixa inclui botões, direcional e as direções dos dois analógicos. Teclas comuns e cliques continuam passando; a leitura física do controle pela SDL continua independente do filtro.

O hook não ativa janelas nem altera o Registro ou instala drivers. A instalação precisa terminar antes de o runtime começar a enviar entrada. Se a instalação falhar, o runtime informa o erro em vez de iniciar o controle sem o filtro.

Com o filtro ativo, o backend Windows também descarta as ações `Dpad` geradas pelo controlador do app. Bloquear apenas as teclas virtuais do Windows ainda deixava essas ações enviarem setas normais a qualquer programa e moverem seu foco. No modo de mouse do Windows, o direcional agora não envia essas setas.

## Teclado de toque nativo

R3 abre e fecha o teclado de toque do Windows nos modos Desktop e Jogo. A apresentação própria do app não é desenhada no Windows. O app solicita o layout Gamepad pela API `CoreInputView.TryShowWithKind(Gamepad)`. Esse layout chegou ao Windows 11 24H2 a partir do build 26100.3613; o foco aprimorado chegou no build 26100.4762. O pedido da API é recusado quando o app chamador não está em primeiro plano. Se o layout estiver indisponível ou o Windows recusar o pedido, o app mantém o teclado comum: durante a sessão iniciada com R3, cada passo do direcional move o ponteiro aproximadamente uma tecla, o analógico esquerdo move-o suavemente e A clica na tecla sob o ponteiro.

A chamada usa `ITipInvocation::Toggle`, uma interface interna utilizada por ferramentas desktop, pois a API pública `InputPane.TryShow` rejeita chamadas de aplicativos em segundo plano. Se o serviço de lançamento ainda não estiver disponível, o app inicia `CommonProgramFiles\microsoft shared\ink\TabTip.exe` sem console e tenta a interface novamente. Não troca arquivos do sistema nem altera o Registro. Como a interface de lançamento é interna, sua disponibilidade precisa ser validada na versão real do Windows.

`IFrameworkInputPane::Location` fornece o retângulo de visibilidade. O app consulta essa informação no runtime, com intervalo de 50 ms, e aguarda até cinco segundos por uma transição. Requisições de abrir e fechar são serializadas para não inverter uma animação ainda em andamento. Abrir pelo próprio Windows e fechar pelo próprio teclado também atualiza a sessão do app. Uma falha fica visível no painel e permite tentar R3 novamente.

Somente um pedido explícito de abertura com R3 libera a navegação nativa durante a sessão do layout Gamepad. Essa liberação acompanha a abertura e o fechamento assíncronos, até o teclado terminar de desaparecer. O backend descarta movimentos, cliques, rolagem, setas e atalhos sintetizados pelo app, preservando apenas liberações de botões; ele não reposiciona o ponteiro. Isso evita misturar a navegação do teclado com mouse simulado. Fechar com R3 ou pelo próprio teclado encerra a sessão; os mapeamentos do app exigem soltar e centralizar o controle antes de retomar.

Uma abertura automática pelo Windows atualiza a visibilidade, mas não libera o filtro nativo no modo Desktop nem ativa os passos do direcional. O controle de ponteiro e clique continua limitado ao retângulo do teclado. Nenhum desses caminhos impede jogos de consultar diretamente o controle físico.

## Modos e encerramento

- Desktop ativo fora da sessão de teclado solicitada com R3: bloqueia novas sequências de teclas virtuais de gamepad.
- Sessão de teclado solicitada com R3, Jogo, pausa, desconexão e onboarding: permite novas sequências nativas.
- Uma tecla mantida conserva a decisão do pressionamento até a soltura, inclusive nas repetições. Isso evita entregar um pressionamento ao Windows e bloquear sua soltura ao trocar de modo.
- No encerramento do runtime, o filtro é desativado, a thread recebe `WM_QUIT`, o hook é removido e a thread é aguardada. O Windows também remove hooks quando o processo termina.

O callback apenas consulta uma flag atômica e atualiza o estado de 24 teclas. A thread de mensagens não faz polling SDL, renderização, acesso a arquivos nem espera por locks do runtime.

## Limites

O filtro atua nas teclas virtuais geradas pelo Windows, não em leituras diretas de XInput, GameInput, Windows.Gaming.Input ou HID realizadas por outros aplicativos. Jogos que consultam o controle diretamente ainda podem reagir enquanto o Gamepad Desktop funciona em segundo plano.

O hook não identifica qual controle produziu a tecla virtual: todos os controles que geram essa faixa no desktop interativo seguem o modo ativo do app. Ele não promete cobertura para a tela de segurança/UAC. O Windows pode remover silenciosamente um hook que exceda o timeout; por isso o callback deve permanecer curto e sem operações bloqueantes.

## Validação

Os testes automatizados verificam a faixa inteira de teclas virtuais, passagem das demais teclas, repetições e pares de pressionamento/soltura ao trocar de modo. A compilação para Windows verifica a integração com as APIs nativas. Esses testes não demonstram o comportamento de um controle físico no Windows 11.

Validar em Windows 11 com o controle Xbox:

1. No modo Desktop, abrir Iniciar, Configurações e Configurações Rápidas. Mover os dois analógicos: o app deve mover o ponteiro/rolar sem deslocar a seleção nativa por causa do controle.
2. Apontar para um item e pressionar A uma vez: deve ocorrer somente o clique pretendido. Verificar clique direito e arraste.
3. Usar o direcional com o teclado fechado em um programa comum e nos painéis do Windows: ele não deve mudar o foco por setas enviadas pelo app. Abrir o teclado de toque com R3 nos modos Desktop e Jogo. Se o layout Gamepad for aceito, confirmar que o Windows recebe o controle nativamente; se o pedido for recusado, confirmar que o direcional percorre as teclas pelo ponteiro, o analógico move o ponteiro e A seleciona. Em ambos os casos, digitar no campo original. Abrir automaticamente pelo Windows e confirmar que isso não libera a navegação nativa nem os passos do direcional no modo Desktop.
4. Fechar com R3 e pelo botão do próprio teclado. Confirmar que o painel acompanha a visibilidade e que os comandos do app só retomam após soltar e centralizar o controle. Pressionar R3 novamente durante a animação e verificar que o estado final corresponde ao último pedido.
5. Usar teclado e mouse físicos: letras, setas, Tab, Enter e cliques devem continuar funcionando.
6. Pausar, entrar no modo Jogo, desconectar e encerrar pela bandeja: novas entradas devem voltar ao comportamento nativo. Repetir mantendo uma direção pressionada durante as transições e confirmar recuperação após soltá-la.
7. Repetir nos programas que apresentavam o conflito e registrar quais ainda reagem por leitura direta do controle.

## Fontes

- [Microsoft: layout Gamepad e melhorias de navegação e foco do teclado de toque](https://blogs.windows.com/windows-insider/2025/07/10/releasing-windows-11-build-26100-4762-to-the-release-preview-channel/).
- [Microsoft: códigos de teclas virtuais](https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes).
- [Microsoft: LowLevelKeyboardProc, supressão, thread de mensagens e timeout](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc).
- [Microsoft: ciclo de vida dos hooks](https://learn.microsoft.com/en-us/windows/win32/winmsg/about-hooks).
- [XInputUWPFix: implementação independente do filtro de teclas de gamepad](https://github.com/BlueAmulet/XInputUWPFix/blob/main/XInputUWPFix.c).
- [Microsoft: alternativa global pelo Registro, ControllerToVKMapping](https://blogs.windows.com/windows-insider/2021/01/06/announcing-windows-10-insider-preview-build-21286/). Esta alternativa não é alterada pelo app.
- [Microsoft: InputPane.TryShow e rejeição de chamadas em segundo plano](https://learn.microsoft.com/en-us/uwp/api/windows.ui.viewmanagement.inputpane.tryshow).
- [Microsoft: IFrameworkInputPane.Location](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-iframeworkinputpane-location).
- [Microsoft GDK: abertura do teclado Gamepad com CoreInputView](https://learn.microsoft.com/en-us/xbox/gdk/docs/features/common/input/overviews/virtualkeyboardsupport).
- [Microsoft: CoreInputView.TryShowWithKind é recusado quando o aplicativo não está em primeiro plano](https://learn.microsoft.com/en-us/uwp/api/windows.ui.viewmanagement.core.coreinputview.tryshow?view=winrt-28000).
- [Exemplo de cliente desktop utilizando ITipInvocation](https://github.com/TransposonY/GestureSign/blob/master/GestureSign.CorePlugins/TouchKeyboard/TouchKeyboard.cs).
