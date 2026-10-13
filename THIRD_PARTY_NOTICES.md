# Third-party notices

O código autoral do Buture Code pertence a Pedro Buture. Licenças de dependências não relicenciam o projeto.

O bootstrap do prompt 01 usou somente a biblioteca padrão Rust. Consulte docs/dependencies.md antes de adicionar bibliotecas. Nenhum código do VS Code ou do Zed foi copiado.

A referência visual foi fornecida pelo proprietário como direção de design; não é código ou uma interface a ser reproduzida pixel a pixel.

## UI — prompt 04

- egui / emath / ecolor / epaint / epaint_default_fonts 0.33.3: MIT OR Apache-2.0, https://github.com/emilk/egui . Selecionar Apache-2.0 onde a alternativa estiver disponível.
- Fontes padrão embutidas por egui: preservar os avisos dos pacotes e das fontes incluídas na distribuição.

`Cargo.lock` registra versões exatas transitivas. Antes de distribuir binários, gerar um inventário completo de licenças e incluir os textos/avisos exigidos por cada dependência transitiva, inclusive licenças MIT que não ofereçam alternativa. Esta nota não equivale a uma auditoria completa de distribuição.

## Janela nativa — prompt 05

- eframe / egui-winit / egui_glow 0.33.3: MIT OR Apache-2.0, https://github.com/emilk/egui .
- winit 0.30.13: Apache-2.0, https://github.com/rust-windowing/winit .
- glutin 0.32.3: Apache-2.0, https://github.com/rust-windowing/glutin .
- glow 0.16.0: MIT OR Apache-2.0 OR Zlib, https://github.com/grovesNL/glow .
- png 0.18.1: MIT OR Apache-2.0, https://github.com/image-rs/image-png ; usado diretamente somente no exemplo de captura para QA.

Versões e licenças confirmadas nos manifests dos pacotes baixados. AccessKit está habilitado pelo backend, sem afirmar validação completa de acessibilidade. Avisos transitivos e textos de licenças ainda precisam ser reunidos antes de uma distribuição binária.

## Detecção de projetos — prompt 23

- serde_json 1.0.154: MIT OR Apache-2.0, https://github.com/serde-rs/json .
- toml 1.1.8+spec-1.1.0: MIT OR Apache-2.0, https://github.com/toml-rs/toml .
- ignore 0.4.33: Unlicense OR MIT, https://github.com/BurntSushi/ripgrep/tree/master/crates/ignore .

Preservar os textos e notices dos pacotes, inclusive dependências transitivas do lockfile, na distribuição. O código autoral continua reservado a Pedro Buture.
