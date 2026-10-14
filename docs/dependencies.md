# Seleção de dependências

Bootstrap: biblioteca padrão Rust, sem dependências externas.

## Toolkit nativo

Avaliação realizada antes da UI:

- **egui / eframe**: toolkit Rust mantido, crates versionados e documentação pública; backend nativo em Linux, Windows e macOS. Licença dual MIT OR Apache-2.0; selecionar Apache-2.0 na distribuição quando houver essa alternativa. Renderização imediata com repaints por interação, sem DOM/WebView. Adequado ao shell inicial; editor futuro precisará de virtualização e medições próprias. Não promete acessibilidade completa em todos os sistemas.
- **GPUI**: toolkit Rust GPU com suporte a plataformas documentado pelo projeto. Avaliado, mas não selecionado neste marco devido ao acoplamento mais próximo ao ecossistema Zed e maior complexidade de integração/plataforma. Não copiar fontes do Zed.
- **Tokio, Serde, Tree-sitter, ropey, notify, tracing, thiserror**: preferências para marcos que realmente precisem de async, persistência, parsing, buffers, watchers, logging e erros. Adicionar somente quando usados; nenhum pacote antecipado sem função neste ciclo.

Fontes primárias: https://github.com/emilk/egui e https://docs.rs/eframe ; GPUI: https://github.com/zed-industries/zed/tree/main/crates/gpui . A licença do projeto não muda por usar dependências permissivas.

Versões selecionadas: egui e eframe **0.33.3**, fixadas na mesma versão, compatíveis com o MSRV declarado 1.88. O toolkit mais novo não é necessário para este shell; upgrades exigem revisão de API, MSRV e renderização. egui entra no prompt 04, eframe somente no prompt 05. eframe usará OpenGL/Glow com X11 e Wayland para reduzir o conjunto de dependências do primeiro marco; sem WebView, persistência ou telemetria.

## Explorer (prompt 09)

Biblioteca padrão: actor em thread, canais e watcher por polling de dois segundos apenas nas pastas expandidas. Evita dependência de watcher nativo neste ciclo e funciona nos três hosts; não detecta instantaneamente mudanças. Diretórios não expandidos não são lidos. A lista visível é virtualizada. Preview limitado a 1 MiB de UTF-8; não é editor. Cópias de diretórios são feitas no worker e podem deixar cópia parcial se ocorrer erro de IO. Links simbólicos não são percorridos nem modificados.

## Prompts 10–20

Sem dependências externas adicionais. Biblioteca padrão fornece INI validado por schema fechado, camadas de configuração, canais/threads, detecção de diretórios e sondagens de subprocessos. Escolha deliberada para não adicionar um runtime async ou serialização genérica para oito preferências tipadas. Ferramentas e gerenciadores de pacotes detectados são executáveis externos, não dependências linkadas nem redistribuídas; seus termos são próprios. Sondagens limitadas e workers evitam bloqueio do loop de apresentação. Expansão de schema/formatos e watcher nativo exigirá nova avaliação de dependências.

## Avaliação antes dos prompts 23–35

Fontes primárias consultadas: documentação e metadados crates.io dos upstreams. As escolhas não alteram os direitos do Buture Code. As dependências preservam suas próprias notices.

- `serde_json` 1.0.154 e `toml` 1.1.8 : parsing real dos manifests, bibliotecas mantidas, MIT OR Apache-2.0 (preferência Apache-2.0). MSRV 1.71 / 1.85, compatíveis Rust 1.88, portáveis. Sem executar conteúdo de manifests.
- `ignore` 0.4.33 : caminhamento recursivo, globs e regras gitignore do ripgrep, mantido e multiplataforma, Unlicense OR MIT, MSRV 1.88. Evita um motor ignore artesanal. Caminhamento limitado e cancelável para monorepos.
- `ropey` 1.6.1 : rope UTF-8 Rust maduro, licença MIT OR Apache-2.0 (preferência Apache-2.0), MSRV 1.65. Inserção/remoção e índices de linhas escaláveis; snapshots compartilham estrutura para undo e workers. Medir workloads do projeto sem prometer performance não medida.
- `regex` 1.13.1 : motor Rust mantido, MIT OR Apache-2.0, MSRV 1.65, sem backtracking exponencial. Compilação no worker, limites de tamanho e resultados em streaming.
- Tree-sitter : runtime C com binding Rust e parsers upstream; MIT para runtime/gramáticas, compatível Windows/Linux/macOS mas exige compilador C no build. A versão 0.27.1 exige Rust 1.90; selecionar 0.25.x compatível com MSRV 1.88 e validar ABI/linguagens. Gramáticas Rust/C/C++/Python/Java/JavaScript/TypeScript/Go/JSON e tree-sitter-grammars TOML/YAML/Markdown; versões fixadas e testes de cada parser. Sem código copiado de VS Code ou Zed. `unsafe` fica nas dependências; nenhum FFI unsafe no código autoral.

Fontes: https://docs.rs/ropey/1.6.1/ropey/ , https://tree-sitter.github.io/tree-sitter/using-parsers/ , https://docs.rs/ignore/0.4.33/ignore/ , https://github.com/rust-lang/regex , https://github.com/serde-rs/json e https://github.com/toml-rs/toml .

Tree-sitter selecionado: **0.25.10**, MSRV upstream 1.76. Gramáticas fixadas na tabela de `THIRD_PARTY_NOTICES.md`; todos os parsers passam por teste de carga/ABI e parsing. Highlighting usa classificação própria de nós e tokens; não reutiliza código de produtos. O editor não executa comandos de grammars do workspace.

Ropey usa `default-features = false` e `simd`: linhas por LF, com CRLF preservado no conteúdo. Isso alinha coordenadas com Tree-sitter; outros separadores Unicode permanecem caracteres do texto. UTF-8 é preservado, sem prometer navegação por graphemes neste marco.

## Prompts 36–50 — avaliação antes da implementação

`portable-pty` 0.9.0 (MIT, upstream WezTerm) fornece PTY Linux/macOS e ConPTY Windows por API Rust segura; manutenção e portabilidade avaliadas na documentação upstream. `vt100` 0.16.2 (MIT, upstream maintained terminal parser) fornece estado ANSI, cores, alternate screen e scrollback limitado. Evita emulação ANSI artesanal; não copiar código de produtos. As dependências têm unsafe interno; código autoral usa somente a API segura. Validar integração nativa e registrar plataformas não executadas. Fontes: https://docs.rs/portable-pty/0.9.0/portable_pty/ e https://docs.rs/vt100/0.16.2/vt100/ .

LSP/DAP usarão framing limitado de mensagens e `serde_json` já avaliado, conforme especificações primárias https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/ e https://microsoft.github.io/debug-adapter-protocol/specification . Git usará executable oficial por argumentos separados, evitando dependência libgit2 e duplicação da implementação do Git. Execução de projeto exige confiança explícita na UI; servidor/adapter não é iniciado automaticamente.

Formatter registry (41): subprocessos locais de rustfmt, clang-format, Ruff, Black, Prettier e gofmt. Mantidos pelos upstreams; flags de stdin/stdout avaliadas nas documentações oficiais, sem redistribuir executables. `toml` reutiliza a versão/licença já avaliada para inferir edition Cargo. Execução revisada no workspace pode carregar configuração/plugins; confiança explícita, sem install automático ou flags de write-in-place.
