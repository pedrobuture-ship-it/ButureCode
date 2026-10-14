# Tasks — prompt 37

Run & Debug apresenta tasks detectadas em worker a partir dos manifests: Cargo, npm/pnpm scripts, Make/CMake, Maven, Gradle e Go. Entradas têm command, args, env, cwd e dependencies. Configure precede CMake Build; outros comandos são independentes. Targets/scripts podem falhar: sua presença não é sucesso observado.

Clique em uma task prepara o plano, valida confinamento ao root, resolve dependências e rejeita ciclos/ausências. Execute somente após marcar confiança explícita e revisar o comando e cwd. Argumentos separados, sem shell implícito. Scripts/build scripts continuam sendo código do projeto, não são sandboxados. Nenhuma task é iniciada por abrir a pasta.

Execução em worker com stdout/stderr limitados por filas e histórico da UI; omissões são informadas. stdin fechado, cancellation termina o filho direto; processos deliberadamente desacoplados não têm garantia de término. rustup/mise auto-install desabilitado no subprocesso. `.env` não é carregado e valores de env não entram na revisão/logs.

Detecção é limitada a 4096 tasks e 256 dependências por plano. UI virtualiza a lista. Comandos com wrapper `.cmd/.bat` no Windows podem não executar pela API direta; Windows/macOS não foram testados. Sem parser de tasks definido pelo workspace neste ciclo.

## Smart Run — prompt 38

RUN oferece alvos inferidos: bins Cargo (incluindo `src/bin` e `[[bin]]` nomeados), scripts Node start/dev/serve/preview, entrypoints Python convencionais, Go package, Gradle application, Maven exec plugin e binários C/C++ encontrados em build/root. Inferência é heurística; um candidato não garante execução correta. Projeto sem candidato mostra a ausência, sem inventar um programa.

Um alvo único prepara a revisão. Alvos múltiplos pedem seleção; a escolha persiste no perfil pessoal `run-preferences.json` (64 KiB). Referência removida é ignorada. Arquivo inválido ou modificado externamente é preservado; flush no encerramento aguarda até dois segundos. A escolha não salva confiança e nunca pula o consentimento da execução. Nenhuma task do workspace foi executada durante a validação; somente fixtures de teste explícitas.
