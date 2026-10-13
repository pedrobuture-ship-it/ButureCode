# Buture Code

Workstation de desenvolvimento nativa em Rust, criada por **Pedro Buture**.

O Buture Code une código e ambiente de desenvolvimento. Buture Doctor, toolchains e Architecture View são os pilares planejados; ainda não estão implementados.

![Janela nativa do Buture Code](design/previews/main-window.png)

## Executar

Requer Rust stable e um ambiente gráfico. No Linux, o backend usa as bibliotecas de runtime OpenGL/EGL, X11 ou Wayland e xkbcommon disponíveis no sistema. Não há instalação automática de ferramentas.

```sh
cargo run -p buture-app
```

## Marco atual — prompts 01–05

- Cargo workspace com 22 crates e fronteiras independentes de domínio.
- Regras de autoria, desenvolvimento, segurança e validação.
- Design system Buture Glass Dark com tokens centralizados.
- GlassSurface reutilizável: Subtle, Panel, Elevated, Floating e Modal.
- Efeitos Full / Reduced / Off; Off usa superfícies sólidas.
- Janela nativa com top bar, activity bar, sidebar, editor sólido, inspector opcional, painel inferior e status bar.
- Botões Panel / Inspector, modos de efeitos e controles de janela funcionais. Arraste a área livre da top bar para mover; duplo clique para maximizar/restaurar.

Este marco é o **shell visual**. O código de exemplo é somente leitura. Navegação de ferramentas, abertura de projetos, edição, terminal, Git e demais integrações serão implementados nos prompts posteriores. O painel inferior é Output da aplicação, sem PTY ou execução de comandos. As preferências de aparência são apenas da sessão.

O ciclo termina no prompt 05 e aguarda instruções do proprietário.

## Verificação

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

Captura reproduzível do framebuffer nativo para revisão visual (abre uma janela de QA, captura cinco casos e fecha):

```sh
cargo run -p buture-app --example capture_shell -- /tmp/buture-shell-preview
```

A geometria final é determinada pelo compositor. [Resultados e limitações](docs/milestone-05.md).

- [Arquitetura](docs/architecture.md)
- [Roadmap](docs/roadmap.md)
- [Design system](docs/design-system.md)
- [Segurança](docs/security.md)
- [Seleção de dependências](docs/dependencies.md)
- [Dependências e licenças](THIRD_PARTY_NOTICES.md)

## Direitos

Copyright © 2026 Pedro Buture. Todos os direitos reservados. Nenhuma licença de código aberto foi concedida ao código autoral deste projeto. As dependências mantêm suas próprias licenças.
