# Roadmap do Gamepad Desktop

O objetivo é operar o computador pelo gamepad em Windows e Linux: apontar, clicar, rolar, digitar e alternar entre tarefas e jogos. Este é o planejamento central do desktop; o [guia de uso](../README.md) descreve a experiência atual.

## Implementado

- [x] Aplicativo Rust independente da extensão, com leitura de controles por SDL3.
- [x] Seleção de dispositivo e importação persistente de perfis SDL.
- [x] Movimento, clique, arrasto, rolagem, Enter, Escape, Tab e lançador do sistema.
- [x] Precisão manual ajustável: segurar LT / L2 usa uma velocidade configurável de 5% a 50% (padrão 25%), com controle nas configurações e apresentação no onboarding.
- [x] Localizador temporário do ponteiro: LT / L2 + R3 amplia o cursor do sistema por um segundo quando a plataforma oferece escala de cursor, com demonstração local no onboarding.
- [x] Modos Desktop/Jogo por dois toques em Select; pausa por Start + Select no Desktop.
- [x] Seletor Alt+Tab, captura do sistema e atalho de tela cheia.
- [x] Teclado de toque do Windows e integração com `cosmic-osk` quando disponível.
- [x] Teclado próprio no Linux, com posição, largura, opacidade e navegação ajustáveis.
- [x] Onboarding com cinco grupos principais, demonstrações locais e lições extras opcionais.
- [x] Painel em três idiomas, escala de interface, preferências salvas e início com a sessão.
- [x] Bandeja, notificações, versão visível e encerramento aguardando a limpeza do runtime.
- [x] Cursor do sistema mantido no tamanho atual sem preferência; escala aplicada após ajuste e restaurada no encerramento.
- [x] Builds x64 Windows/Linux, pacotes `.deb`/`.tar.gz`, CI e atualizador no painel.
- [x] Testes da lógica de entrada, isolamento do onboarding, percursos, persistência e layout.

“Implementado” descreve o código disponível. A validação física do escopo atual foi concluída; o backlog abaixo fica focado em features e correções de bugs.

## Features planejadas

- [ ] Assistente visual para mapear e calibrar controles desconhecidos sem importar uma linha de texto.
- [ ] Permitir concluir configuração, reconexão e recuperação usando apenas o controle.
- [ ] Ampliar o suporte do teclado próprio a layouts e acentos do Linux.
- [ ] Oferecer um teclado visual compatível com GNOME Wayland.
- [ ] Abrir o teclado visual ao focar campos, preservando o foco e a entrada do aplicativo.

## Correções de bugs

Este roadmap ainda não registra bugs pendentes. Adicione aqui problemas reproduzíveis, com passos para reproduzir, resultado esperado e resultado observado.

## Ideias a avaliar

Estes recursos ainda não são compromissos de implementação:

- Menu rápido com copiar/colar, volume e ações de janela.
- Personalização de combinações e perfis por aplicativo.

Navegação por elementos acessíveis fica fora do escopo atual. O desktop usa ponteiro e atalhos; a navegação espacial da extensão pertence ao [projeto de navegador](../../extension/README.md).

## Critério de conclusão

Features devem descrever o comportamento esperado e ter a documentação atualizada. Correções de bugs devem incluir passos reproduzíveis e uma verificação contra regressões. Em ambos os casos, formatação e lint precisam passar.

Consulte [desenvolvimento](development.md) para comandos e [visão do sistema](system-overview.md) para os fluxos atuais.
