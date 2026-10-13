# Toolchains e instalação assistida — prompts 11–20

`buture-toolchains` detecta host (Windows/Linux/macOS e x86_64/aarch64), shell informado pelo SO, diretórios absolutos de PATH, home e locais conhecidos de SDKs. Não modifica o ambiente nem ativa ambientes Python.

`ToolchainProvider` mantém providers independentes para Rust, C/C++, Python, Java, Node.js e Go. Cada ferramenta tem função, caminho e estado observado: ausente, versão obtida ou falha de sondagem. Uma falha de shim, timeout ou comando de versão não suportado não equivale a ferramenta ausente. Instaladores só são oferecidos para ferramentas ausentes e mapeamentos conhecidos.

As sondagens executam apenas argumentos fixos de versão em executáveis do ambiente do usuário e diretórios conhecidos; sem shell, stdin ou comandos de projetos. São iniciadas pela ação Scan no onboarding ou Settings → Development → Toolchains. Providers rodam em threads independentes. Cada subprocesso de versão tem timeout de três segundos e saída limitada; não há estimativa inventada de progresso. Rust inclui diretórios de toolchains rustup; Python inclui pyenv, Conda e ambientes indicados pelo host; Java inclui JAVA_HOME, SDKMAN e locais de JDK; Node inclui nvm, Volta, mise e caminhos informados pelo host. Não é enumeração exaustiva de todas as instalações do disco.

O onboarding apresenta cinco etapas, seleção de linguagens e resumo de resultados reais. As opções sem provider neste ciclo são informadas como não suportadas pela varredura. Conclusão e linguagens são salvas nas preferências do usuário.

## Instalação assistida

Fluxo: ferramenta ausente → explicação → método e comando exato → autorização explícita → execução no worker → nova sondagem e rescan. O plano é uma estrutura imutável; a API de execução aceita apenas um plano confirmado. Nenhum comando vem de settings ou arquivos de workspace. Nomes de ferramentas e pacotes pertencem a uma lista fechada. Não há interpolação em shell.

- Arch/Omarchy: pacman, sem AUR ou atualização automática de repositórios/sistema.
- Debian/Ubuntu: apt-get para mapeamentos suportados.
- Fedora: dnf para mapeamentos suportados.
- macOS: Homebrew já instalado em um dos prefixos conhecidos.
- Windows: winget disponível, identificadores exatos e fonte winget; contratos não são aceitos automaticamente.
- Outros ambientes: nenhuma ação Install automática.

Linux mostra `sudo -n` no comando e explica o requisito administrativo. Não solicita nem guarda senhas; sem autorização administrativa disponível, a execução falha e o usuário pode copiar o comando para um terminal externo. Não executar refresh/upgrade silencioso para corrigir falhas de pacotes. Em Arch, manter o sistema atualizado separadamente conforme o procedimento da distribuição. Install pode trazer dependências; Repair e Update não são oferecidos porque não há implementação própria segura neste marco.

A saída fica em memória com buffer limitado. A instalação não é interrompida ao fechar a janela da aplicação: operações de um gerenciador de pacotes não devem ser tratadas como subprocessos descartáveis de versão. Não há botão Cancel depois de iniciar mudanças no sistema. Êxito do gerenciador sem ferramenta visível ou sem versão verificável é informado como verificação incompleta. O PATH do processo não é alterado; em alguns sistemas é necessário reiniciar e refazer a varredura.

## Limitações e fontes

Validado em Linux x86_64. Windows/macOS não foram compilados ou executados neste ambiente. Scripts Windows `.cmd`/`.bat` são detectados, mas não executados por shell para obter versões. Algumas ferramentas não possuem comando universal de versão (gofmt, MSVC); o caminho e a falha permanecem visíveis. Mapeamentos de pacotes dependem da versão da distribuição e podem falhar honestamente; não há promessa de instalação universal ou resolução automática de conflitos.

Documentação consultada: [winget install](https://learn.microsoft.com/en-us/windows/package-manager/winget/install), [Homebrew](https://docs.brew.sh/Manpage), [Rust no Arch](https://wiki.archlinux.org/title/rust), [rust-analyzer](https://archlinux.org/packages/extra/x86_64/rust-analyzer/), [JDK](https://archlinux.org/packages/extra/x86_64/jdk-openjdk/) e [Node](https://archlinux.org/packages/extra/x86_64/nodejs/). Nenhum instalador real foi executado na validação desta etapa.

Sondagens também desabilitam auto-instalação por shims usando `RUSTUP_AUTO_INSTALL=0` e `MISE_AUTO_INSTALL=false`, em diretório temporário próprio: [rustup](https://rust-lang.github.io/rustup/environment-variables.html) e [mise](https://mise.jdx.dev/configuration/settings.html).
