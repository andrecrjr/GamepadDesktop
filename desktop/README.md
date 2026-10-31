# Gamepad Desktop

Use o controle para navegar pelo computador, mover o ponteiro e digitar sem tirar o foco do aplicativo que você está usando.

O app controla o ponteiro e envia comandos de teclado. No Windows, R3 abre o teclado de toque do próprio sistema. No COSMIC Wayland, R3 abre o teclado do desktop; nos demais ambientes Linux, usa o teclado visual do app quando o compositor permite. Ele continua funcionando quando o painel é recolhido para a bandeja do sistema.

Notificações do sistema avisam sobre pausa e retomada, conexão do controle e falhas mesmo com o painel recolhido. No Windows, a primeira execução cria o atalho por usuário “Gamepad Desktop” no menu Iniciar para identificar os avisos.

> O desktop está em desenvolvimento. Cada aplicativo interpreta as teclas recebidas à sua maneira, e ainda falta validar o uso com controles físicos em Windows e Linux.

![Prévia do teclado visual](keyboard-preview.png)

## Começar

No Windows, baixe o [executável x64](dist/GamepadDesktop-Windows-x64.exe), conecte o controle e abra `GamepadDesktop-Windows-x64.exe`. O painel mostra quando o controle foi reconhecido e quais comandos estão disponíveis. Use **Buscar atualizações**, no rodapé do painel, para instalar a versão mais recente e reiniciar o app.

Em distribuições x64 baseadas em Debian (como Debian, Ubuntu e Pop!_OS), instale o [pacote `.deb`](dist/GamepadDesktop-Linux-x64.deb) pela loja do sistema ou com `sudo apt install ./GamepadDesktop-Linux-x64.deb`. **Buscar atualizações** baixa o novo pacote e pede autorização para instalá-lo. Em outras distribuições, baixe e extraia o [pacote x64 `.tar.gz`](dist/GamepadDesktop-Linux-x64.tar.gz); abra o app a partir de uma pasta gravável pelo usuário para permitir a troca do executável. Para gerar os pacotes localmente, instale `cargo-deb` com `cargo install cargo-deb --version 3.8.0 --locked`. As instruções para executar o app e as permissões necessárias estão no [guia do sistema](docs/system-overview.md#como-iniciar-no-linux).

## Boas-vindas e prática guiada

O capítulo de mouse tem cinco práticas: mover o ponteiro, clique esquerdo, clique direito com menu de contexto, segurar/mover/soltar para arrastar e rolar páginas. Cada etapa destaca os SVGs dos botões de Xbox/PlayStation e tem demonstração sem controle conectado.

A interface começa ampliada em 125% para facilitar a leitura à distância. Use **A− / A+** no onboarding ou nas configurações para escolher de 100% a 175%. A escolha é salva automaticamente e vale para os textos, botões e ícones do painel.

Na primeira abertura, o roteiro apresenta os modos **Desktop** e **Jogo**, depois ensina um comando por vez: mover, clicar, rolar, abrir/digitar/fechar o teclado, alternar modos e pausar.

- A prática usa os mesmos comandos do aplicativo em um desktop simulado, sem enviar cliques ou teclas aos outros programas.
- Sem controle, use **Ver demonstração** ou **Continuar**. Você pode pular qualquer etapa ou todo o roteiro.
- Durante o roteiro, **LB/L1** volta e **RB/R1** continua. Mouse e teclado (Tab/Enter) também funcionam.
- **Select duas vezes** alterna Desktop/Jogo. **Start + Select** pausa/retoma o modo Desktop; são gestos diferentes.
- Ao terminar, escolha **Começar no Desktop** ou **Começar no modo Jogo**. A conclusão fica salva nas preferências.
- **Rever boas-vindas**, no rodapé do painel, reabre a prática. O roteiro respeita o idioma escolhido e permite selecionar símbolos de Xbox/PlayStation.

Configurações de versões anteriores continuam válidas e recebem o roteiro uma vez. A disponibilidade do teclado real depende do sistema; a demonstração funciona mesmo quando o teclado visual não está disponível na sessão.

## Comandos principais

| Controle                             | Ação                                                         |
| ------------------------------------ | ------------------------------------------------------------ |
| Analógico esquerdo                   | Mover o ponteiro                                             |
| Analógico direito, cima/baixo        | Rolar a página                                               |
| Direcional                           | Navegação no teclado aberto; fora dele, setas somente no Linux |
| Botão inferior — A / ✕ / B no Switch | Clicar; no teclado visual, digitar a tecla selecionada       |
| Botão direito — B / ○ / A no Switch  | Clique direito                                               |
| Botão esquerdo — X / □ / Y no Switch | Enter                                                        |
| Botão superior — Y / △ / X no Switch | Escape                                                       |
| L3                                   | Abrir o lançador do sistema                                  |
| LB / L1                              | Tab                                                          |
| RB / R1                              | Abre o seletor Alt+Tab; R1 avança; X confirma; Y cancela     |
| Segure Start/Menu + LB / L1          | Print Screen · inicia a captura do sistema                  |
| Segure Start/Menu + RB / R1          | F11 · alternar tela cheia                                   |
| R3 — pressionar o analógico direito  | Abrir ou fechar o teclado, nos modos Desktop e Jogo           |
| Start + Select / Menu + View         | Pausar ou retomar; solte os botões e centralize para retomar |

Os nomes e comandos completos aparecem no painel, incluindo os equivalentes de Xbox, PlayStation e Switch. A seção de ajustes permite escolher o controle, ajustar velocidade e personalizar o teclado.

No Windows, **R3** abre o teclado de toque do sistema. Quando o Windows aceita o layout **Gamepad**, o controle navega pelas teclas nativamente. Se o layout não estiver disponível ou o pedido for rejeitado, o direcional move o ponteiro entre as teclas, o analógico esquerdo move o ponteiro suavemente e **A** seleciona. Isso vale nos modos Desktop e Jogo e só durante a sessão aberta com R3. R3 fecha o teclado; os comandos do app retomam depois de fechar, soltar e centralizar o controle. Abrir o teclado automaticamente pelo Windows não libera a navegação nativa no modo Desktop.

No teclado próprio do Linux, o **analógico esquerdo** move a seleção entre as teclas e navega pelos ajustes. Ele respeita a zona morta e a repetição configuradas. Nas diagonais, o eixo mais inclinado define a direção; se você usar o direcional ao mesmo tempo, ele tem prioridade.

## O que saber antes de usar

- No Windows, o direcional não envia setas ao aplicativo ativo durante o modo Desktop. Na sessão de teclado aberta com R3, o próprio Windows recebe direcional e analógico. No Linux, as setas são enviadas ao aplicativo ativo e o suporte depende do próprio aplicativo.
- No Windows, o modo Desktop ativo filtra as teclas virtuais de gamepad geradas pelo sistema. O filtro libera a navegação nativa durante a sessão de teclado aberta com R3, no modo Jogo, na pausa e na desconexão. Ele não bloqueia aplicativos que leem diretamente o controle; veja os [limites e a validação no Windows 11](docs/windows-gamepad-navigation.md).
- No Windows, pausar com Start + Select oculta os cursores padrão enquanto há um controle conectado. Retomar, desconectar o controle ou encerrar normalmente pela bandeja restaura o esquema de cursores. Cursores personalizados de aplicativos podem continuar visíveis. Se o processo for encerrado à força durante a pausa, reaplique o esquema em Configurações do mouse → Ponteiros para restaurá-los.
- O teclado próprio do Linux usa o layout QWERTY em inglês; símbolos e acentos podem variar conforme o layout ativo. O teclado de toque do Windows usa os layouts e idiomas configurados no sistema.
- Em algumas sessões Wayland, incluindo o GNOME Wayland, o teclado visual não está disponível.
- Os pacotes Windows e Linux existem, mas o uso com controle físico ainda precisa de validação em Windows 11, Linux/X11 e Wayland.

## Documentação

- [Como o sistema funciona](docs/system-overview.md): controles, fluxo do controle, diferenças entre sistemas, limites e execução.
- [Pesquisa do teclado visual](docs/keyboard-research.md): decisões de layout, foco e posicionamento.
- [Navegação do gamepad no Windows](docs/windows-gamepad-navigation.md): filtro de teclas virtuais, limites e validação física.
- [Roadmap](docs/roadmap.md): trabalho concluído e validações pendentes.

### Teclado no COSMIC (Wayland)

Em sessões COSMIC, o app usa o `cosmic-osk` já iniciado pelo desktop. R3 abre/fecha;
COSMIC controla a seleção e digitação pelo gamepad, e B fecha. As teclas e o layout
seguem as configurações do COSMIC. O app não injeta entradas duplicadas enquanto
esse teclado está aberto.

Se o serviço não estiver disponível ou rejeitar o comando, o app usa o teclado
próprio existente. Outros desktops Linux continuam usando esse teclado, com as
mesmas limitações de compositor (incluindo a ausência de layer-shell no GNOME).
A integração usa as ações `show`/`hide` de `com.system76.CosmicOSK`; elas alteram o
estado `always_shown` do COSMIC. Não habilite abertura automática do OSK por outros
atalhos durante o uso: esta integração acompanha aberturas por R3 e fechamento por
R3/B, não toda mudança externa de visibilidade.
