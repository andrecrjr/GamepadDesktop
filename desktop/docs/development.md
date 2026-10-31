# Desenvolvimento do desktop

O aplicativo Rust em `desktop/` é independente da [extensão de navegador](../../extension/README.md). Veja a [visão do sistema](system-overview.md) para o fluxo de entrada e as diferenças de plataforma.

## Executar localmente

Instale Rust, CMake e um compilador C/C++. No Linux, a compilação também usa as bibliotecas de desenvolvimento listadas no [workflow desktop](../../.github/workflows/desktop.yml). Em Ubuntu/Debian, a lista usada pelo CI é:

```sh
sudo apt-get install build-essential cmake dpkg-dev pkg-config libasound2-dev libudev-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev libxinerama-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libegl1-mesa-dev libdbus-1-dev libatspi2.0-dev
```

Para os comandos de formatação e lint, instale os componentes do Rust:

```sh
rustup component add rustfmt clippy
```

Na raiz do repositório:

```sh
make desktop-run
```

A compilação padrão incorpora SDL3 a partir do código-fonte. Com SDL3 instalada no sistema, `cargo run --manifest-path desktop/Cargo.toml --no-default-features` usa essa instalação.

Executar o programa no Linux também exige acesso ao controle e a `/dev/uinput`. Instalar dependências de compilação não concede essas permissões.

## Qualidade

```sh
make desktop-format
make desktop-check
make desktop-lint
make desktop-test
```

Clippy trata avisos como erros. Testes com controle físico, dispositivos virtuais e compositor são optativos e exigem os recursos indicados em cada teste. Passar no teste automatizado ou no build não confirma o comportamento em um Windows ou compositor real.

## Build e pacotes

| Comando na raiz              | Resultado                                                           |
| ---------------------------- | ------------------------------------------------------------------- |
| `make desktop-release`       | Executável release da plataforma atual em `desktop/target/release/` |
| `make desktop-linux-package` | Arquivos Linux x64 `.deb` e `.tar.gz` em `desktop/dist/`            |
| `make desktop-windows`       | Executável Windows x64 em `desktop/dist/`                           |
| `make desktop-build`         | Pacotes Linux e executável Windows                                  |

Para gerar `.deb`, instale o empacotador usado pelo projeto:

```sh
cargo install cargo-deb --version 3.8.0 --locked
```

O build Windows a partir do Linux usa o alvo `x86_64-pc-windows-gnu`, MinGW e CMake:

```sh
rustup target add x86_64-pc-windows-gnu
make desktop-windows
```

O Makefile permite configurar `MINGW_BIN` e remove símbolos do executável distribuído com `x86_64-w64-mingw32-strip`. Isso reduz o arquivo, com menos informação disponível para depuração. O workflow Windows do CI faz o build no próprio Windows.

## Prévia do teclado

No Linux, renderize o teclado compartilhado sem precisar de compositor ou controle físico:

```sh
make desktop-preview
```

O exemplo salva prévias PPM em `/tmp/gamepad-keyboard-preview`. Elas representam o teclado próprio do app, não o teclado nativo do Windows ou COSMIC.

O [benchmark do overlay](overlay-performance.md) documenta a medição e sua reprodução. Os registros brutos acompanham o relatório.

## Assets da marca

A origem da marca é [`assets/gamepad-desktop.svg`](../../assets/gamepad-desktop.svg). Para atualizar as cópias do desktop, site e extensão:

```sh
make web-install
make brand-assets
npm --prefix extension run brand:check
```

As cópias geradas são versionadas para o build Rust funcionar sem Node. O ICO inclui vários tamanhos para o Windows. Os SVGs de botões do controle são assets de instrução separados; os créditos estão em [ASSET_CREDITS.md](../../extension/ASSET_CREDITS.md).

## Releases

Commits convencionais em `main` alimentam o semantic-release. As releases usam tags `desktop-v*` e anexam o executável Windows x64, os pacotes Linux `.deb`/`.tar.gz` e os metadados de revisão usados pelo atualizador. A extensão tem validação e empacotamento próprios.

A configuração está no [workflow desktop](../../.github/workflows/desktop.yml) e em [`.releaserc.json`](../../.releaserc.json).
