# Buture Code

Workstation de desenvolvimento nativa em Rust, criada por **Pedro Buture**.

![Buture Code](design/previews/main-window.png)

## Executar

Requer Rust stable e ambiente gráfico. No Linux, o backend usa OpenGL/EGL, X11 ou Wayland e xkbcommon do sistema. Não há instalação automática.

```sh
cargo run -p buture-app
```

## Marco atual — prompts 01–44

Buture Glass Dark com tokens e GlassSurface centralizados, modos Full / Reduced / Off e editor em superfície sólida. Doctor apresenta evidências e soluções; Project Overview detecta manifests e monorepos sem executar scripts.

Editor nativo com buffer Rope UTF-8, undo/redo, edição, salvamento com proteção contra mudanças externas, tabs/pins/preview, splits e sessão de layout. Tree-sitter fornece syntax highlighting e folding em segundo plano. Edição inclui multicursor, ocorrências, indentação, comentários, pares automáticos, duplicação e movimentação de linhas.

Busca em documento/workspace com regex, case/whole word, globs e substituição revisada. Command Palette e Quick Open usam busca fuzzy. Atalhos têm defaults por SO, overrides pessoais, conflitos e chords de duas teclas. Settings → Keyboard Shortcuts abre o perfil.

Open Folder: Ctrl/Cmd+O; Open File: Ctrl/Cmd+Shift+O; Quick Open: Ctrl/Cmd+P; Palette: Ctrl/Cmd+Shift+P. Toolchains e instalação assistida mantêm revisão e confirmação explícitas.

Terminal PTY com tabs/split, task runner e Smart Run com revisão antes da execução. Cliente LSP por stdio, recursos de linguagem, formatters e Problems Panel usam resultados reais de ferramentas. Source Control oferece status, stage/unstage, branches e operações Git com confiança e confirmação.

A implementação foi pausada após o prompt **44**, por pedido de Pedro. O prompt **45** tem trabalho parcial preservado em patch; os prompts 46–50 permanecem pendentes. [Arquivo para retomar no prompt 45](RETOMADA-PROMPT-45.md). [Resultados e limitações do marco 44](docs/milestone-44.md).

[Resultados e limitações do marco 35](docs/milestone-35.md). [Marco 20 histórico](docs/milestone-20.md).

## Verificação

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
cargo run -p buture-app --example capture_shell -- /tmp/buture-shell-preview
```

O exemplo abre uma janela nativa, captura os modos de efeitos, editor, splits, busca, Palette, Quick Open, shortcuts, Doctor, Toolchains e onboarding, e encerra. A varredura de QA consulta versões reais; não instala ferramentas nem grava preferências pessoais. A geometria é determinada pelo compositor.

- [Terminal](docs/terminal.md)
- [Tasks e Smart Run](docs/tasks.md)
- [Linguagens e LSP](docs/language.md)
- [Formatters](docs/formatting.md)
- [Problems](docs/problems.md)
- [Git e Source Control](docs/git.md)
- [Arquitetura](docs/architecture.md)
- [Roadmap](docs/roadmap.md)
- [Design system](docs/design-system.md)
- [Toolchains e instalação assistida](docs/toolchains.md)
- [Editor](docs/editor.md)
- [Atalhos](docs/keybindings.md)
- [Doctor](docs/doctor.md)
- [Projetos](docs/project.md)
- [Performance](docs/performance.md)
- [Segurança](docs/security.md)
- [Dependências](docs/dependencies.md)
- [Third-party notices](THIRD_PARTY_NOTICES.md)

## Direitos

Copyright © 2026 Pedro Buture. Todos os direitos reservados. Nenhuma licença de código aberto foi concedida ao código autoral deste projeto. As dependências mantêm suas próprias licenças.
