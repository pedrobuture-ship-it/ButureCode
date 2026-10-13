# Arquitetura

Cargo workspace Rust stable, edition 2024, MSRV declarado 1.88. O lockfile fixa a resolução de dependências.

```text
buture-app
├── buture-core (identidade)
├── eframe (janela e backend nativo)
└── buture-ui
    ├── egui (apresentação)
    └── buture-settings (política de efeitos)
```

`buture-app` é a fronteira de aplicação: inicializa eframe, a janela e o loop nativo. `buture-core` concentra identidade sem dependências. `buture-settings` define VisualEffects, independente do toolkit; persistência e settings em camadas ficam para o prompt 10. `buture-ui` fornece tokens, Backdrop, GlassSurface e IdeShell.

## Renderização

Backend OpenGL/Glow, X11 ou Wayland no Linux. eframe documenta suporte nativo a Windows e macOS; esses sistemas ainda não foram executados ou compilados neste ambiente. A integração gráfica está contida na camada de apresentação.

ShellLayout calcula regiões em pontos lógicos. O inspector é opcional e cede espaço quando a janela tem menos de 1150 pontos de largura. Top bar oferece drag, maximização, minimização, fechamento e alternância dos painéis. O editor usa preenchimento sólido explícito. O conteúdo atual é uma prévia somente leitura, não um buffer editável.

Backdrop gera uma textura interna usando tokens e pré-calcula os raios padrão de blur. GlassSurface reutiliza o cache por raio e controla fallback; não há shaders individuais, captura do desktop nem blur atrás do código. O backend repinta por interação; não há timer contínuo no aplicativo principal. O timer de captura pertence somente ao exemplo de QA.

## Limites do marco

Os demais crates reservam os domínios dos prompts posteriores. Não há implementações fictícias de terminal, editor, filesystem explorer, Git, LSP, Doctor ou IA. Nenhum diagnóstico de ferramentas é apresentado como concluído.

APIs de domínio deverão permanecer independentes do toolkit. Adaptadores dependerão de tipos fundamentais, sem ciclos. UI não executará subprocessos, I/O lento ou parsing no loop de renderização. Anyhow, se necessário, fica nas fronteiras; erros de domínio deverão ser tipados.
