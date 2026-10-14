# Retomada — começar no prompt 45

Registro em 14/10/2026. Pedro pediu para documentar o avanço e parar para continuar no prompt 45. Não executar os próximos prompts apenas por abrir este arquivo.

## Estado entregue

Código ativo concluído até o **prompt 44**, commit de implementação `a4095a2`. Os prompts 36–44 têm commits individuais e passaram por fmt, clippy, testes e build antes de cada commit. O [marco 44](docs/milestone-44.md) descreve funcionalidades e limites. Os prompts 45–50 não estão concluídos.

O trabalho parcial do 45 foi preservado em [docs/handoff/prompt-45-wip.patch](docs/handoff/prompt-45-wip.patch). Contém sete arquivos, inclusive os novos módulos de diff Git e UI. Foi retirado da árvore de código ativa para deixar a base validada do 44. Nenhuma alteração do proprietário foi descartada.

## Como começar amanhã

1. Ler `AGENTS.md`, o prompt 45 de `Buture Code — Prompts completos do zero.md` e este registro.
2. Inspecionar `design/reference/buture-code-glass.png` antes de mudar UI. Preservar tokens, GlassSurface e superfícies sólidas de texto.
3. Verificar `git status --short`; preservar quaisquer novas alterações do proprietário.
4. Conferir e aplicar o patch, a partir da raiz do repositório:

```sh
git apply --check docs/handoff/prompt-45-wip.patch
git apply docs/handoff/prompt-45-wip.patch
```

O patch foi conferido com `git apply --check` sobre a base 44. Não reaplicar se o trabalho já tiver sido restaurado. Se a base mudar, revisar conflitos sem descartar trabalho local.

## O que já está esboçado no 45

- Parsing de unified diff, hunks, linhas e indicação de diff binário.
- Planos de stage/unstage hunk, rechecando HEAD/diff antes de mudar somente o index.
- Plano de discard de arquivo rastreado, com comparação do arquivo e index antes da operação.
- Viewer inline e side-by-side; abertura pelo Source Control e diálogo de descarte.
- Caso de captura nativa e testes novos, ainda sem validação completa.

**Não tratar o patch como pronto.** A última execução de clippy falhou por `clippy::question_mark` em `crates/buture-ui/src/diff_view.rs`, perto da linha 124. Trocar o `let Some(diff) = self.diff.clone() else { return None; };` por `let diff = self.diff.clone()?;` e verificar novas falhas. A suíte completa e a inspeção nativa do 45 não foram concluídas.

## O que falta para concluir o 45

- Revisar requisitos originais: inline, side-by-side, stage hunk, unstage hunk e discard com confirmação.
- Conferir alinhamento de blocos no side-by-side: o esboço distribui linhas removidas/adicionadas em linhas separadas.
- Bloquear stage hunk quando o buffer correspondente estiver dirty, de forma consistente com o stage de arquivo do prompt 44.
- Revisar resultados obsoletos, mudança de root/trust, limites de diff, binários, CRLF, renames e ausência de newline.
- Verificar que discard não toca arquivo com alterações não salvas e mantém confirmação concreta. O esboço não descarta arquivos untracked.
- Executar testes de Git em repos temporários, sem alterar o repositório real para QA.
- Ajustar a captura do caso diff para carregar uma vez quando o status estiver pronto; inspecionar a janela nativa.
- Documentar limites reais e somente então criar o commit `[feat] add Git diff editor`.

Antes de commitar:

```sh
/home/buture/.cargo/bin/cargo +stable fmt --all --check
/home/buture/.cargo/bin/cargo +stable clippy --workspace --all-targets -- -D warnings
/home/buture/.cargo/bin/cargo +stable test --workspace
/home/buture/.cargo/bin/cargo +stable build --workspace
```

Para captura isolada, depois de ajustar o caso diff:

```sh
/home/buture/.cargo/bin/cargo +stable build -p buture-app --example capture_shell
env WAYLAND_DISPLAY= timeout 25s target/debug/examples/capture_shell /tmp/buture-prompt45-preview diff
```

A sessão anterior conseguiu capturas reais via X11. Não instalar ferramentas nem alterar configuração do SO automaticamente. Uma janela aberta pelo proprietário pode conter trabalho não salvo; não encerrá-la para QA.

## Limites e pontos para revisão posterior

Rust-analyzer real não ficou disponível neste ambiente: lifecycle e requests LSP foram testados com servidor local de fixture. Não afirmar validação completa de IntelliSense com rust-analyzer. Rustfmt real foi validado; outros formatters e Windows/macOS não foram executados.

Pontos identificados para revisar, sem apresentá-los como corrigidos: desarmar o guard de kill do PTY após saída normal; fortalecer teste PTY cujo marcador pode aparecer no echo do comando; conferir RAII do processo de tasks em erro de wait; validar capabilities no initialize LSP; conferir caret quando completion de um arquivo é aplicado após trocar tab; revisar herança de variáveis GIT_DIR/GIT_INDEX_FILE nas operações nativas.

Depois do 45, a sequência previamente autorizada vai até 50: Git Graph, Commit Assistant, Debugger Core, Debugger UI, Memory Inspector. Retomar essa sequência somente quando Pedro pedir continuação. Não antecipar 51–95.

## Autoria

Preservar a identidade Git configurada por Pedro; não inventar nome/e-mail. Não adicionar Co-authored-by, Generated-by, créditos de IA ou licença MIT ao projeto. Usar commits pequenos e compiláveis, sem force push.
