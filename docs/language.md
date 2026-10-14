# Language services — prompt 39

Settings → Languages → Language Servers ou Command Palette → Language Servers abre a configuração local. Detect rust-analyzer apenas encontra o executable no PATH; versão e inicialização não são fabricadas. Start exige confiança explícita no workspace e no executable absoluto. Nenhuma instalação automática.

Cliente independente de linguagem implementa initialize/initialized, didOpen/didChange/didSave/didClose, shutdown/exit, requests correlacionadas, notificações, limites e timeouts. Rope snapshots são serializados em worker; versões LSP próprias acompanham revision/content id. Sync usa alterações completas, posições UTF-16, até 64 documentos e 2 MiB por documento. Save vem do evento real de IO, não do estado visual de dirty.

Transporte `buture-protocol` compartilha Content-Length limitado a 16 MiB, stdin/stdout por threads e filas limitadas, sem shell/network. Programa e cwd devem ser absolutos. Timeouts de 45 s para initialize, 15 s para features, 3 s para shutdown antes de término forçado. Erros/exits são informados; stderr é drenado sem persistir logs privados.

Rust-analyzer começa com build scripts/proc-macros/check on save desabilitados, `cargo.noDeps`, `--offline` e CARGO_NET_OFFLINE. Isso reduz execução e uso de rede, não oferece sandbox contra executable/configuração local maliciosa. Workspace/applyEdit recebido do servidor é recusado sem revisão interativa; comandos de servidor não são disparados automaticamente.

Teste com servidor mock local verifica todo o lifecycle, framing, Unicode UTF-16 e uma requisição correlacionada. Isso não equivale a validar rust-analyzer em todas as versões/plataformas. Windows/macOS não foram executados. Fonte primária de configuração: https://rust-analyzer.github.io/book/configuration.html . Nenhum contexto enviado a um serviço remoto.

Neste host, a consulta explícita de versão falhou: mise não tem versão definida para o shim; o fallback rustup também encontrou recursão do shim. Nenhuma configuração/instalação foi alterada. A validação de lifecycle usou o servidor mock local, e a UI de configuração foi capturada em janela nativa X11. Não afirmar validação de uma sessão real rust-analyzer neste host.
