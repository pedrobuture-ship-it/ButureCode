# Third-party notices

O código autoral do Buture Code pertence a Pedro Buture. Licenças de dependências não relicenciam o projeto.

O bootstrap usa somente a biblioteca padrão Rust. Consulte docs/dependencies.md antes de adicionar bibliotecas. Nenhum código do VS Code ou do Zed foi copiado.

A referência visual foi fornecida pelo proprietário como direção de design; não é código ou uma interface a ser reproduzida pixel a pixel.

## UI — prompt 04

- egui / emath / ecolor / epaint / epaint_default_fonts 0.33.3: MIT OR Apache-2.0, https://github.com/emilk/egui . Selecionar Apache-2.0 onde a alternativa estiver disponível.
- Fontes padrão embutidas por egui: preservar os avisos dos pacotes e das fontes incluídas na distribuição.

`Cargo.lock` registra versões exatas transitivas. Antes de distribuir binários, gerar um inventário completo de licenças e incluir os textos/avisos exigidos por cada dependência transitiva, inclusive licenças MIT que não ofereçam alternativa. Esta nota não equivale a uma auditoria completa de distribuição.
