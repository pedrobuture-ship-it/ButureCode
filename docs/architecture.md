# Arquitetura

Cargo workspace Rust stable, edition 2024, MSRV declarado 1.88. O lockfile fixa dependências. APIs dos domínios independem de egui.

```text
buture-app → eframe, buture-core, buture-ui
buture-ui → egui, buture-settings, buture-workspace, buture-toolchains
buture-settings / buture-workspace / buture-toolchains → biblioteca padrão
```

`buture-app` inicializa a janela e os workers de preferências/host. `buture-core` concentra a identidade. `buture-ui` fornece tokens, Backdrop, GlassSurface, layout, Explorer, onboarding e apresentação de Toolchains.

`buture-workspace` é um actor de filesystem: recebe ações e envia árvores/preview/erros. Apenas diretórios expandidos são listados; watcher por polling a cada dois segundos. A UI virtualiza linhas. Preview limitado a 1 MiB, sem edição. As ações validam confinamento ao workspace e rejeitam links simbólicos. IO ocorre no worker; não há comandos de projeto.

`buture-settings` valida chaves e valores, resolve Default → User → Workspace e persiste INI. Caminho do usuário: XDG_CONFIG_HOME ou ~/.config no Linux, Application Support no macOS, APPDATA no Windows. Workspace: `.buture/settings.ini`. Preferências privadas e conclusão do onboarding só vêm de User. Settings não aceita secrets, comandos ou providers arbitrários. Categorias futuras estão reservadas e não fingem funcionalidades prontas. Arquivo pessoal inválido não é sobrescrito por defaults.

`buture-toolchains` detecta o host e implementa providers, sondagens de versão com limites e planos de instalação assistida. Providers rodam em workers e transmitem resultados; UI não inicia subprocessos diretamente. A execução de instalação exige um plano confirmado e o resultado é verificado. [Detalhes e limitações](toolchains.md).

## Renderização

OpenGL/Glow, X11 ou Wayland no Linux. Windows/macOS suportados pelo toolkit, sem validação nativa neste ambiente. ShellLayout usa pontos lógicos; inspector cede espaço em janelas estreitas. Editor/preview é sólido. Tokens centralizam as cores; GlassSurface centraliza composição, blur e fallback, inclusive o onboarding.

Backdrop interno e cache compartilhado; sem captura de wallpaper do SO ou blur atrás de texto. Repaint por interação e polling periódico de canais (200 ms), necessário para refletir eventos de workers. Não há medição de performance ou promessa de consumo em idle neste marco.

Crates dos prompts seguintes continuam reservados: não há PTY, buffer editável, LSP, Doctor, Git completo, IA ou integrações fictícias. Não há ciclos de crates nem dependências externas novas nos prompts 06–20. Erros de domínio tipados; `unsafe_code` proibido no código autoral.
