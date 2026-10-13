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
