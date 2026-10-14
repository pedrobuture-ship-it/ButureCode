# Formatting — prompt 41

Settings → Editor → Formatting permite Auto by language ou rustfmt, clang-format, Ruff, Black, Prettier e gofmt, com override de executable absoluto. Command Palette e Ctrl/Cmd+Shift+I / Ctrl/Cmd+Alt+Shift+I acionam document/selection. Seleção deve formar um fragmento completo aceito pelo formatter; snippets incompletos falham sem substituir o buffer.

Antes da primeira execução, um plano mostra executable, argumentos, source/range e cwd. Confiança vinculada a root/provider/executable, somente na sessão; workspace novo/configuração de executable alterada exige revisão. Prettier pode carregar plugins do projeto. Não é sandbox contra ferramenta/configuração maliciosa. Nenhum tool instalado automaticamente.

stdin/stdout em workers, 8 MiB input/16 MiB output, timeout 20 s. Não usar flags write-in-place. UTF-8 validado, CRLF preservado, selection não pode dividir CRLF. Rustfmt usa edition detectada em Cargo e skip_children para não formatar módulos externos ao snapshot. Empty output para source não vazio é recusado.

Resultado manual aplica uma transação de undo se revision/content id ainda corresponder. Format on Save é preferência User, que Workspace não habilita. Exige trust existente para executar; sem trust/ferramenta, com erro ou resultado obsoleto, salva o snapshot original solicitado. Edições posteriores continuam no buffer e ficam dirty; mudanças externas no disco continuam sujeitas à proteção de conflito.

Testes Linux executam rustfmt real e verificam CRLF, retorno stdout e arquivo/módulo em disco intactos. Fallback de save conserva alterações feitas depois da solicitação; workspace não pode habilitar a preferência. Outros executables/plataformas dependem do ambiente e não foram validados end-to-end. Wrappers Windows `.cmd/.bat` não são avaliados via shell.

Fontes primárias: https://github.com/rust-lang/rustfmt , https://clang.llvm.org/docs/ClangFormat.html , https://docs.astral.sh/ruff/formatter/ , https://black.readthedocs.io/en/stable/usage_and_configuration/the_basics.html , https://prettier.io/docs/cli , https://pkg.go.dev/cmd/gofmt . Executables externos mantêm seus próprios termos; não são redistribuídos neste repo.
