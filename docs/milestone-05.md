# Marco 05 — janela principal

Data: 13 de outubro de 2026. Autoria: Pedro Buture.

## Entregue

Prompts 01–05: workspace com 22 crates, regras do repositório, Buture Glass Dark, GlassSurface e janela nativa. Sete regiões do shell presentes; inspector e painel inferior opcionais. A referência visual foi analisada antes da UI; o resultado usa componentes próprios.

A prévia de código é somente leitura. Activity Bar e sidebar são a estrutura visual, sem funcionalidades dos prompts 06–09. Não há filesystem explorer, PTY, editor real, LSP, toolchains, Doctor, IA ou telemetria.

## Validação

Rust stable 1.99.0, Linux x86_64. `cargo fmt --all --check`, Clippy com warnings como erros, testes do workspace e build passaram. Cinco testes de UI verificam o blur gaussiano, fallbacks Off e Reduced, geometria das regiões e tesselação do shell nos três modos de efeitos.

Execução nativa usando eframe/Glow e Wayland: capturas de Full, Reduced, Off e painéis ocultos em 1920×1080. Caso compacto em 941×1030: a geometria foi imposta pelo compositor de janelas, em vez dos 960×640 solicitados. Layouts 960×640, 1280×720, 1440×900 e 1920×1080 também foram verificados por testes de geometria; isso não equivale a captura nativa em cada resolução.

Inspeção visual: editor opaco, alinhamento das regiões, inspector visível em largura suficiente, ausência de inspector na janela estreita, painel inferior ocultável e legibilidade do modo Off. Símbolos sem cobertura de fonte foram substituídos por geometria vetorial ou texto ASCII. O exemplo de QA encerra após as cinco capturas.

A captura Full está em `design/previews/main-window.png`. Reprodução:

```sh
cargo run -p buture-app --example capture_shell -- /tmp/buture-shell-preview
```

## Limitações

- Windows e macOS ainda não compilados ou executados nesta sessão.
- Blur do backdrop interno, sem transparência ou blur de wallpaper do SO.
- Aparência não persistida; seleção de efeitos vive somente nesta sessão.
- Ainda não há medição de performance nem auditoria completa de licenças para distribuição binária.

Aguardar ordens do proprietário antes do prompt 06.
