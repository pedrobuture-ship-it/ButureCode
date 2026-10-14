# Arquitetura

Cargo workspace Rust stable, edition 2024, MSRV declarado 1.88. O lockfile fixa dependências. APIs dos domínios independem de egui.

```text
buture-app → eframe, buture-core, buture-ui
buture-ui → settings, workspace, toolchains, doctor, project, editor, search, core
buture-ui → terminal, tasks, lsp, format, git
buture-terminal → portable-pty, vt100
buture-tasks → project, serde_json
buture-lsp → protocol, editor, serde_json
buture-format → editor, toolchains, toml
buture-git → toolchains
buture-doctor → toolchains
buture-project → serde_json, toml, ignore
buture-editor → ropey, tree-sitter/grammars, serde_json
buture-search → editor, core, regex, ignore
buture-settings → core
```

`buture-app` inicializa a janela e os workers de preferências/host. `buture-core` concentra identidade, comandos, fuzzy matching e keybindings. `buture-ui` fornece tokens, Backdrop, GlassSurface, layout, Explorer, onboarding e apresentação de Toolchains.

`buture-workspace` é um actor de filesystem: recebe ações e envia árvores/preview/erros. Apenas diretórios expandidos são listados; watcher por polling a cada dois segundos. A UI virtualiza linhas. Preview de filesystem limitado a 1 MiB; abertura editável é delegada a buture-editor. As ações validam confinamento ao workspace e rejeitam links simbólicos. IO ocorre no worker; não há comandos de projeto.

`buture-settings` valida chaves e valores, resolve Default → User → Workspace e persiste INI. Caminho do usuário: XDG_CONFIG_HOME ou ~/.config no Linux, Application Support no macOS, APPDATA no Windows. Workspace: `.buture/settings.ini`. Preferências privadas e conclusão do onboarding só vêm de User. Settings não aceita secrets, comandos ou providers arbitrários. Categorias futuras estão reservadas e não fingem funcionalidades prontas. Arquivo pessoal inválido não é sobrescrito por defaults.

`buture-toolchains` detecta o host e implementa providers, sondagens de versão com limites e planos de instalação assistida. Providers rodam em workers e transmitem resultados; UI não inicia subprocessos diretamente. A execução de instalação exige um plano confirmado e o resultado é verificado. [Detalhes e limitações](toolchains.md).

## Renderização

OpenGL/Glow, X11 ou Wayland no Linux. Windows/macOS suportados pelo toolkit, sem validação nativa neste ambiente. ShellLayout usa pontos lógicos; inspector cede espaço em janelas estreitas. Editor/preview é sólido. Tokens centralizam as cores; GlassSurface centraliza composição, blur e fallback, inclusive o onboarding.

Backdrop interno e cache compartilhado; sem captura de wallpaper do SO ou blur atrás de texto. Repaint por interação e polling periódico de canais (200 ms), necessário para refletir eventos de workers. Há um benchmark isolado de buffer em [performance](performance.md); consumo em idle e startup não foram medidos.

`buture-doctor` agrega evidências e diagnósticos; `buture-project` detecta manifests/monorepos; `buture-editor` controla Rope, IO, transações, sessão e parsing incremental; `buture-search` controla consultas, índice e substituições revisadas. Workers coalescem snapshots e rejeitam resultados obsoletos. Views e grupos da UI compartilham buffers, mantendo seleção/scroll próprios. Veja [editor](editor.md).

Terminal, tasks, LSP, formatação e Git executam trabalho em workers, com filas e limites de output. LSP usa transporte stdio local em buture-protocol; alterações recebidas têm revisão e proteção contra versões obsoletas. Source Control consulta Git por argv, sem executar comandos via shell. Veja os documentos específicos para os limites de cada domínio.

O código ativo vai até o prompt 44; diff (45) está preservado em patch de retomada. Não há debugger, Memory Inspector, IA ou extensões implementados neste marco. Erros de domínio tipados; `unsafe_code` proibido no código autoral.
