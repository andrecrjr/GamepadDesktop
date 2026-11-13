# Roadmap do Gamepad Desktop

O objetivo é operar o computador pelo gamepad em Windows e Linux: apontar, clicar, rolar, digitar e alternar entre tarefas e jogos. Este é o planejamento central do desktop; o [guia de uso](../README.md) descreve a experiência atual.

## Implementado

- [x] Aplicativo Rust independente da extensão, com leitura de controles por SDL3.
- [x] Seleção de dispositivo e importação persistente de perfis SDL.
- [x] Movimento, clique, arrasto, rolagem, Enter, Escape, Tab e lançador do sistema.
- [x] Modos Desktop/Jogo por dois toques em Select; pausa por Start + Select no Desktop.
- [x] Seletor Alt+Tab, captura do sistema e atalho de tela cheia.
- [x] Teclado de toque do Windows e integração com `cosmic-osk` quando disponível.
- [x] Teclado próprio no Linux, com posição, largura, opacidade e navegação ajustáveis.
- [x] Onboarding com cinco grupos principais, demonstrações locais e lições extras opcionais.
- [x] Painel em três idiomas, escala de interface, preferências salvas e início com a sessão.
- [x] Bandeja, notificações, versão visível e encerramento aguardando a limpeza do runtime.
- [x] Cursor com escala inicial de 100% e restauração após personalização.
- [x] Builds x64 Windows/Linux, pacotes `.deb`/`.tar.gz`, CI e atualizador no painel.
- [x] Testes da lógica de entrada, isolamento do onboarding, percursos, persistência e layout.

“Implementado” descreve o código disponível. Não substitui a validação física de cada combinação abaixo.

## Prioridade: validar a experiência real

| Área                       | O que falta confirmar                                                                                                 |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| Windows                    | Mouse, arrasto, rolagem, foco, teclado e bandeja em aplicativos reais                                                 |
| Teclado do Windows         | Abrir, digitar e fechar com R3 nos layouts compacto e grande; testar a alternativa quando o layout Gamepad é recusado |
| COSMIC/Wayland             | Entrada, foco, teclado nativo, reconexão e mudanças externas de visibilidade                                          |
| Linux/X11 e outros Wayland | Teclado próprio, foco, permissões, múltiplos monitores e limites do compositor                                        |
| Controles                  | Xbox, DualShock/DualSense, Switch Pro e genéricos físicos, por USB e Bluetooth                                        |
| Atualizações               | Instalação e reinício nos caminhos Windows, Linux `.deb` e `.tar.gz`                                                  |
| Encerramento               | Restauração de cursores e liberação de entradas durante erros e desconexões                                           |
| Onboarding                 | Leitura à distância, escalas maiores, prática completa e compreensão do percurso opcional                             |

Registre sistema/versão, ambiente gráfico, modelo do controle, tipo de conexão, aplicativo e resultado. Publique uma matriz de suporte com base nesses registros.

## Melhorias seguintes

- [ ] Assistente visual para mapear e calibrar controles desconhecidos sem importar uma linha de texto.
- [ ] Verificar que configuração, reconexão e recuperação podem ser concluídas apenas pelo controle.
- [ ] Ampliar o suporte do teclado próprio a layouts e acentos do Linux.
- [ ] Investigar uma alternativa de teclado para GNOME Wayland.
- [ ] Avaliar abertura automática ao focar campos sem comprometer o foco ou a entrada do aplicativo.
- [ ] Definir versões mínimas e requisitos de instalação a partir da matriz validada.

## Ideias a avaliar

Estes recursos ainda não são compromissos de implementação:

- Precisão temporária com gatilho para apontar em alvos pequenos.
- Menu rápido com copiar/colar, volume e ações de janela.
- Personalização de combinações e perfis por aplicativo.

Navegação por elementos acessíveis fica fora do escopo atual. O desktop usa ponteiro e atalhos; a navegação espacial da extensão pertence ao [projeto de navegador](../../extension/README.md).

## Critério para concluir uma melhoria

O comportamento precisa de verificação adequada, formatação, lint e documentação atualizada. Para mudanças de plataforma, o registro deve distinguir o que passou em testes automatizados do que foi observado em um sistema real.

Consulte [desenvolvimento](development.md) para comandos e [visão do sistema](system-overview.md) para os fluxos atuais.
