# Marco 35 — revisão dos prompts 21–35

Pedro Buture autorizou ampliar o ciclo do prompt 20 até keybindings. Implementação em sequência, com commits por prompt, preservando a identidade Git configurada e os direitos do projeto. Prompts 36–95 permanecem fora deste ciclo.

## Resultado

Doctor com evidências/soluções; detecção de projetos e Overview; buffer Rope editável; canvas nativo com Buture Emerald, tabs e splits; parsing incremental e folding; operações avançadas; busca/substituição revisada; Palette; Quick Open; defaults de atalhos por SO, overrides, conflitos e chords.

Revisão de segurança protege alterações locais durante reload, mantém metadados dos tabs, impede descarte durante saves pendentes e intercepta fechamento nativo com mudanças/saves em andamento. Save temporário não remove um arquivo preexistente que não criou. Doctor não declara prontidão para uma linguagem sem evidência de provider. Perfil de atalhos inválido/externamente alterado é preservado.

## Verificação

Linux x86_64, Rust stable 1.99.0:

- `cargo fmt --all --check`: passou.
- `cargo clippy --workspace --all-targets -- -D warnings`: passou.
- `cargo test --workspace`: 44 testes passaram, zero falhas.
- `cargo build --workspace`: passou.
- Benchmark otimizado 10 mil/100 mil/1 milhão de linhas: [medidas e limites](performance.md).

Testes cobrem Unicode/CRLF e undo transacional, ABI dos 13 parsers/variantes, equivalência de parse incremental e novo, virtualização de linhas, tabs/splits, ignore/regex/conflitos externos, preservação de reloads, defaults/overrides/chords e cancelamento de close durante save. Não substituir esses testes por alegações de validação de uso completo.

Capturas da janela nativa Linux foram inspecionadas ao longo dos prompts 22–34. A tentativa final pela sessão Wayland ficou limitada por sessão bloqueada e monitor desligado. O backend X11 permitiu captura real do framebuffer; 13 frames finais capturados (Full/Reduced/Off, compact, editor-only, Explorer, editor, tabs, split, busca, Palette, Quick Open e shortcuts). Inspeção visual confirmou popups/resultados e a tela de atalhos; capturas selecionadas estão em `design/previews`. Nenhuma ferramenta foi instalada durante QA; exemplos não gravam preferências pessoais. Geometria física depende do compositor (1920×1080; compact variou para 941×1030).

## Limites de entrega

- Windows/macOS não executados; defaults e APIs portáveis não equivalem a validação nativa nesses hosts.
- Não há PTY/task runner/LSP/Git completo/temas adicionais. Commands futuros desabilitados; build/test do projeto não executados.
- Canvas virtualiza linhas verticais; linhas gigantescas, graphemes, IME/preedit e acessibilidade exigem trabalho adicional.
- Session persiste layout/caminhos, não backup de conteúdo não salvo; fechamento pede decisão explícita.
- Save compara disco antes de escrever, sem lock contra escritores concorrentes. Workspace Replace All pode terminar parcialmente após erro de IO; resultado informa arquivos aplicados.
- Index/manifest/regex/parsing têm limites e podem produzir resultados parciais informados. Highlights não são semântica de compilador.

Detalhes: [editor](editor.md), [atalhos](keybindings.md), [Doctor](doctor.md), [projetos](project.md), [dependências](dependencies.md), [segurança](security.md).
