# Language services — prompt 39

Settings → Languages → Language Servers ou Command Palette → Language Servers abre a configuração local. Detect rust-analyzer apenas encontra o executable no PATH; versão e inicialização não são fabricadas. Start exige confiança explícita no workspace e no executable absoluto. Nenhuma instalação automática.

Cliente independente de linguagem implementa initialize/initialized, didOpen/didChange/didSave/didClose, shutdown/exit, requests correlacionadas, notificações, limites e timeouts. Rope snapshots são serializados em worker; versões LSP próprias acompanham revision/content id. Sync usa alterações completas, posições UTF-16, até 64 documentos e 2 MiB por documento. Save vem do evento real de IO, não do estado visual de dirty.

Transporte `buture-protocol` compartilha Content-Length limitado a 16 MiB, stdin/stdout por threads e filas limitadas, sem shell/network. Programa e cwd devem ser absolutos. Timeouts de 45 s para initialize, 15 s para features, 3 s para shutdown antes de término forçado. Erros/exits são informados; stderr é drenado sem persistir logs privados.

Rust-analyzer começa com build scripts/proc-macros/check on save desabilitados, `cargo.noDeps`, `--offline` e CARGO_NET_OFFLINE. Isso reduz execução e uso de rede, não oferece sandbox contra executable/configuração local maliciosa. Workspace/applyEdit recebido do servidor é recusado sem revisão interativa; comandos de servidor não são disparados automaticamente.

Teste com servidor mock local verifica todo o lifecycle, framing, Unicode UTF-16 e uma requisição correlacionada. Isso não equivale a validar rust-analyzer em todas as versões/plataformas. Windows/macOS não foram executados. Fonte primária de configuração: https://rust-analyzer.github.io/book/configuration.html . Nenhum contexto enviado a um serviço remoto.

Neste host, a consulta explícita de versão falhou: mise não tem versão definida para o shim; o fallback rustup também encontrou recursão do shim. Nenhuma configuração/instalação foi alterada. A validação de lifecycle usou o servidor mock local, e a UI de configuração foi capturada em janela nativa X11. Não afirmar validação de uma sessão real rust-analyzer neste host.

## Intelligence — prompt 40

Command Palette oferece completion, hover, definition, references, rename, signature help, document/workspace symbols e code actions. Atalhos: Ctrl/Cmd+Space, F12, Shift+F12, F2, Ctrl/Cmd+Shift+Space, Ctrl/Cmd+Period. Disponibilidade vem das capabilities do servidor; não há resultados sintéticos quando ele está ausente. Respostas são analisadas no worker, com listas/documentação limitadas.

Completion aplica TextEdits como uma transação de undo e rejeita snapshots antigos. Snippets não são negociados. Rename e text edits de code actions exigem preview/Apply; modificam buffers, sem gravar disco automaticamente. Validam todos os buffers antes de aplicar, rejeitam versões antigas e caminhos fora do root. Resource create/delete/rename e execução de comandos fornecidos pelo servidor permanecem recusados. Save segue a proteção de conflitos existente.

Diagnostics publicados aparecem no gutter e no painel Problems; marcador é invalidado após mudança de revision/content id. A UI mantém até 10 mil diagnostics / 512 arquivos. Diagnostics com versão incompatível são ignorados; servidor que omite a versão não fornece garantia de frescor. Localizações são UTF-16 e navegação abre o arquivo/posição informada, sem afirmar validade depois de mudanças externas.

Testes verificam resposta de completion no mock local, parsing/UTF-16, preparação que mantém disco intacto e undo, e atomicidade da aplicação quando um buffer muda durante a revisão. A sessão rust-analyzer real continua indisponível neste host; a captura nativa registra a UI nesse estado, sem resultados fabricados. Mouse/commands acionam os resultados; navegação completa de popup por teclado e snippets requerem refinamento posterior.
