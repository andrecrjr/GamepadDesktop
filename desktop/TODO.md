# Próximos passos — desktop controlado por gamepad

## Objetivo de conclusão

Operar o desktop pelo controle em Windows e Linux: navegar por elementos acessíveis, usar o ponteiro quando a navegação semântica não estiver disponível, digitar em campos focados com o teclado virtual próprio e trocar/acionar janelas e funções do sistema.

“Completo” significa cobrir uma matriz publicada de sistemas e aplicativos. Aplicativos que não expõem a interface por acessibilidade ainda precisam funcionar no modo mouse; não se deve prometer navegação por elementos em todo aplicativo.

## Estado atual

- [x] Pasta Rust independente da extensão.
- [x] Protótipo eframe com grade numérica/QWERTY, realce de seleção e campo de demonstração.
- [x] Teclado próprio por L3, navegação com direcional/analógico e composição na grade.
- [x] Inserir o texto confirmado no cursor acessível (AT-SPI no Linux; controle em foco no Windows), substituindo a seleção atual quando o sistema expõe esse campo.
- [x] Guia visível dos comandos de mouse e teclado; L3 solicita reabrir o painel recolhido.
- [x] Leitura comum por SDL3 no Linux e Windows, com perfis de diversas famílias e eventos em segundo plano.
- [x] Seleção de dispositivo e importação/persistência de perfis SDL para controles desconhecidos detectados.
- [x] Conversão de analógicos e botões em movimento, clique/arrasto, rolagem, setas, Enter, Escape, Super/Windows e Alt+Tab.
- [x] Pausa por Start, liberação de cliques ao desconectar e retomada após centralizar/soltar o controle.
- [ ] Concluir a validação física de controle de outros aplicativos e janelas.

## Corte atual: operar o desktop em modo mouse

Esta entrega prioriza as partes de entrada e mouse das etapas 1, 2, 3, 5 e 7. A navegação por itens ainda não foi implementada. A entrada de texto em campos externos tem um primeiro fluxo, sujeito ao suporte de acessibilidade do aplicativo e à validação física em cada sistema.

- [x] Painel com conexão, pausa, velocidade e zona morta.
- [x] Práticas separadas de clique esquerdo, clique direito, arrastar e rolar no onboarding, com demonstrações locais.
- [x] Escala do painel de 100% a 175%, com padrão de 125%, ajuste no onboarding e nas configurações e preferência persistente.
- [x] Loop de controle independente do foco da janela do app.
- [x] Bandeja com abrir/recolher painel e encerramento explícito; fechar o painel mantém o serviço ativo.
- [x] Validar recolher/reabrir e encerrar pela bandeja no COSMIC/Wayland sem reiniciar o controle.
- [x] Adaptadores de entrada nativa para Linux e Windows.
- [x] Testes da lógica de movimento, clique, pausa, desconexão, atalhos e repetição.
- [x] Classificação de mouse/teclado virtual pelo udev e testes da seleção/persistência de perfis.
- [x] Leitura física do 8BitDo pela SDL3 e importação de perfil para um controle virtual desconhecido.
- [x] Checagem, lint e build release Windows x64 com SDL3 incorporada; pacote ZIP criado.
- [ ] Criar assistente visual para aprender botões/eixos e calibrar controles desconhecidos sem importar texto.
- [ ] Validar Xbox, DualShock/DualSense, Switch Pro e genéricos físicos por USB e Bluetooth nos dois sistemas.
- [x] Validar build/link de SDL3 no executável GUI x64.
- [ ] Validar entrada de texto e controle físico no Windows 11.
- [ ] Validar mover, clicar, arrastar, rolar e abrir um programa no COSMIC/Wayland com o 8BitDo Ultimate 2 conectado.
- [ ] Validar em Windows, Linux/X11 e outros compositores Wayland.

As etapas completas abaixo só terminam quando seus critérios de validação forem atendidos.

## Etapas

### 1. Fechar a matriz de suporte e escolher a entrada do controle

- [ ] Definir as versões mínimas de Windows e os ambientes Linux a validar: Wayland e X11, incluindo os compositores disponíveis para teste.
- [x] Comparar bibliotecas e APIs nativas: SDL3 escolhida com o usuário para ampliar famílias e perfis, substituindo evdev direto/XInput exclusivo.
- [x] Registrar a decisão e os limites atuais no README; validar fisicamente os modelos da matriz continua pendente.
- [ ] Definir mapeamento inicial, botão para abrir o menu do app, troca de modo e recuperação quando o controle desconectar. Deixar os botões configuráveis; avaliar R3 para abrir o menu Iniciar sem fixá-lo como padrão antes do teste.

**Pronto quando:** o app detecta, identifica e volta a detectar um controle removido/reconectado nos sistemas-alvo.

### 2. Normalizar comandos e controlar a interface do app

- [ ] Criar eventos de domínio independentes da biblioteca: direções, eixos, confirmar, voltar, ombro, clique e botão pressionado/liberado.
- [ ] Tratar deadzone, aceleração, repetição, diagonais e diferença entre toque e pressão longa.
- [x] Conectar o gamepad ao teclado visual, com entrada roteada internamente e sem enviar cliques/setas à janela ativa.
- [ ] Permitir escolher controle, ver conexão e editar mapeamentos sem usar teclado ou mouse.
- [ ] Adicionar pausa/retomada e uma ação de saída sempre acessível pelo controle.

**Pronto quando:** é possível abrir e operar todas as telas do próprio Gamepad Navigator só pelo gamepad, incluindo reconexão.

### 3. Criar os adaptadores nativos de cada sistema

- [ ] Fazer uma prova de conceito de acessibilidade, foco, injeção de texto/ponteiro e enumeração de janelas em Windows, Linux/Wayland e Linux/X11.
- [ ] Comparar as APIs e as bibliotecas Rust disponíveis antes de definir os contratos comuns. Manter detalhes específicos do sistema nos adaptadores.
- [ ] Expor capacidades disponíveis e indisponíveis por sessão, com mensagens de erro úteis e sem travar o restante do app.
- [ ] Documentar permissões, prompts do sistema, dependências de runtime e limitações por compositor.

**Pronto quando:** há um teste mínimo por sistema para ler foco, acionar um elemento acessível e enviar entrada de mouse/teclado, com falhas explicadas.

### 4. Implementar o modo item por item

- [ ] Windows: consultar a árvore e eventos de foco do UI Automation; navegar por nome, tipo, estado, retângulo e ações suportadas.
- [ ] Linux: consultar árvore, foco e ações pelo AT-SPI; testar separadamente em Wayland e X11.
- [ ] Ignorar itens ocultos/desabilitados, manter o item atual visível e oferecer confirmação, voltar, rolar e entrar/sair de grupos.
- [ ] Cobrir menus, listas, caixas de seleção, abas, diálogos e campos de formulário.
- [ ] Criar uma matriz de compatibilidade para navegador, gerenciador de arquivos, configurações e editor de texto.
- [ ] Quando a árvore não existir ou não for útil, oferecer troca para modo mouse e mostrar essa limitação.

**Pronto quando:** os itens acessíveis de cada aplicativo da matriz podem ser focados e acionados sem usar o ponteiro.

### 5. Implementar o modo mouse

- [ ] Mover o ponteiro pelo analógico com deadzone, velocidade ajustável e precisão para movimentos pequenos.
- [ ] Mapear confirmar para clique esquerdo; permitir clique direito, pressionar/soltar, arrastar, rolagem e movimento entre monitores.
- [ ] Implementar uma forma clara de iniciar e encerrar arrasto e de recuperar o ponteiro se o controle for desconectado durante a ação.
- [ ] Implementar entrada por sistema: avaliar `SendInput` no Windows; no Wayland, avaliar o portal Remote Desktop/EIS e seu fluxo de autorização; manter caminho próprio para X11.
- [ ] Validar mover/arrastar janelas e as limitações de cada compositor.

**Pronto quando:** ações comuns com ponteiro funcionam nos três ambientes-alvo e o usuário consegue sair de qualquer estado de clique/arrasto.

### 6. Fazer o teclado próprio digitar no app focado

- [ ] Observar mudanças de foco e detectar campos que aceitam texto pelas APIs de acessibilidade.
- [ ] Abrir/fechar o teclado virtual automaticamente ao entrar/sair de um campo de texto.
- [ ] Manter o campo original como destino enquanto o overlay é controlado; verificar se a janela do teclado rouba foco em cada sistema.
- [ ] Comparar atualização por padrão de acessibilidade com eventos de teclado do sistema, preservando cursor, seleção, composição/IME e acentos.
- [ ] Incluir layout PT-BR, acentos, números/símbolos, maiúsculas, apagar, espaço, confirmar e cancelar.
- [ ] Tratar corretamente senha, conteúdo somente leitura, campo desabilitado e aplicativos sem informação de foco.

**Pronto quando:** escolher teclas pelo gamepad insere o texto no campo correto de cada app da matriz, sem deslocar o foco nem duplicar caracteres.

### Atalhos a avaliar com o usuário

Estas combinações são propostas; ainda não estão implementadas:

- [ ] LT / L2: precisão para apontar em alvos pequenos.
- [ ] LT + LB / L1: Shift+Tab para voltar ao campo anterior.
- [ ] Select / View / −: menu rápido acessível pelo controle, com copiar, colar, volume e ações de janela.
- [ ] No menu rápido: voltar/avançar no navegador, mostrar desktop e maximizar/restaurar.
- [ ] Tornar as combinações configuráveis e mostrar possíveis conflitos antes de salvar.

### 7. Adicionar janelas e ações do sistema

- [ ] Listar janelas e alternar entre elas por gamepad, preservando modo e posição de foco.
- [ ] Oferecer voltar à janela anterior e retornar ao campo de texto após fechar o teclado virtual.
- [ ] Adicionar ações configuráveis para menu Iniciar, alternância de janela, minimizar, maximizar e fechar.
- [ ] Permitir mostrar/ocultar o overlay sem interromper o app que está sendo controlado.
- [ ] Validar comportamento em múltiplos monitores, áreas de trabalho e diálogos modais.

**Pronto quando:** é possível trocar de janela, usar ações do sistema e voltar ao contexto anterior sem mouse/teclado.

### 8. Finalizar a experiência e distribuir

- [ ] Criar configuração de sensibilidade, deadzone, repetição, layouts, mapeamentos e perfis por aplicativo.
- [ ] Mostrar estado do controle, modo ativo, janela-alvo e permissões de entrada sem cobrir conteúdo importante.
- [ ] Criar primeira execução guiada, recuperação de permissões e instruções de compatibilidade.
- [ ] Definir instalação/atualização para Windows e Linux e opção de iniciar com a sessão.
- [ ] Criar CI de Rust para Windows e Linux; validar Linux em Wayland e X11.
- [ ] Fazer testes de integração com controles físicos e atualizar a matriz pública de aplicativos/sistemas.

**Pronto quando:** uma pessoa instala o app, conecta o controle, concede permissões necessárias e conclui o fluxo sem configurar o projeto manualmente.

## Critério de qualidade para cada etapa

- [ ] Seguir RED → GREEN para cada comportamento observável.
- [ ] Rodar `cargo fmt`, `cargo clippy` e testes da etapa.
- [ ] Anotar sistemas, controle físico e aplicativos usados na validação.
- [ ] Atualizar este arquivo e o README ao concluir uma etapa.

## Referências técnicas

- [UI Automation — visão geral e clientes](https://learn.microsoft.com/en-us/windows/win32/winauto/entry-uiautocore-overview)
- [UI Automation — padrões de controle](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-controlpatternsoverview)
- [Windows `SendInput`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput)
- [AT-SPI2](https://wiki.freedesktop.org/www/Accessibility/AT-SPI2/)
- [Portal XDG Remote Desktop](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html)
- [Comportamento atual da extensão: navegação espacial](../extension/lib/spatial.ts)
- [Comportamento atual da extensão: teclado flutuante](../extension/lib/floating-keyboard.ts)
