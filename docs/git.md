# Git — prompt 43

Domínio independente da UI usa o executable Git do ambiente: repository detection, status porcelain v1 -z, branch, stage, unstage, commit, checkout/create branch, fetch/pull/push. Mutações só são executadas por Plan confirmado. Nenhuma rede automática; não há force push/reset hard/clean.

Leituras sem pager, fsmonitor, external diff ou textconv. Output/patch até 16 MiB, status até 10 mil entradas, subprocessos em workers com timeout 60 s. Nomes de arquivos são NUL-delimited e preservam bytes no Unix. Paths de operações são relativos ao repo, sem fuga ou `.git`; paths sensíveis comuns bloqueados para stage/commit. Essa heurística não é scanner completo de secrets: revisar staged continua obrigatório.

Stage/unstage preservam worktree. Unstage em repo unborn remove somente index entries. Pull é ff-only. Troca de branch usa switch sem force; Git recusa overwrite de alterações. Operações de rede/configuração podem usar ferramentas/credential helpers do ambiente e exigem ação/confiança explícita da UI; URLs com userinfo são redigidas em mensagens, sem persistir logs.

Commit preserva nome/e-mail configurados, não adiciona trailers e confere staged diff/tree/HEAD. Cria o objeto do tree revisado e atualiza a ref por compare-and-swap, sem substituir index/worktree concorrentes. Hooks de commit não são executados por esse fluxo; assinatura segue commit.gpgsign. Uma ref concorrente pode impedir a atualização e deixar um objeto órfão normal do Git; não prometer isolamento de todo filesystem nem rollback remoto.

Testes criam repo temporário com a identidade já configurada pelo proprietário, verificam Unicode/spaces, stage/unstage/unborn, commit e recusa após staged mudar. Nenhuma identidade de teste é inventada; nenhum remote é acessado nos testes. Git real Linux validado; Windows/macOS não executados. Requer CLI com restore/switch/pathspec-from-file (Git 2.25+ recomendado).

## Source Control — prompt 44

Activity Source Control ou Palette → Git Commit abre Changes/Staged Changes, commit box e branch indicator. Status em worker, refresh periódico apenas quando visível. Stage/unstage/create/checkout/fetch/pull/push vêm de botões; sem confiança no repo ficam desabilitados. Execuções que ainda estão sendo preparadas são canceladas se root/trust mudar.

Commit abre revisão de mensagem/tree com consentimento explícito. Preview geral limitado a 8 KiB, informado; não afirmar revisão de diff que não foi feita. Editor dirty bloqueia stage daquele arquivo e checkout/pull; não há autosave ou descarte implícito. Repo/trust são resetados ao mudar workspace, e resultados de scans anteriores são descartados. Fechar espera a operação Git ativa para não ocultar efeitos remotos incertos.

Testes UI cobrem trust revogada antes da execução e listas filtradas com zero staged. Native Linux inspecionado; não foi executada mutação/rede no repo real durante QA, somente leituras e fixtures temporárias.
