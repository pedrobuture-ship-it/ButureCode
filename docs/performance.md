# Performance observada — marco 35

Benchmark isolado de buffer, Linux x86_64, Rust stable 1.99.0, perfil Cargo bench otimizado. Executado em 13/10/2026 com `cargo bench -p buture-editor --bench buffer`. Texto sintético `let value = 42;\n`; cada rodada insere/remove um caractere no meio e consulta posição. Uma execução, sem inferir p95 ou regressão a partir de um único resultado.

| Linhas | Construção do buffer | 1000 rodadas insert/remove/map |
| --- | --- | --- |
| 10 mil | 424,402 µs | 1,782184 ms |
| 100 mil | 3,802869 ms | 2,122535 ms |
| 1 milhão | 42,725151 ms | 2,588283 ms |

Isso mede Rope/buffer, sem disco, fonte, UI, parsing, snapshots de todos os cenários ou consumo de memória. Não prova startup rápido, fluidez de linha gigantesca ou consumo de idle. O teste de UI de 100 mil linhas verifica que a produção de shapes fica restrita ao viewport; não mede FPS.

Renderização virtualiza linhas. Parsing, busca, manifests, indexação e IO ocorrem fora do loop de apresentação. Índice de caminhos é sob demanda; consultas obsoletas são canceladas e resultados de parsing filtrados por identidade/revision. Backdrop/effects são compartilhados. Limites de entrada e resultados estão descritos em [editor](editor.md).

Próximas medições devem separar startup, latência input-to-frame, long lines, memória/undo, parse inicial/incremental e idle em Linux/Windows/macOS. Nenhuma dessas métricas foi fabricada neste marco.
