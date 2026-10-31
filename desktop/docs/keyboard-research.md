# Teclado QWERTY para gamepad

> Registro histórico da pesquisa e das correções do teclado próprio. A implementação atual usa o teclado nativo no Windows e no COSMIC quando disponível; consulte o [guia de uso](../README.md#on-screen-keyboard) e a [visão do sistema](system-overview.md) para o comportamento atual.

Pesquisa de 19/10/2026. Requisito: teclado próprio, QWERTY fiel, operável pelo gamepad, com experiência equivalente em Linux e Windows.

## O defeito de velocidade

A SDL fornece o estado atual de cada botão, não uma sequência de repetições do teclado do sistema. A repetição do direcional pertence ao nosso controlador. No código anterior, `MouseController::update` chamava `release` a cada leitura enquanto o teclado estava aberto; esse método zerava os quatro temporizadores. Na leitura seguinte, um botão mantido já podia repetir. O ciclo dorme aproximadamente 8 ms: trocar apenas a constante de intervalo não corrigia o problema.

A correção libera somente os botões do mouse durante a digitação. Os temporizadores permanecem ativos até terminar o modo ou rearmar a direção. O teste de regressão falhou na segunda leitura de 8 ms antes da correção e passou depois, tanto a 8 quanto a 20 ms por leitura.

Fonte: [SDL_GetGamepadButton](https://wiki.libsdl.org/SDL3/SDL_GetGamepadButton).

## QWERTY e memória espacial

Wilson e Agrawala estudaram seleção de caracteres com dois analógicos, usando metades do QWERTY. O estudo encontrou transferência de familiaridade com teclado e maior velocidade que a seleção com um único cursor. Isso apoia preservar as posições conhecidas das letras; não demonstra que qualquer interface com dois analógicos seja melhor para todos.

O estudo TwoStick também mostra a diferença entre facilidade inicial e desempenho após treino: oito participantes usaram cada técnica em vinte sessões de quinze minutos; a técnica alternativa começou mais lenta e terminou com 14,9 palavras/minuto, contra 12,9 da seleção QWERTY. Esses resultados antigos e controlados não são uma previsão de desempenho deste aplicativo. O requisito deste projeto permanece QWERTY convencional.

Fontes: [Microsoft Research, CHI 2006](https://www.microsoft.com/en-us/research/publication/text-entry-using-dual-joystick-game-controller/), [TwoStick, Graphics Interface 2007](https://graphicsinterface.org/proceedings/gi2007/gi2007-15/).

## Customizações implementadas

| Área         | Comportamento                                                                                                          |
| ------------ | ---------------------------------------------------------------------------------------------------------------------- |
| Layout       | Bloco ANSI QWERTY de 15 unidades por fileira; Tab 1,5; Caps 1,75; Shift esquerdo 2,25; Shift direito 2,75; Espaço 6,25 |
| Seleção      | Navegação vertical considera a extensão horizontal da tecla e mantém o alinhamento ao atravessar teclas largas         |
| Repetição    | Primeiro movimento imediato; espera e intervalo independentes, com opção de desativar repetição                        |
| Tamanho      | Largura de 520 a 1000 px, altura proporcional; padrão de 640 × 229 px                                                  |
| Posição      | Parte superior ou inferior, alterável sem sair do gamepad                                                              |
| Aparência    | Opacidade ajustável; destaque de seleção, Caps e modificadores; legendas alternativas dos símbolos                     |
| Ajustes      | Start abre/fecha; cima/baixo escolhe; esquerda/direita altera; o ajuste não envia texto ao aplicativo                  |
| Preferências | Arquivo JSON no diretório de configuração do usuário; valores carregados são limitados a faixas válidas                |
| Atalhos      | Ctrl, Alt e Super combinam com a próxima tecla e são liberados após o envio; Shift de uso único e Caps persistente     |

As legendas usam texto disponível nas fontes embarcadas. Símbolos como a seta de Backspace e algumas setas direcionais apareceram como quadrados na prévia real; foram substituídos por palavras.

## Foco, cursor e posição do campo

Desenhar o teclado e descobrir onde fica o campo de outro aplicativo são capacidades diferentes. A API flutuante da Steam recebe explicitamente o retângulo do campo para evitar cobri-lo. Um teclado independente não recebe automaticamente essa informação de cada programa.

No Wayland, `keyboard_interactivity = none` impede que a camada receba foco; uma região de entrada vazia permite o repasse do mouse. Essa camada não fornece o texto ou o cursor do aplicativo ativo. O protocolo de entrada de texto utiliza cooperação entre cliente, compositor e método de entrada para comunicar conteúdo e retângulo do cursor. Portanto, posicionamento automático junto ao campo e espelhamento de texto exigem integração adicional; os controles manuais de posição continuam úteis.

No Windows, `WS_EX_NOACTIVATE` é a capacidade nativa para janelas que não devem se tornar a janela ativa. UI Automation pode fornecer informação de texto quando o aplicativo implementa os padrões necessários; isso não garante cobertura universal.

O fechamento do painel também depende do backend: o Winit documenta `set_visible(false)` como não suportado em Wayland. Cancelar o fechamento e pedir essa ocultação mantinha a janela visível. No Linux, o fechamento nativo agora é permitido e o processo continua aguardando a bandeja; no Windows, a ocultação mantém a fila de eventos ativa. O teste do evento de fechamento falhou antes dessa correção e passou depois.

Fonte: [Winit: Window::set_visible](https://docs.rs/winit/0.30.13/winit/window/struct.Window.html#method.set_visible).

Fontes: [Steamworks: ShowFloatingGamepadTextInput](https://partner.steamgames.com/doc/api/isteamutils#ShowFloatingGamepadTextInput), [protocolo layer shell](https://wayland.app/protocols/wlr-layer-shell-unstable-v1), [text-input-v3](https://wayland.app/protocols/text-input-unstable-v3), [Microsoft: estilos de janela](https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles), [Microsoft: TextPattern](https://learn.microsoft.com/en-us/dotnet/framework/ui-automation/ui-automation-textpattern-overview).

## Limites e próxima avaliação

- O layout entregue é ANSI US/QWERTY. ABNT2 também é QWERTY, mas requer outra distribuição de símbolos e tratamento de acentos. No Linux, o mapa ativo do sistema interpreta os códigos de tecla enviados por `uinput`; o desenho US não substitui esse mapa.
- A interface e as preferências são comuns; a exibição sem foco e o envio de entrada usam capacidades específicas de cada sistema. Compositores sem layer shell continuam exigindo uma alternativa de apresentação.
- Os testes verificam repetição, Caps/Shift, combinações, ajustes pelo controle, fileiras, proporções e seleção sobre a barra de espaço. A prévia usa o renderizador real em software em três tamanhos e no modo de ajustes.
- A avaliação com o gamepad físico deve confirmar conforto, legibilidade, digitação no cursor original e posição adequada em campos próximos às bordas. Build Windows não substitui essa avaliação no Windows 11.

## Posicionamento junto ao campo e fundo no Windows — 19/10/2026

A implementação agora usa o retângulo do elemento focado via [UI Automation GetFocusedElement](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationclient/nf-uiautomationclient-iuiautomation-getfocusedelement) no Windows e [AT-SPI Component.GetExtents](https://gnome.pages.gitlab.gnome.org/gtk/atspi2/method.Component.get_extents.html) no Linux. No AT-SPI, os estados Focused, Editable, Visible e Showing são exigidos. Nenhum conteúdo do campo é consultado. Eventos de foco e consultas periódicas atualizam apenas a geometria em uma thread separada, com timeout das chamadas. A busca inicial é limitada em tempo e quantidade de elementos.

Ordem de posicionamento: acima com separação de 12 px; abaixo; lateral; borda com menor sobreposição se não houver espaço livre. O centro horizontal acompanha o campo, com contenção nas bordas do monitor. Windows converte coordenadas físicas da UIA para pontos da janela egui e usa a área útil; Wayland escolhe a saída anunciada pelo compositor e usa margens relativas ao monitor.

Isso não resolve a ausência de coordenadas globais em todos os clientes Wayland. A [documentação GTK](https://gnome.pages.gitlab.gnome.org/gtk/gtk3/method.Window.get_position.html) explica que a posição da janela pode ser sempre (0, 0) nesses sistemas. Quando as coordenadas Screen e Window coincidem, o rastreador considera o resultado ambíguo e usa a posição manual. Isso também pode excluir uma janela legitimamente em (0, 0); é uma escolha conservadora. Aplicativos sem árvore acessível também usam a alternativa. A renderização Wayland ainda depende de wlr-layer-shell, como antes.

O retângulo escuro do Windows tinha uma causa independente do posicionamento: o viewport era sempre visível, mesmo com a sessão fechada, e eframe limpava o fundo com sua cor padrão. Agora a visibilidade acompanha a sessão, a limpeza usa RGBA zero e a sombra nativa é desativada. [egui ViewportBuilder](https://docs.rs/egui/0.37.0/egui/struct.ViewportBuilder.html) documenta que transparência do viewport requer também configurar App::clear_color no eframe. Testes reproduziram a visibilidade incorreta antes da correção; a aparência final precisa de conferência no Windows 11 real.
