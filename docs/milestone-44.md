# Marco 44 — avanço e validação

Código implementado até o prompt 44. Pausa solicitada por Pedro para retomar no 45. O esboço de diff foi preservado em patch, sem integrá-lo como funcionalidade concluída; veja [a retomada](../RETOMADA-PROMPT-45.md).

| Prompt | Entrega | Commit |
| --- | --- | --- |
| 36 | Terminal PTY, ANSI, tabs/split e entrada/saída reais | 552b4d1 |
| 37 | Task runner, planos revisados, dependências e output limitado | 3c6c02b |
| 38 | Smart Run, candidatos e preferência de seleção | dbd613a |
| 39 | Transporte stdio e lifecycle de cliente LSP | 4d2b2ee |
| 40 | Recursos de linguagem e revisão de edições | d315539 |
| 41 | Registry de formatters e format on save opt-in | 84cf7e4 |
| 42 | Problems com filtros, contagens reais e navegação | aa73d4f |
| 43 | Git nativo, planos e commits com identidade configurada | 9bcfcc7 |
| 44 | Source Control, Changes/Staged, branches e revisão de commit | a4095a2 |

## Evidências e limites

Fmt, clippy com warnings como erros, testes do workspace e build passaram antes de cada commit. A base 44 foi novamente verificada ao preparar a retomada. Capturas de janela nativa Linux foram inspecionadas para os painéis adicionados. Arquivos temporários dessa sessão estão em `/tmp/buture-shell-50`; são evidência local, não artefatos permanentes nem um marco 50 concluído.

PTY e Rustfmt foram exercitados com processos reais locais. Git foi testado em repositórios temporários com a identidade já configurada pelo proprietário; QA da UI no repo real fez leituras, sem operações remotas ou mutações. Nenhuma ferramenta foi instalada automaticamente.

Rust-analyzer não ficou disponível por configuração de shims do host. LSP foi validado com um servidor local de fixture e testes de parsing, lifecycle, correlação e revisão de alterações. Não houve validação end-to-end com rust-analyzer real. Outros formatters, Windows e macOS não foram executados. Não há medições novas de startup ou consumo em idle.

Cancelamento de tasks atua no processo direto; não garante encerramento de descendentes destacados. Terminal e ferramentas confiadas não são sandbox do SO. O fluxo de commit Git usa commit-tree e atualização condicional da ref, preservando o tree revisado, index/worktree e identidade; não executa hooks de commit. Veja os limites completos em [terminal](terminal.md), [tasks](tasks.md), [linguagens](language.md), [formatação](formatting.md), [Problems](problems.md) e [Git](git.md).

O prompt 45 permanece parcial, sem suíte completa ou inspeção nativa concluída. Git Graph, Commit Assistant, debugger e Memory Inspector (46–50) permanecem pendentes.
