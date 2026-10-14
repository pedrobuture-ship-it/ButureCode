# Problems — prompt 42

Painel Problems mostra diagnostics realmente recebidos: arquivo, linha/coluna UTF-16, mensagem e source. Errors, Warnings e Info/hints têm contadores e filtros; busca considera arquivo/mensagem/source. Click abre arquivo/posição informada. Gutter usa o mesmo diagnóstico e invalida o marcador quando o buffer muda.

Dados compartilhados por Arc; filtering em worker único, coalescing e cancelamento por generation. Lista virtualizada, mensagens longas resumidas no row com tooltip. Sem servidor/diagnostics recebidos, o painel mostra ausência de dados, não “build passou”. Contadores durante filtering refletem o último lote concluído.

Limites de cache e freshness estão em [language](language.md). Não há extração heurística de errors de output de tasks: stdout arbitrário não é convertido em diagnostics fictícios. Teste verifica filtros/severidades/sources/posição e cancelamento. A captura nativa deste host mostra estado sem diagnostics porque rust-analyzer não está disponível.
