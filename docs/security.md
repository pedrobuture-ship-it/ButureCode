# Segurança

Nenhuma ferramenta é instalada automaticamente. Install exige revisão da explicação, método e comando exato, marcação de consentimento e ação Execute na aplicação. A execução usa argumentos separados, sem shell. Ferramentas/pacotes são uma lista fechada; nenhum comando vem de arquivos do workspace. Linux mostra e explica `sudo -n`; não coleta senhas, não eleva silenciosamente, não atualiza o sistema e não usa AUR automaticamente. [Fluxo e limitações](toolchains.md).

Sondagens usam um diretório temporário próprio, fora do workspace, e desabilitam auto-instalação de rustup e mise no subprocesso. Outros executáveis do ambiente continuam sendo código externo confiado pelo usuário; isso não é sandbox contra um executável malicioso.

A varredura consulta executáveis de PATH absoluto e diretórios conhecidos do ambiente do usuário com argumentos fixos. Shims são ferramentas do ambiente: falhas e timeout são informados; arquivos `.cmd`/`.bat` do Windows não são avaliados via shell. A aplicação não executa comandos definidos em projetos.

Explorer exige confirmação para exclusão permanente, rejeita operações fora do root, caminhos relativos de fuga, sobrescrita de destinos existentes e traversal de links simbólicos. Copy pode deixar destino parcial se IO falhar. A validação de caminhos usa APIs portáveis e não garante isolamento contra alterações concorrentes maliciosas do filesystem entre verificação e operação; não tratar um workspace hostil como sandbox do SO.

Preferências são validadas e não aceitam secrets; o workspace não habilita IA, telemetria ou conclusão do onboarding. Arquivos pessoais inválidos não são substituídos por defaults. `.env` não é versionado. O preview permite ler arquivos escolhidos pelo usuário; nenhuma informação é enviada a serviços externos. Logs de instalação ficam em memória e são limitados; não há armazenamento de credenciais ou telemetria.

Rede só pode ser usada pelo gerenciador de pacotes após confirmação de instalação. Não há IA, extensões, SSH ou banco de dados neste ciclo. `unsafe_code` é proibido no código autoral; dependências mantêm suas próprias licenças.

Edição compara conteúdo salvo/disco antes de substituir arquivos; reloads em andamento preservam alterações locais. Fechar com alterações exige decisão explícita. Substituição no workspace tem revisão e pode terminar parcialmente em erro; veja [editor](editor.md). Sessão não grava conteúdo não salvo. Atalhos são pessoais, validados, e não aceitam execução de comandos arbitrários. Busca/índice não seguem symlinks e excluem caminhos sensíveis por padrão, mas abertura explícita pode ler arquivos escolhidos pelo usuário.
